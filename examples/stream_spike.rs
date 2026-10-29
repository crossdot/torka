//! Phase 0.5 spike: prove the ADR-002 streaming path end-to-end.
//!
//! Flow: create+seed a local test torrent in-process (librqbit) -> expose a
//! seekable loopback HTTP-range endpoint backed by `ManagedTorrent::stream` ->
//! `ffprobe` + `ffmpeg` read it over `http://` and remux to HLS -> verify a
//! playable segment appears, then prove `-ss` seek works.
//!
//! Run: `cargo run --example stream_spike` (expects /tmp/spike/test.mp4).

use std::{
    io::SeekFrom,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use librqbit::{CreateTorrentOptions, Session};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

#[derive(Clone)]
struct AppState {
    handle: Arc<librqbit::ManagedTorrent>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let src = "/tmp/spike/test.mp4";
    let hls_dir = "/tmp/spike/hls";
    let seek_dir = "/tmp/spike/seek";

    // 1. Session + create/serve a torrent of the local test file.
    let session = Session::new("/tmp/spike/session".into()).await?;
    let (ctor, handle) = session
        .create_and_serve_torrent(Path::new(src), CreateTorrentOptions::default())
        .await?;
    let torrent_bytes = ctor.as_bytes()?;
    println!(
        "torrent: {} bytes, info_hash={:?}",
        torrent_bytes.len(),
        ctor.info_hash()
    );

    handle.wait_until_initialized().await?;
    let file_len = handle.with_metadata(|m| m.file_infos[0].len)?;
    println!("selected file 0: len={file_len}");

    // 2. Seekable loopback HTTP-range server backed by FileStream.
    let app = Router::new()
        .route("/file", get(file_handler))
        .with_state(AppState { handle });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18081").await?;
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 3. Probe over http:// (proves seekable http + range reads).
    let probe = tokio::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1",
            "http://127.0.0.1:18081/file",
        ])
        .output()
        .await?;
    println!(
        "ffprobe duration: {}s",
        String::from_utf8_lossy(&probe.stdout).trim()
    );

    // 4. Remux to HLS over http:// (stream copy, no transcode).
    let _ = std::fs::remove_dir_all(hls_dir);
    std::fs::create_dir_all(hls_dir)?;
    let start = Instant::now();
    let mut child = tokio::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-i",
            "http://127.0.0.1:18081/file",
            "-c",
            "copy",
            "-f",
            "hls",
            "-hls_time",
            "2",
            "-hls_list_size",
            "0",
            "-hls_segment_filename",
            &format!("{hls_dir}/seg_%03d.ts"),
            &format!("{hls_dir}/out.m3u8"),
        ])
        .spawn()?;

    let first_seg = wait_for_segment(hls_dir, Duration::from_secs(30)).await?;
    println!(
        "time-to-first-segment: {:?} (first: {first_seg})",
        start.elapsed()
    );

    let status = child.wait().await?;
    println!("ffmpeg exit: {status}");
    let segs = list_ts(hls_dir)?;
    println!("full remux produced {} segments", segs.len());

    // 5. Seek probe: -ss 5 before -i must seek the http source to t=5.
    let _ = std::fs::remove_dir_all(seek_dir);
    std::fs::create_dir_all(seek_dir)?;
    let seek_status = tokio::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-ss",
            "5",
            "-i",
            "http://127.0.0.1:18081/file",
            "-c",
            "copy",
            "-f",
            "hls",
            "-hls_time",
            "2",
            "-hls_list_size",
            "0",
            "-hls_segment_filename",
            &format!("{seek_dir}/seg_%03d.ts"),
            &format!("{seek_dir}/out.m3u8"),
        ])
        .status()
        .await?;
    let seek_segs = list_ts(seek_dir)?;
    println!(
        "seek(-ss 5) exit: {seek_status}, produced {} segments (vs {})",
        seek_segs.len(),
        segs.len()
    );

    server.abort();
    session.stop().await;
    println!("SPIKE OK");
    Ok(())
}

async fn file_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let mut stream = state
        .handle
        .clone()
        .stream(0)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let len = stream.len();

    let range = headers
        .get(axum::http::header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("bytes="))
        .and_then(|v| v.split_once('-'))
        .and_then(|(s, e)| {
            let s = s.parse::<u64>().ok()?;
            let e = if e.is_empty() {
                None
            } else {
                Some(e.parse::<u64>().ok()?.saturating_add(1))
            };
            Some((s, e))
        });

    let (status, start, end) = match range {
        Some((s, e)) => {
            let end = e.unwrap_or(len);
            if s >= len || end > len || end <= s {
                return Err((StatusCode::RANGE_NOT_SATISFIABLE, "bad range".into()));
            }
            (StatusCode::PARTIAL_CONTENT, s, end)
        }
        None => (StatusCode::OK, 0, len),
    };

    stream
        .seek(SeekFrom::Start(start))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let body =
        axum::body::Body::from_stream(ReaderStream::with_capacity(stream.take(end - start), 65536));

    let mut resp = HeaderMap::new();
    resp.insert("accept-ranges", HeaderValue::from_static("bytes"));
    resp.insert(
        "content-length",
        HeaderValue::from_str(&(end - start).to_string()).unwrap(),
    );
    if status == StatusCode::PARTIAL_CONTENT {
        resp.insert(
            "content-range",
            HeaderValue::from_str(&format!("bytes {start}-{}/{}", end - 1, len)).unwrap(),
        );
    }

    Ok((status, resp, body).into_response())
}

async fn wait_for_segment(dir: &str, timeout: Duration) -> anyhow::Result<String> {
    let start = Instant::now();
    loop {
        if let Some(name) = list_ts(dir)?.into_iter().next() {
            return Ok(name);
        }
        if start.elapsed() > timeout {
            anyhow::bail!("timeout waiting for first HLS segment");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn list_ts(dir: &str) -> std::io::Result<Vec<String>> {
    let mut out = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".ts"))
        .collect::<Vec<_>>();
    out.sort();
    Ok(out)
}
