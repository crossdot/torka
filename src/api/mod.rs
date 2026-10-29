pub mod health;

use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

use crate::{config::Config, error::AppError};

/// Builds the application router. `Config` is the shared state (Clone + Send + Sync).
pub fn router(config: Config) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(config)
}

async fn not_found() -> AppError {
    AppError::not_found("route not found")
}
