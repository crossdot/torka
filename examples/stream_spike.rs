//! Phase 5 integration smoke: a locally-created test torrent flows through
//! `TorrentEngine` → `SourceServer` → `FfmpegRunner` to produce HLS with seek.
//!
//! Run: `cargo run --example stream_spike` (requires `ffmpeg`/`ffprobe`).

use std::{path::Path, time::Duration};

use librqbit::{CreateTorrentOptions, Session};
use movie_streamer::streaming::{
    FfmpegRunner, HlsConfig, SourceServer, TorrentEngine, TorrentSource,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let media = "/tmp/spike/test.mp4";
    let root = "/tmp/spike";

    // 1. Generate a short test video (keyframes every second).
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root)?;
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=duration=10:size=1280x720:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=10",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-g",
            "30",
            "-keyint_min",
            "30",
            "-sc_threshold",
            "0",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            media,
        ])
        .status()
        .await?;
    anyhow::ensure!(status.success(), "test media generation failed");

    // 2. Create a `.torrent` of it (throwaway seeder session).
    let seeder = Session::new(format!("{root}/seed").into()).await?;
    let (ctor, _seed) = seeder
        .create_and_serve_torrent(Path::new(media), CreateTorrentOptions::default())
        .await
        .map_err(|e| anyhow::anyhow!("create torrent: {e}"))?;
    let torrent_bytes = ctor.as_bytes()?.to_vec();

    // 3. Resolve metadata + enumerate files (file is already on disk).
    let engine = TorrentEngine::new(format!("{root}/dl")).await?;
    let handle = engine
        .add(TorrentSource::Bytes(torrent_bytes), Some(root.into()))
        .await?;
    let files = engine.files(&handle).await?;
    println!("files: {files:?}");

    // 4. Seekable loopback source.
    let source = SourceServer::bind().await?;
    let url = source.register(handle.clone(), 0)?;
    println!("source url: {url}");

    // 5. Remux to HLS over http:// (stream copy).
    let out = format!("{root}/hls");
    std::fs::create_dir_all(&out)?;
    let cfg = HlsConfig {
        ffmpeg_path: "ffmpeg".into(),
        input_url: url.clone(),
        output_dir: out.into(),
        start_time_secs: None,
    };
    let start = std::time::Instant::now();
    let mut ff = FfmpegRunner::start(&cfg)?;
    ff.wait_ready(Duration::from_secs(30)).await?;
    println!("time-to-first-segment: {:?}", start.elapsed());
    ff.stop().await;
    let full_segs = count_segments(&format!("{root}/hls"));

    // 6. Seek probe: `-ss 5` must restart the pipeline at t=5.
    let seek_out = format!("{root}/seek");
    std::fs::create_dir_all(&seek_out)?;
    let mut ff2 = FfmpegRunner::start(&HlsConfig {
        output_dir: seek_out.into(),
        start_time_secs: Some(5.0),
        ..cfg
    })?;
    ff2.wait_ready(Duration::from_secs(30)).await?;
    ff2.stop().await;
    let seek_segs = count_segments(&format!("{root}/seek"));

    println!("full remux: {full_segs} segments; seek(-ss 5): {seek_segs} segments");
    println!("SPIKE OK");
    Ok(())
}

fn count_segments(dir: &str) -> usize {
    std::fs::read_dir(dir)
        .map(|e| {
            e.flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "ts"))
                .count()
        })
        .unwrap_or(0)
}
