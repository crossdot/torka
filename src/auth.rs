//! Bearer-token guard (ADR-005). When `APP_API_TOKEN` is configured, every
//! `/api/*` request must present `Authorization: Bearer <token>`; otherwise the
//! API is open (startup logs a warning). Health/readiness and the SPA stay public.

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::error::AppError;

pub async fn require_auth(
    State(token): State<Option<String>>,
    request: Request,
    next: Next,
) -> Response {
    if let Some(expected) = token {
        let ok = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|t| t == expected);

        if !ok {
            return AppError::unauthorized("invalid or missing bearer token").into_response();
        }
    }
    next.run(request).await
}
