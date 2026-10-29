//! Search contracts: normalized models, the provider trait, a static registry,
//! and the parallel [`SearchService`].

pub mod fake;
pub mod model;
pub mod provider;
pub mod registry;
pub mod service;

pub use model::{SearchQuery, SearchResponse, TorrentResult};
pub use registry::ProviderRegistry;
pub use service::SearchService;
