use std::sync::Arc;

use crate::{
    config::Config,
    rate_limit::RateLimiter,
    search::{SearchCache, SearchService},
    streaming::StreamingManager,
};

/// Shared application state (Clone + Send + Sync), passed to handlers via
/// axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub search: Arc<SearchService>,
    pub cache: Arc<SearchCache>,
    pub streaming: Arc<StreamingManager>,
    pub rate_limiter: Arc<RateLimiter>,
}
