//! Typed application error and the unified API error shape from `API.md`.
//!
//! `AppError` is the single error type returned by handlers; it maps to the
//! `{"error":{"code","message","request_id"}}` JSON contract and never leaks
//! internals. `request_id` is a process-local monotonic id for log correlation;
//! it is refined into a per-request value when the auth middleware lands.

use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_request_id() -> String {
    format!("{:016x}", REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: ApiErrorInner,
}

#[derive(Debug, Serialize)]
struct ApiErrorInner {
    code: &'static str,
    message: String,
    request_id: String,
}

#[derive(Debug)]
pub enum AppError {
    Validation(String),
    NotFound(String),
    Unauthorized(String),
    RateLimited(String),
    Internal(String),
}

impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }

    pub fn rate_limited(msg: impl Into<String>) -> Self {
        Self::RateLimited(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(m)
            | Self::NotFound(m)
            | Self::Unauthorized(m)
            | Self::RateLimited(m)
            | Self::Internal(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (code, status, message) = match self {
            Self::Validation(m) => ("VALIDATION_ERROR", StatusCode::BAD_REQUEST, m),
            Self::NotFound(m) => ("NOT_FOUND", StatusCode::NOT_FOUND, m),
            Self::Unauthorized(m) => ("UNAUTHORIZED", StatusCode::UNAUTHORIZED, m),
            Self::RateLimited(m) => ("RATE_LIMITED", StatusCode::TOO_MANY_REQUESTS, m),
            Self::Internal(m) => ("INTERNAL_ERROR", StatusCode::INTERNAL_SERVER_ERROR, m),
        };

        if status.is_server_error() {
            tracing::error!(code, message = %message, "request failed");
        }

        let body = ApiErrorBody {
            error: ApiErrorInner {
                code,
                message,
                request_id: next_request_id(),
            },
        };

        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn error_body_is_wrapped_in_error_object() {
        let resp = AppError::not_found("boom").into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(v["error"]["code"], "NOT_FOUND");
        assert_eq!(v["error"]["message"], "boom");
        assert!(v["error"]["request_id"].is_string());
        assert!(v.get("code").is_none(), "must not leak flat `code`");
    }
}
