//! Search provider implementations (RuTracker feed, Rutor HTML) plus the shared
//! HTTP client and magnet parsing. Providers are compiled-in and dispatched
//! through [`crate::search::registry::AnyProvider`].

pub mod http;
pub mod magnet;
pub mod rutor;
pub mod rutracker;

pub use http::HttpClient;
pub use rutor::RutorProvider;
pub use rutracker::RuTrackerProvider;
