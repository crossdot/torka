//! Streaming manager: owns the torrent engine + source server and the in-memory
//! playback-session registry, enforcing the session limit and lifecycle.

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

use crate::{config::Config, error::AppError};

use super::{
    session::{PlaybackSession, SessionEnv, SessionState, run_session},
    source::SourceServer,
    torrent::{TorrentEngine, TorrentSource},
};

pub struct StreamingManager {
    engine: Arc<TorrentEngine>,
    source: Arc<SourceServer>,
    sessions: Arc<Mutex<HashMap<String, Arc<PlaybackSession>>>>,
    session_dir: PathBuf,
    limit: Arc<Semaphore>,
    ready_timeout: Duration,
    resolve_timeout: Duration,
    ffmpeg_path: String,
}

impl StreamingManager {
    pub async fn new(config: &Config) -> Result<Self, AppError> {
        let engine = Arc::new(TorrentEngine::new(config.cache_dir.clone()).await?);
        let source = Arc::new(SourceServer::bind().await?);
        Ok(Self {
            engine,
            source,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            session_dir: PathBuf::from(&config.session_dir),
            limit: Arc::new(Semaphore::new(config.max_active_sessions.max(1))),
            ready_timeout: Duration::from_secs(config.ffmpeg_start_timeout_secs),
            resolve_timeout: Duration::from_secs(config.torrent_resolve_timeout_secs),
            ffmpeg_path: config.ffmpeg_path.clone(),
        })
    }

    /// Create a session and drive it in a background task. Returns the session
    /// id and its initial state (`created`).
    pub async fn create(
        &self,
        source: TorrentSource,
        file_id: Option<usize>,
        output_folder: Option<PathBuf>,
    ) -> Result<(String, SessionState), AppError> {
        self.prune_terminal().await;

        let permit = self
            .limit
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::validation("session limit reached"))?;

        let id = Uuid::new_v4().to_string();
        let dir = self.session_dir.join(&id);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| AppError::internal(format!("create session dir: {e}")))?;

        let session = Arc::new(PlaybackSession::new(id.clone(), dir));

        self.sessions
            .lock()
            .await
            .insert(id.clone(), session.clone());

        let env = SessionEnv {
            engine: self.engine.clone(),
            source: self.source.clone(),
            ffmpeg_path: self.ffmpeg_path.clone(),
            ready_timeout: self.ready_timeout,
            resolve_timeout: self.resolve_timeout,
        };

        tokio::spawn(async move {
            let _permit = permit; // held for the session lifetime
            run_session(session, source, file_id, output_folder, env).await;
        });

        Ok((id, SessionState::Created))
    }

    /// Drop sessions that have reached a terminal state (kept in the registry
    /// so `status` can report `stopped`/`failed` after stop).
    async fn prune_terminal(&self) {
        let snapshot: Vec<(String, Arc<PlaybackSession>)> = self
            .sessions
            .lock()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let mut terminal = Vec::new();
        for (id, s) in snapshot {
            if s.state().await.is_terminal() {
                terminal.push(id);
            }
        }

        if !terminal.is_empty() {
            let mut sessions = self.sessions.lock().await;
            for id in terminal {
                sessions.remove(&id);
            }
        }
    }

    pub async fn status(&self, id: &str) -> Option<SessionState> {
        let session = self.sessions.lock().await.get(id).cloned()?;
        Some(session.state().await)
    }

    /// Idempotent stop: signals cancellation and removes the session.
    pub async fn stop(&self, id: &str) {
        if let Some(session) = self.sessions.lock().await.get(id).cloned() {
            session.cancel();
        }
    }

    /// Transition `ready -> playing` when a client fetches the playlist.
    pub async fn mark_playing(&self, id: &str) {
        if let Some(session) = self.sessions.lock().await.get(id).cloned() {
            session.mark_playing().await;
        }
    }

    pub fn playlist_path(&self, id: &str) -> Option<PathBuf> {
        let dir = self.session_dir.join(id);
        let p = dir.join("out.m3u8");
        p.is_file().then_some(p)
    }

    /// Resolve a validated segment filename to its path, or `None` if invalid.
    pub fn segment_path(&self, id: &str, name: &str) -> Option<PathBuf> {
        if !is_safe_segment_name(name) {
            return None;
        }
        let p = self.session_dir.join(id).join(name);
        p.is_file().then_some(p)
    }
}

/// A single path component matching `seg_<digits>.ts` — no traversal.
fn is_safe_segment_name(name: &str) -> bool {
    let rest = name.strip_prefix("seg_").unwrap_or(name);
    let digits = rest.strip_suffix(".ts").unwrap_or(rest);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_name_validation() {
        assert!(is_safe_segment_name("seg_000.ts"));
        assert!(is_safe_segment_name("seg_42.ts"));
        assert!(!is_safe_segment_name("../etc/passwd"));
        assert!(!is_safe_segment_name("out.m3u8"));
        assert!(!is_safe_segment_name("seg_.ts"));
        assert!(!is_safe_segment_name("seg_abc.ts"));
    }
}
