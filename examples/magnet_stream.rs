//! Live playback smoke: resolve a real magnet and stream it to HLS.
//! Usage: `MAGNET="magnet:?xt=..." cargo run --example magnet_stream`

use std::time::{Duration, Instant};

use movie_streamer::{
    config::Config,
    streaming::{SessionState, StreamingManager, TorrentSource},
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let magnet = std::env::var("MAGNET").expect("set MAGNET env var");
    let root = "/tmp/torka-magnet-test";
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root)?;

    let config = Config {
        cache_dir: format!("{root}/dl"),
        session_dir: format!("{root}/sessions"),
        ffmpeg_path: "ffmpeg".into(),
        torrent_resolve_timeout_secs: 120,
        ffmpeg_start_timeout_secs: 120,
        ..Config::default()
    };
    let manager = StreamingManager::new(&config).await?;

    let source = TorrentSource::from_magnet(magnet)?;
    let (id, _) = manager.create(source, None, None, None).await?;
    println!("session: {id}");

    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        let s = manager.status(&id).await.expect("session gone");
        println!("status: {}", s.as_str());
        if matches!(s, SessionState::Ready | SessionState::Playing) {
            break;
        }
        if s.is_terminal() {
            anyhow::bail!("session ended: {}", s.as_str());
        }
        if Instant::now() > deadline {
            manager.stop(&id).await;
            anyhow::bail!("timed out waiting for ready");
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    println!("playlist: {:?}", manager.playlist_path(&id));
    println!("segment0: {:?}", manager.segment_path(&id, "seg_000.ts"));

    if let Some(p) = manager.segment_path(&id, "seg_000.ts") {
        println!("segment0 size: {} bytes", std::fs::metadata(&p)?.len());
    }
    if let Some(p) = manager.playlist_path(&id) {
        let head = std::fs::read_to_string(&p)?;
        println!(
            "playlist head: {}",
            head.lines().take(4).collect::<Vec<_>>().join(" | ")
        );
    }

    tokio::time::sleep(Duration::from_secs(3)).await;
    manager.stop(&id).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    println!(
        "final: {}",
        manager
            .status(&id)
            .await
            .map(|s| s.as_str().to_string())
            .unwrap_or_else(|| "gone".into())
    );
    println!("OK");
    Ok(())
}
