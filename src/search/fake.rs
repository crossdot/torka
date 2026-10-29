//! Deterministic in-memory provider for tests and local development.
//!
//! Synthetic by construction — never a substitute for a real source, and never
//! enabled unless `PROVIDER_FAKE_ENABLED` is set.

use std::time::Duration;

use super::{
    model::{SearchQuery, TorrentResult},
    provider::{Provider, ProviderError},
};

#[derive(Clone)]
pub struct FakeProvider {
    id: &'static str,
    delay: Option<Duration>,
    fail: bool,
}

impl FakeProvider {
    pub fn new(id: &'static str) -> Self {
        Self {
            id,
            delay: None,
            fail: false,
        }
    }

    /// Simulate a slow provider (used to exercise timeouts).
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Simulate a provider that always fails.
    pub fn with_fail(mut self) -> Self {
        self.fail = true;
        self
    }
}

impl Provider for FakeProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<TorrentResult>, ProviderError> {
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }
        if self.fail {
            return Err(ProviderError::Unavailable(format!("{} is down", self.id)));
        }

        Ok(vec![
            TorrentResult {
                id: format!("{}-1", self.id),
                title: format!("{} [1080p]", query.q),
                year: query.year.or(Some(2024)),
                provider_id: self.id.to_string(),
                size_bytes: Some(1_400_000_000),
                seeders: Some(42),
                leechers: Some(3),
                info_hash: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
                magnet_available: true,
                quality: Some("1080p".to_string()),
                languages: vec!["ru".to_string(), "en".to_string()],
                source_url: None,
                magnet: None,
            },
            TorrentResult {
                id: format!("{}-2", self.id),
                title: format!("{} [720p]", query.q),
                year: query.year.or(Some(2023)),
                provider_id: self.id.to_string(),
                size_bytes: Some(900_000_000),
                seeders: None,
                leechers: None,
                info_hash: None,
                magnet_available: false,
                quality: Some("720p".to_string()),
                languages: vec![],
                source_url: None,
                magnet: None,
            },
        ])
    }
}
