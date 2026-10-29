//! Static provider registry: the fixed set of compiled-in providers, dispatched
//! through a single enum (no dynamic plugins in the MVP).

use std::time::Duration;

use crate::{
    config::Config,
    providers::{HttpClient, RuTrackerProvider, RutorProvider},
};

use super::{
    fake::FakeProvider,
    model::{SearchQuery, TorrentResult},
    provider::{Provider, ProviderError},
};

/// The set of compiled-in providers.
#[derive(Clone)]
pub enum AnyProvider {
    Fake(FakeProvider),
    RuTracker(RuTrackerProvider),
    Rutor(RutorProvider),
}

impl Provider for AnyProvider {
    fn id(&self) -> &'static str {
        match self {
            Self::Fake(p) => p.id(),
            Self::RuTracker(p) => p.id(),
            Self::Rutor(p) => p.id(),
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<TorrentResult>, ProviderError> {
        match self {
            Self::Fake(p) => p.search(query).await,
            Self::RuTracker(p) => p.search(query).await,
            Self::Rutor(p) => p.search(query).await,
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

    /// Build from configuration. Each provider is added only when its flag is
    /// set; the fake provider is for dev/test only.
    pub fn from_config(config: &Config) -> Result<Self, String> {
        let mut registry = Self::default();

        if config.provider_fake_enabled {
            registry = registry.with_provider(AnyProvider::Fake(FakeProvider::new("fake")));
        }

        if config.provider_rutracker_enabled || config.provider_rutor_enabled {
            let http = HttpClient::new(Duration::from_secs(config.search_timeout_secs))
                .map_err(|e| format!("failed to build HTTP client: {e}"))?;

            if config.provider_rutracker_enabled {
                registry = registry.with_provider(AnyProvider::RuTracker(RuTrackerProvider::new(
                    http.clone(),
                    config.provider_rutracker_forum_id.clone(),
                )));
            }
            if config.provider_rutor_enabled {
                registry = registry.with_provider(AnyProvider::Rutor(RutorProvider::new(http)));
            }
        }

        Ok(registry)
    }
}
