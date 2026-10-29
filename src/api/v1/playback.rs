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
    /// Preferred playback mode: `auto` (default), `remux`, or `transcode`.
    #[serde(default)]
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
    error: Option<String>,
    duration_secs: Option<f64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/playback/sessions", post(create))
        .route("/playback/sessions/{id}", get(status).delete(stop))
        .route("/playback/sessions/{id}/seek", post(seek))
        .route("/playback/sessions/{id}/master.m3u8", get(master_playlist))
        .route("/playback/sessions/{id}/segments/{name}", get(segment))
}

async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreatePlaybackRequest>,
) -> ApiResult<(StatusCode, Json<CreatePlaybackResponse>)> {
    // Resolve the opaque source_id to the cached magnet (with trackers).
    let magnet = state
        .cache
        .get(&req.source_id)
        .ok_or_else(|| AppError::validation("unknown source_id"))?;
    let source = TorrentSource::from_magnet(magnet)?;

    let (session_id, state0) = state
        .streaming
        .create(source, req.file_id, None, req.preferred_mode)
        .await?;
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
    let (status, error, duration_secs) = state
        .streaming
        .status_with_error(&id)
        .await
        .ok_or_else(|| AppError::not_found("session not found"))?;
    Ok(Json(SessionStatusResponse {
        session_id: id,
        status: status.as_str().to_string(),
        error,
        duration_secs,
    }))
}

/// Idempotent stop.
async fn stop(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    state.streaming.stop(&id).await;
    StatusCode::ACCEPTED
}

#[derive(Deserialize)]
struct SeekRequest {
    position_secs: f64,
}

/// Seek the session: restart the pipeline at `position_secs` (new generation).
async fn seek(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SeekRequest>,
) -> ApiResult<StatusCode> {
    if !req.position_secs.is_finite() || req.position_secs < 0.0 {
        return Err(AppError::validation("invalid position_secs"));
    }
    match state.streaming.status(&id).await {
        Some(SessionState::Ready) | Some(SessionState::Playing) => {}
        Some(_) => return Err(AppError::validation("session not ready")),
        None => return Err(AppError::not_found("session not found")),
    }
    state.streaming.seek(&id, req.position_secs).await?;
    Ok(StatusCode::ACCEPTED)
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
