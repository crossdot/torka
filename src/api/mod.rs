pub mod health;

use std::path::Path;

use axum::{Router, extract::State, routing::get};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{config::Config, error::AppError};

/// Builds the application router. `Config` is the shared state (Clone + Send + Sync).
pub fn router(config: Config) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .nest("/api/v1", api_v1())
        .fallback_service(spa(&config.static_dir))
        .layer(TraceLayer::new_for_http())
        .with_state(config)
}

/// `/api/v1` scope. Phase 2 adds search/session routes here; until then every
/// unmatched `/api/v1/*` path returns the unified JSON 404 (never the SPA).
fn api_v1() -> Router<Config> {
    Router::new().fallback(api_not_found)
}

async fn api_not_found(_state: State<Config>) -> AppError {
    AppError::not_found("route not found")
}

/// Serves the built Vue SPA; unknown non-API paths fall back to `index.html`.
/// `.fallback` (not `.not_found_service`) keeps the 200 status for client-side routes.
fn spa(static_dir: &str) -> ServeDir<ServeFile> {
    let index = Path::new(static_dir).join("index.html");
    ServeDir::new(static_dir).fallback(ServeFile::new(index))
}
