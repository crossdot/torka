use axum::{Json, extract::State};
use serde::Serialize;

use crate::{config::Config, error::ApiResult};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

/// Liveness: the process is up and able to serve requests.
pub async fn health(State(_config): State<Config>) -> ApiResult<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

/// Readiness: mandatory components are ready. Phase 1 has no external
/// dependencies, so readiness is currently unconditional and does not depend on
/// any external site (per API.md).
pub async fn ready(State(_config): State<Config>) -> ApiResult<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        status: "ready",
        version: env!("CARGO_PKG_VERSION"),
    }))
}
