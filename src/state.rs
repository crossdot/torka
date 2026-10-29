use std::sync::Arc;

use crate::{
    config::Config,
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
}
