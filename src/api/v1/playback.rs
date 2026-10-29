//! Playback session HTTP API (API.md): create/status/stop sessions and serve
//! HLS playlist/segments, scoped to the owning session.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiResult, AppError},
    state::AppState,
    streaming::{SessionState, torrent::TorrentSource},
};

#[derive(Deserialize)]
struct CreatePlaybackRequest {
    source_id: String,
    file_id: Option<usize>,
    /// Accepted for API compatibility; only remux is implemented (Phase 7 adds
    /// transcode-mode selection).
    #[serde(default)]
    #[allow(dead_code)]
    preferred_mode: Option<String>,
}

#[derive(Serialize)]
struct CreatePlaybackResponse {
    session_id: String,
    status: String,
    status_url: String,
}

#[derive(Serialize)]
struct SessionStatusResponse {
    session_id: String,
    status: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/playback/sessions", post(create))
        .route("/playback/sessions/{id}", get(status).delete(stop))
        .route("/playback/sessions/{id}/master.m3u8", get(master_playlist))
        .route("/playback/sessions/{id}/segments/{name}", get(segment))
}

async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreatePlaybackRequest>,
) -> ApiResult<(StatusCode, Json<CreatePlaybackResponse>)> {
    // Resolve the opaque source_id to an info hash, then a magnet.
    let info_hash = state
        .cache
        .get(&req.source_id)
        .ok_or_else(|| AppError::validation("unknown source_id"))?;
    let magnet = format!("magnet:?xt=urn:btih:{info_hash}");
    let source = TorrentSource::from_magnet(magnet)?;

    let (session_id, state0) = state.streaming.create(source, req.file_id, None).await?;
    let status_url = format!("/api/v1/playback/sessions/{session_id}");

    Ok((
        StatusCode::ACCEPTED,
        Json(CreatePlaybackResponse {
            session_id: session_id.clone(),
            status: state0.as_str().to_string(),
            status_url,
        }),
    ))
}

async fn status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<SessionStatusResponse>> {
    let status = state
        .streaming
        .status(&id)
        .await
        .ok_or_else(|| AppError::not_found("session not found"))?;
    Ok(Json(SessionStatusResponse {
        session_id: id,
        status: status.as_str().to_string(),
    }))
}

/// Idempotent stop.
async fn stop(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    state.streaming.stop(&id).await;
    StatusCode::ACCEPTED
}

async fn master_playlist(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    match state.streaming.status(&id).await {
        Some(SessionState::Ready) | Some(SessionState::Playing) => {}
        Some(_) => return Err(AppError::validation("session not ready")),
        None => return Err(AppError::not_found("session not found")),
    }

    let path = state
        .streaming
        .playlist_path(&id)
        .ok_or_else(|| AppError::not_found("playlist not found"))?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| AppError::internal(format!("read playlist: {e}")))?;

    // Backend signal of playback start (STREAMING.md §8: `playing` is a
    // documented backend-observed event, not proof of client decoding).
    state.streaming.mark_playing(&id).await;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")],
        bytes,
    )
        .into_response())
}

async fn segment(
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let path = state
        .streaming
        .segment_path(&id, &name)
        .ok_or_else(|| AppError::not_found("segment not found"))?;
    state.streaming.touch(&id).await;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| AppError::internal(format!("read segment: {e}")))?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "video/mp2t")],
        bytes,
    )
        .into_response())
}
