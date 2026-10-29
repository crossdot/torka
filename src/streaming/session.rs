//! Playback session lifecycle: state machine + orchestration (resolve torrent →
//! register source → probe codecs → FFmpeg HLS (remux or transcode) → ready →
//! cleanup), with idle-timeout expiry.

use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{Mutex, Notify, watch};
use tokio_util::sync::CancellationToken;

use crate::{config::HwAccelBackend, error::AppError};

use super::{
    ffmpeg::{Encode, FfmpegRunner, HlsConfig},
    plan::{PlaybackMode, select_mode},
    probe::probe,
    source::SourceServer,
    torrent::{TorrentEngine, TorrentFile, TorrentSource},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Created,
    Resolving,
    Buffering,
    Ready,
    Playing,
    Stopping,
    Stopped,
    Failed,
    Expired,
}

impl SessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Resolving => "resolving",
            Self::Buffering => "buffering",
            Self::Ready => "ready",
            Self::Playing => "playing",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Stopped | Self::Failed | Self::Expired)
    }
}

pub struct PlaybackSession {
    pub id: String,
    pub dir: PathBuf,
    state: Arc<Mutex<SessionState>>,
    cancel: CancellationToken,
    last_activity: Arc<Mutex<Instant>>,
    seek_tx: watch::Sender<Option<f64>>,
    seek_rx: watch::Receiver<Option<f64>>,
    error: Arc<Mutex<Option<String>>>,
    /// Total media duration in seconds (from ffprobe), for the player UI.
    duration_secs: Arc<Mutex<Option<f64>>>,
    /// Signalled once the serve loop has restarted the pipeline after a seek
    /// (first segment ready), so `manager::seek` can be synchronous.
    pub(crate) seek_done: Arc<Notify>,
}

impl PlaybackSession {
    pub fn new(id: String, dir: PathBuf) -> Self {
        let (seek_tx, seek_rx) = watch::channel(None);
        Self {
            id,
            dir,
            state: Arc::new(Mutex::new(SessionState::Created)),
            cancel: CancellationToken::new(),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            seek_tx,
            seek_rx,
            error: Arc::new(Mutex::new(None)),
            duration_secs: Arc::new(Mutex::new(None)),
            seek_done: Arc::new(Notify::new()),
        }
    }

    pub async fn state(&self) -> SessionState {
        *self.state.lock().await
    }

    async fn set_state(&self, s: SessionState) {
        *self.state.lock().await = s;
    }

    /// Transition `ready -> playing` and mark activity (a client started
    /// consuming the playlist).
    pub async fn mark_playing(&self) {
        self.touch().await;
        let mut state = self.state.lock().await;
        if *state == SessionState::Ready {
            *state = SessionState::Playing;
        }
    }

    /// Mark client activity (playlist/segment requests reset the idle timer).
    pub async fn touch(&self) {
        *self.last_activity.lock().await = Instant::now();
    }

    async fn idle_elapsed(&self) -> Duration {
        self.last_activity.lock().await.elapsed()
    }

    /// Request stop; the background task performs cleanup and reaches a terminal
    /// state. Idempotent.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Request a seek to `position_secs`: the serve loop restarts FFmpeg with
    /// `-ss` at that position (a new generation).
    pub fn seek(&self, position_secs: f64) {
        let _ = self.seek_tx.send(Some(position_secs));
    }

    /// Failure reason, if the session ended in `failed`.
    pub async fn error(&self) -> Option<String> {
        self.error.lock().await.clone()
    }

    async fn set_error(&self, msg: String) {
        *self.error.lock().await = Some(msg);
    }

    /// Record the total media duration (ffprobe).
    pub async fn set_duration(&self, secs: Option<f64>) {
        *self.duration_secs.lock().await = secs;
    }

    pub async fn duration(&self) -> Option<f64> {
        *self.duration_secs.lock().await
    }
}

/// Shared services a session task uses.
pub struct SessionEnv {
    pub engine: Arc<TorrentEngine>,
    pub source: Arc<SourceServer>,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub ready_timeout: Duration,
    pub resolve_timeout: Duration,
    /// Idle timeout after `ready`; `None` disables expiry.
    pub idle_timeout: Option<Duration>,
    pub hwaccel_backend: HwAccelBackend,
    pub hwaccel_device: Option<String>,
}

/// How a session ended (drives the terminal state).
enum SessionEnd {
    Stopped,
    Expired,
}

/// Runs the session to completion (ready then stop/expiry), or to failure/cancel.
/// Always performs cleanup (unregister source, remove session dir) and lands in
/// a terminal state.
pub async fn run_session(
    session: Arc<PlaybackSession>,
    source: TorrentSource,
    file_id: Option<usize>,
    output_folder: Option<PathBuf>,
    preferred_mode: Option<String>,
    env: SessionEnv,
) {
    let mut token: Option<String> = None;

    let result = orchestrate(
        &session,
        source,
        file_id,
        output_folder,
        preferred_mode.as_deref(),
        &env,
        &mut token,
    )
    .await;

    if let Some(t) = token {
        env.source.unregister(&t);
    }
    let _ = tokio::fs::remove_dir_all(&session.dir).await;

    let final_state = match result {
        Ok(SessionEnd::Stopped) => SessionState::Stopped,
        Ok(SessionEnd::Expired) => SessionState::Expired,
        Err(e) => {
            tracing::warn!(session = %session.id, error = %e, "playback failed");
            session.set_error(e.to_string()).await;
            SessionState::Failed
        }
    };
    session.set_state(final_state).await;
}

async fn orchestrate(
    session: &Arc<PlaybackSession>,
    source: TorrentSource,
    file_id: Option<usize>,
    output_folder: Option<PathBuf>,
    preferred_mode: Option<&str>,
    env: &SessionEnv,
    token: &mut Option<String>,
) -> Result<SessionEnd, AppError> {
    // Resolve torrent metadata (bounded by the resolve timeout).
    session.set_state(SessionState::Resolving).await;
    let handle = tokio::select! {
        r = tokio::time::timeout(env.resolve_timeout, env.engine.add(source, output_folder)) => {
            match r {
                Ok(Ok(handle)) => handle,
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(AppError::internal("torrent metadata resolve timed out")),
            }
        }
        _ = session.cancel.cancelled() => return Ok(SessionEnd::Stopped),
    };

    let files = env.engine.files(&handle).await?;
    let file_id = select_file(file_id, &files)?;

    let url = env.source.register(handle.clone(), file_id)?;
    *token = Some(url.rsplit('/').next().unwrap_or_default().to_string());

    // Probe codecs and choose remux vs transcode.
    session.set_state(SessionState::Buffering).await;
    let info = probe(&env.ffprobe_path, &url, env.resolve_timeout).await?;
    session.set_duration(info.duration_secs).await;
    let encode = match select_mode(
        preferred_mode,
        &info,
        env.hwaccel_backend,
        env.hwaccel_device.as_deref(),
    )? {
        PlaybackMode::Remux => Encode::Copy,
        PlaybackMode::Transcode(e) => e,
    };
    tracing::debug!(session = %session.id, video = ?info.video_codec, audio = ?info.audio_codec, "playback mode selected");

    let cfg = HlsConfig {
        ffmpeg_path: env.ffmpeg_path.clone(),
        input_url: url,
        output_dir: session.dir.clone(),
        start_time_secs: None,
        encode,
    };
    let mut ff = FfmpegRunner::start(&cfg)?;

    let ready = tokio::select! {
        r = ff.wait_ready(env.ready_timeout) => r,
        _ = session.cancel.cancelled() => {
            ff.stop().await;
            return Ok(SessionEnd::Stopped);
        }
    };
    ready?;
    session.set_state(SessionState::Ready).await;

    // Serve: stop, idle expiry, or seek (restart FFmpeg at a new position).
    let end = serve(session, &mut ff, &cfg, env).await?;
    ff.stop().await;
    Ok(end)
}

/// One event the serve loop reacts to.
#[derive(Debug)]
enum Event {
    Stop,
    Seek(f64),
    IdleTick,
}

/// Wait for stop, seek, or an idle tick. Testable without FFmpeg.
async fn wait_event(
    cancel: &CancellationToken,
    seek_rx: &mut watch::Receiver<Option<f64>>,
    idle_check: Duration,
) -> Event {
    tokio::select! {
        _ = cancel.cancelled() => Event::Stop,
        changed = seek_rx.changed() => {
            if changed.is_ok() {
                if let Some(pos) = *seek_rx.borrow() {
                    Event::Seek(pos)
                } else {
                    Event::IdleTick
                }
            } else {
                Event::IdleTick
            }
        }
        _ = tokio::time::sleep(idle_check) => Event::IdleTick,
    }
}

/// Serve the ready session: handle stop, idle expiry, and seek (restart the
/// pipeline with `-ss` at the requested position).
async fn serve(
    session: &Arc<PlaybackSession>,
    ff: &mut FfmpegRunner,
    base_cfg: &HlsConfig,
    env: &SessionEnv,
) -> Result<SessionEnd, AppError> {
    let mut seek_rx = session.seek_rx.clone();
    let idle_check = env
        .idle_timeout
        .map(|t| Duration::from_secs(5).min(t))
        .unwrap_or(Duration::from_secs(3600));

    loop {
        match wait_event(&session.cancel, &mut seek_rx, idle_check).await {
            Event::Stop => return Ok(SessionEnd::Stopped),
            Event::IdleTick => {
                if let Some(timeout) = env.idle_timeout {
                    if session.idle_elapsed().await >= timeout {
                        return Ok(SessionEnd::Expired);
                    }
                }
            }
            Event::Seek(pos) => {
                ff.stop().await;
                // Clear stale HLS output so wait_ready detects the new run's first
                // segment (otherwise the previous segment file masks readiness).
                let out = base_cfg.output_dir.clone();
                let _ = tokio::fs::remove_dir_all(&out).await;
                let _ = tokio::fs::create_dir_all(&out).await;
                let cfg = HlsConfig {
                    start_time_secs: Some(pos),
                    ..base_cfg.clone()
                };
                *ff = FfmpegRunner::start(&cfg)?;
                ff.wait_ready(env.ready_timeout).await?;
                session.seek_done.notify_waiters();
                session.touch().await;
                tracing::info!(session = %session.id, position = pos, "seek restarted pipeline");
            }
        }
    }
}

fn select_file(file_id: Option<usize>, files: &[TorrentFile]) -> Result<usize, AppError> {
    match file_id {
        Some(id) if files.iter().any(|f| f.index == id) => Ok(id),
        Some(id) => Err(AppError::validation(format!("no such file: {id}"))),
        None => files
            .first()
            .map(|f| f.index)
            .ok_or_else(|| AppError::validation("torrent has no files")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(index: usize, size: u64) -> TorrentFile {
        TorrentFile {
            index,
            name: format!("f{index}"),
            size_bytes: size,
        }
    }

    #[test]
    fn select_file_validates_explicit_id() {
        let files = vec![file(0, 10), file(1, 20)];
        assert_eq!(select_file(Some(1), &files).unwrap(), 1);
        assert!(select_file(Some(9), &files).is_err());
    }

    #[test]
    fn select_file_defaults_to_largest() {
        // files are pre-sorted largest-first by TorrentEngine::files
        let files = vec![file(3, 999), file(0, 10)];
        assert_eq!(select_file(None, &files).unwrap(), 3);
    }

    #[tokio::test]
    async fn wait_event_seek() {
        let (tx, mut rx) = watch::channel(None);
        tx.send(Some(5.0)).unwrap();
        let cancel = CancellationToken::new();
        let event = wait_event(&cancel, &mut rx, Duration::from_secs(60)).await;
        match event {
            Event::Seek(pos) => assert_eq!(pos, 5.0),
            other => panic!("expected Seek, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn wait_event_stop() {
        let (_tx, mut rx) = watch::channel(None);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let event = wait_event(&cancel, &mut rx, Duration::from_secs(60)).await;
        assert!(matches!(event, Event::Stop));
    }

    #[tokio::test]
    async fn wait_event_idle_tick() {
        let (_tx, mut rx) = watch::channel(None);
        let cancel = CancellationToken::new();
        let event = wait_event(&cancel, &mut rx, Duration::from_millis(20)).await;
        assert!(matches!(event, Event::IdleTick));
    }
}
