//! Playback session lifecycle: state machine + orchestration (resolve torrent →
//! register source → FFmpeg HLS → ready → cleanup), with idle-timeout expiry.

use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;

use super::{
    ffmpeg::{FfmpegRunner, HlsConfig},
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
}

impl PlaybackSession {
    pub fn new(id: String, dir: PathBuf) -> Self {
        Self {
            id,
            dir,
            state: Arc::new(Mutex::new(SessionState::Created)),
            cancel: CancellationToken::new(),
            last_activity: Arc::new(Mutex::new(Instant::now())),
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
}

/// Shared services a session task uses.
pub struct SessionEnv {
    pub engine: Arc<TorrentEngine>,
    pub source: Arc<SourceServer>,
    pub ffmpeg_path: String,
    pub ready_timeout: Duration,
    pub resolve_timeout: Duration,
    /// Idle timeout after `ready`; `None` disables expiry.
    pub idle_timeout: Option<Duration>,
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
    env: SessionEnv,
) {
    let mut token: Option<String> = None;

    let result = orchestrate(&session, source, file_id, output_folder, &env, &mut token).await;

    if let Some(t) = token {
        env.source.unregister(&t);
    }
    let _ = tokio::fs::remove_dir_all(&session.dir).await;

    let final_state = match result {
        Ok(SessionEnd::Stopped) => SessionState::Stopped,
        Ok(SessionEnd::Expired) => SessionState::Expired,
        Err(e) => {
            tracing::warn!(session = %session.id, error = %e, "playback failed");
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

    // Start FFmpeg -> HLS and wait for the first playable segment.
    session.set_state(SessionState::Buffering).await;
    let cfg = HlsConfig {
        ffmpeg_path: env.ffmpeg_path.clone(),
        input_url: url,
        output_dir: session.dir.clone(),
        start_time_secs: None,
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

    // Stay ready until stopped or idle-expired.
    let end = wait_for_stop_or_idle(session, env.idle_timeout).await;
    ff.stop().await;
    Ok(end)
}

/// Wait for stop (cancel) or idle expiry, whichever comes first.
async fn wait_for_stop_or_idle(
    session: &Arc<PlaybackSession>,
    idle_timeout: Option<Duration>,
) -> SessionEnd {
    let Some(timeout) = idle_timeout else {
        session.cancel.cancelled().await;
        return SessionEnd::Stopped;
    };

    let check = Duration::from_secs(5).min(timeout);
    loop {
        tokio::select! {
            _ = session.cancel.cancelled() => return SessionEnd::Stopped,
            _ = tokio::time::sleep(check) => {
                if session.idle_elapsed().await >= timeout {
                    return SessionEnd::Expired;
                }
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
    async fn idle_timeout_expires_session() {
        let session = Arc::new(PlaybackSession::new("x".into(), "/tmp/x".into()));
        let end = wait_for_stop_or_idle(&session, Some(Duration::from_millis(50))).await;
        assert!(matches!(end, SessionEnd::Expired));
    }

    #[tokio::test]
    async fn cancel_stops_before_idle_expiry() {
        let session = Arc::new(PlaybackSession::new("x".into(), "/tmp/x".into()));
        let s2 = session.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            s2.cancel();
        });
        let end = wait_for_stop_or_idle(&session, Some(Duration::from_secs(60))).await;
        assert!(matches!(end, SessionEnd::Stopped));
    }
}
