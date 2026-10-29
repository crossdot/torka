pub mod health;
pub mod v1;

use std::path::Path;

use axum::{Router, middleware, routing::get};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{auth, rate_limit, state::AppState};

/// Builds the application router. `AppState` is the shared state (Clone + Send + Sync).
pub fn router(state: AppState) -> Router {
    let api_token = state.config.api_token.clone();
    let rate_limiter = state.rate_limiter.clone();
    let spa = spa(&state.config.static_dir);

    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .nest(
            "/api/v1",
            v1::router()
                .layer(middleware::from_fn_with_state(
                    api_token,
                    auth::require_auth,
                ))
                .layer(middleware::from_fn_with_state(
                    rate_limiter,
                    rate_limit::rate_limit,
                )),
        )
        .fallback_service(spa)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Serves the built Vue SPA; unknown non-API paths fall back to `index.html`.
/// `.fallback` (not `.not_found_service`) keeps the 200 status for client-side routes.
fn spa(static_dir: &str) -> ServeDir<ServeFile> {
    let index = Path::new(static_dir).join("index.html");
    ServeDir::new(static_dir).fallback(ServeFile::new(index))
}
