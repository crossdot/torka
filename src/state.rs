use std::sync::Arc;

use crate::{config::Config, search::SearchService};

/// Shared application state (Clone + Send + Sync), passed to handlers via
/// axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub search: Arc<SearchService>,
}
