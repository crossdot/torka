//! The [`Provider`] trait and its error type.

use std::future::Future;

use super::model::{SearchQuery, TorrentResult};

/// Error a provider can return. Internal messages are for logs only; the mapped
/// [`code`](Self::code) is what gets exposed to clients.
#[derive(Debug)]
pub enum ProviderError {
    Timeout,
    Unavailable(String),
}

impl ProviderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Timeout => "PROVIDER_TIMEOUT",
            Self::Unavailable(_) => "PROVIDER_UNAVAILABLE",
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => f.write_str("provider timed out"),
            Self::Unavailable(m) => write!(f, "provider unavailable: {m}"),
        }
    }
}

impl std::error::Error for ProviderError {}

/// A search provider. Implementations are compiled-in and dispatched through
/// [`super::registry::AnyProvider`] (static registry, no dynamic plugins).
///
/// Uses return-position `impl Trait` (not `async fn`) so the future carries an
/// explicit `Send` bound and the `async_fn_in_trait` lint is avoided.
pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;

    fn search(
        &self,
        query: &SearchQuery,
    ) -> impl Future<Output = Result<Vec<TorrentResult>, ProviderError>> + Send;
}
