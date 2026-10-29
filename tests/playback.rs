//! Phase 6 integration test: full playback session lifecycle with a local
//! torrent. Requires `ffmpeg` (libx264); run with `cargo test --test playback -- --ignored`.

use std::{path::Path, time::Duration};

use librqbit::{CreateTorrentOptions, Session};
use movie_streamer::{
    config::Config,
    streaming::{SessionState, StreamingManager, TorrentSource},
};

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn full_playback_lifecycle() {
    let root = "/tmp/torka-pb-test";
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();
    let media = format!("{root}/test.mp4");

    // 1. Generate a short test video (keyframes every second).
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=duration=6:size=640x360:rate=25",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=6",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-g",
            "25",
            "-keyint_min",
            "25",
            "-sc_threshold",
            "0",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            &media,
        ])
        .status()
        .await
        .unwrap();
    assert!(status.success());

    // 2. Create a `.torrent` of it (throwaway seeder session).
    let seeder = Session::new(format!("{root}/seed").into()).await.unwrap();
    let (ctor, _seed) = seeder
        .create_and_serve_torrent(Path::new(&media), CreateTorrentOptions::default())
        .await
        .unwrap();
    let bytes = ctor.as_bytes().unwrap().to_vec();

    // 3. Streaming manager with writable dirs.
    let config = Config {
        cache_dir: format!("{root}/dl"),
        session_dir: format!("{root}/sessions"),
        ffmpeg_path: "ffmpeg".into(),
        max_active_sessions: 2,
        ffmpeg_start_timeout_secs: 30,
        ..Config::default()
    };
    let manager = StreamingManager::new(&config).await.unwrap();

    // 4. Create a session (file already on disk via output_folder + overwrite).
    let (id, initial) = manager
        .create(
            TorrentSource::Bytes(bytes),
            Some(0),
            Some(Path::new(root).to_path_buf()),
            None,
        )
        .await
        .unwrap();
    assert_eq!(initial, SessionState::Created);

    // 5. Poll until ready.
    let state = wait_for(
        &manager,
        &id,
        |s| s == SessionState::Ready,
        Duration::from_secs(30),
    )
    .await;
    assert_eq!(state, SessionState::Ready);

    // 6. Playlist + segment resolvable; traversal rejected.
    assert!(manager.playlist_path(&id).is_some());
    assert!(manager.segment_path(&id, "seg_000.ts").is_some());
    assert!(manager.segment_path(&id, "../evil.ts").is_none());

    // 7. Stop -> terminal, session dir cleaned up.
    manager.stop(&id).await;
    let state = wait_for(
        &manager,
        &id,
        SessionState::is_terminal,
        Duration::from_secs(10),
    )
    .await;
    assert!(state.is_terminal());
    assert!(!Path::new(&format!("{root}/sessions/{id}")).exists());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn transcode_incompatible_source() {
    let root = "/tmp/torka-pb-transcode";
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();
    let media = format!("{root}/test.mp4");

    // 1. Generate a short MPEG-4 Part 2 video (not HLS-compatible -> must transcode).
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=duration=4:size=640x360:rate=25",
            "-c:v",
            "mpeg4",
            "-q:v",
            "5",
            "-f",
            "mp4",
            &media,
        ])
        .status()
        .await
        .unwrap();
    assert!(status.success());

    // 2. Create a `.torrent`.
    let seeder = Session::new(format!("{root}/seed").into()).await.unwrap();
    let (ctor, _seed) = seeder
        .create_and_serve_torrent(Path::new(&media), CreateTorrentOptions::default())
        .await
        .unwrap();
    let bytes = ctor.as_bytes().unwrap().to_vec();

    let config = Config {
        cache_dir: format!("{root}/dl"),
        session_dir: format!("{root}/sessions"),
        ffmpeg_path: "ffmpeg".into(),
        max_active_sessions: 2,
        ffmpeg_start_timeout_secs: 60,
        torrent_resolve_timeout_secs: 60,
        ..Config::default()
    };
    let manager = StreamingManager::new(&config).await.unwrap();

    // 3. Auto mode: mpeg4 is not HLS-compatible, so it must transcode to H.264.
    let (id, _) = manager
        .create(
            TorrentSource::Bytes(bytes),
            Some(0),
            Some(Path::new(root).to_path_buf()),
            None,
        )
        .await
        .unwrap();

    let state = wait_for(
        &manager,
        &id,
        |s| s == SessionState::Ready,
        Duration::from_secs(60),
    )
    .await;
    assert_eq!(state, SessionState::Ready);

    // 4. The produced segment must be H.264 (transcoded), not mpeg4.
    let seg = manager.segment_path(&id, "seg_000.ts").unwrap();
    let probe = tokio::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=nw=1:nk=1",
            seg.to_str().unwrap(),
        ])
        .output()
        .await
        .unwrap();
    assert!(probe.status.success());
    let codec = String::from_utf8_lossy(&probe.stdout);
    assert!(
        codec.contains("h264") && !codec.contains("mpeg4"),
        "unexpected segment codec: {codec:?}"
    );

    manager.stop(&id).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn seek_restarts_pipeline() {
    let root = "/tmp/torka-pb-seek";
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();
    let media = format!("{root}/test.mp4");

    // 1. Generate a 12s video (1s keyframes).
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=duration=12:size=640x360:rate=25",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=12",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-g",
            "25",
            "-keyint_min",
            "25",
            "-sc_threshold",
            "0",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            &media,
        ])
        .status()
        .await
        .unwrap();
    assert!(status.success());

    // 2. Torrent + manager + create -> ready.
    let seeder = Session::new(format!("{root}/seed").into()).await.unwrap();
    let (ctor, _seed) = seeder
        .create_and_serve_torrent(Path::new(&media), CreateTorrentOptions::default())
        .await
        .unwrap();
    let bytes = ctor.as_bytes().unwrap().to_vec();

    let config = Config {
        cache_dir: format!("{root}/dl"),
        session_dir: format!("{root}/sessions"),
        ffmpeg_path: "ffmpeg".into(),
        max_active_sessions: 2,
        ffmpeg_start_timeout_secs: 60,
        torrent_resolve_timeout_secs: 60,
        ..Config::default()
    };
    let manager = StreamingManager::new(&config).await.unwrap();

    let (id, _) = manager
        .create(
            TorrentSource::Bytes(bytes),
            Some(0),
            Some(Path::new(root).to_path_buf()),
            None,
        )
        .await
        .unwrap();
    let state = wait_for(
        &manager,
        &id,
        |s| s == SessionState::Ready,
        Duration::from_secs(60),
    )
    .await;
    assert_eq!(state, SessionState::Ready);

    // 3. Record the first segment mtime, then seek to 6s.
    let seg = manager.segment_path(&id, "seg_000.ts").unwrap();
    let before = std::fs::metadata(&seg).unwrap().modified().unwrap();
    manager.seek(&id, 6.0).await;

    // 4. Wait for the new generation to rewrite the first segment.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        if std::fs::metadata(&seg).unwrap().modified().unwrap() != before {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "segment was not rewritten after seek"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // 5. Still ready (seek did not fail).
    let state = manager.status(&id).await.unwrap();
    assert!(matches!(state, SessionState::Ready | SessionState::Playing));

    manager.stop(&id).await;
}

async fn wait_for(
    manager: &StreamingManager,
    id: &str,
    pred: impl Fn(SessionState) -> bool,
    timeout: Duration,
) -> SessionState {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(s) = manager.status(id).await {
            if pred(s) {
                return s;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timeout waiting for session {id}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
