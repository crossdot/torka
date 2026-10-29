//! Seekable loopback HTTP-range source server (ADR-002).
//!
//! Serves torrent file streams over `http://127.0.0.1:<port>/stream/<token>`
//! with HTTP Range support, so FFmpeg can consume them via the `http` protocol
//! with seeking. Bound to loopback only; the token is an opaque per-stream id.

use std::{
    collections::HashMap,
    io::SeekFrom,
    sync::{Arc, RwLock},
};

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use librqbit::ManagedTorrent;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::error::AppError;

use super::SeekableStream;

type Stream = Box<dyn SeekableStream>;

#[derive(Clone)]
struct Route {
    handle: Arc<ManagedTorrent>,
    file_id: usize,
    len: u64,
}

#[derive(Clone, Default)]
struct SourceState {
    routes: Arc<RwLock<HashMap<String, Route>>>,
}

pub struct SourceServer {
    state: SourceState,
    addr: std::net::SocketAddr,
    join: tokio::task::JoinHandle<()>,
}

impl SourceServer {
    pub async fn bind() -> Result<Self, AppError> {
        let state = SourceState::default();
        let app = Router::new()
            .route("/stream/{token}", get(handler))
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| AppError::internal(format!("bind source server: {e}")))?;
        let addr = listener
            .local_addr()
            .map_err(|e| AppError::internal(format!("source addr: {e}")))?;

        let join = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        Ok(Self { state, addr, join })
    }

    /// Register a stream for a torrent file and return its loopback URL.
    pub fn register(
        &self,
        handle: Arc<ManagedTorrent>,
        file_id: usize,
    ) -> Result<String, AppError> {
        let len = handle
            .with_metadata(|m| m.file_infos.get(file_id).map(|f| f.len))
            .map_err(|e| AppError::internal(format!("file metadata: {e}")))?
            .ok_or_else(|| AppError::validation(format!("no such file: {file_id}")))?;

        let token = next_token();
        self.state
            .routes
            .write()
            .map_err(|_| AppError::internal("source registry poisoned"))?
            .insert(
                token.clone(),
                Route {
                    handle,
                    file_id,
                    len,
                },
            );

        Ok(format!("http://{}/stream/{token}", self.addr))
    }

    pub fn unregister(&self, token: &str) {
        if let Ok(mut routes) = self.state.routes.write() {
            routes.remove(token);
        }
    }

    /// Point an existing token at a fresh torrent handle (after a re-add on
    /// seek); the file id and length are unchanged.
    pub fn replace(&self, token: &str, handle: Arc<ManagedTorrent>) -> Result<(), AppError> {
        let mut routes = self
            .state
            .routes
            .write()
            .map_err(|_| AppError::internal("source registry poisoned"))?;
        if let Some(route) = routes.get_mut(token) {
            route.handle = handle;
        }
        Ok(())
    }

    pub fn shutdown(self) {
        self.join.abort();
    }
}

async fn handler(
    State(state): State<SourceState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let route = state
        .routes
        .read()
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "registry poisoned".into(),
            )
        })?
        .get(&token)
        .cloned()
        .ok_or((StatusCode::NOT_FOUND, "stream not found".into()))?;

    // A fresh seekable stream per request (FileStream is not shareable/cloneable).
    let stream: Stream = Box::new(
        route
            .handle
            .clone()
            .stream(route.file_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
    );
    let len = route.len;

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

    let mut stream = stream;
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

fn next_token() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("s{:x}", COUNTER.fetch_add(1, Ordering::Relaxed))
}
