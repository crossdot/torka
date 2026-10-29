//! Static provider registry: the fixed set of compiled-in providers, dispatched
//! through a single enum (no dynamic plugins in the MVP).

use super::{
    fake::FakeProvider,
    model::{SearchQuery, TorrentResult},
    provider::{Provider, ProviderError},
};

/// The set of compiled-in providers. Add real adapters (RuTracker/Rutor) here
/// in later phases.
#[derive(Clone)]
pub enum AnyProvider {
    Fake(FakeProvider),
}

impl Provider for AnyProvider {
    fn id(&self) -> &'static str {
        match self {
            Self::Fake(p) => p.id(),
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<TorrentResult>, ProviderError> {
        match self {
            Self::Fake(p) => p.search(query).await,
        }
    }
}

/// An ordered collection of enabled providers.
#[derive(Default)]
pub struct ProviderRegistry {
    providers: Vec<AnyProvider>,
}

impl ProviderRegistry {
    pub fn with_provider(mut self, provider: AnyProvider) -> Self {
        self.providers.push(provider);
        self
    }

    pub fn enabled(&self) -> &[AnyProvider] {
        &self.providers
    }

    /// Build from configuration. Adds the fake provider only when explicitly
    /// enabled (dev/test); real providers are wired in later phases.
    pub fn from_config(config: &crate::config::Config) -> Self {
        let mut registry = Self::default();
        if config.provider_fake_enabled {
            registry = registry.with_provider(AnyProvider::Fake(FakeProvider::new("fake")));
        }
        registry
    }
}
