//! Parallel search coordination: bounded concurrency, per-provider timeout,
//! partial results, and dedupe by validated info hash.

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use futures_util::future::join_all;

use super::{
    model::{ProviderStatus, SearchQuery, SearchResponse, TorrentResult, normalize_info_hash},
    provider::{Provider, ProviderError},
    registry::{AnyProvider, ProviderRegistry},
};

pub struct SearchService {
    registry: ProviderRegistry,
    timeout: Duration,
    max_parallel: usize,
}

impl SearchService {
    pub fn new(registry: ProviderRegistry, timeout: Duration, max_parallel: usize) -> Self {
        Self {
            registry,
            timeout,
            max_parallel,
        }
    }

    /// Run every enabled provider concurrently (chunked by `max_parallel`),
    /// each under `timeout`, and aggregate partial results. A failing or timed-out
    /// provider only affects its own status; other providers' results survive.
    pub async fn search(&self, query: &SearchQuery) -> SearchResponse {
        let mut items = Vec::new();
        let mut statuses = Vec::new();

        let chunk_size = self.max_parallel.max(1);
        for chunk in self.registry.enabled().chunks(chunk_size) {
            let futures = chunk.iter().map(|p| run_one(p, query, self.timeout));
            for (mut found, status) in join_all(futures).await {
                items.append(&mut found);
                statuses.push(status);
            }
        }

        SearchResponse {
            items: dedupe(items),
            providers: statuses,
        }
    }

    /// Stable ids of the currently enabled providers (for diagnostics).
    pub fn enabled_provider_ids(&self) -> Vec<String> {
        self.registry
            .enabled()
            .iter()
            .map(|p| p.id().to_string())
            .collect()
    }
}

async fn run_one(
    provider: &AnyProvider,
    query: &SearchQuery,
    timeout: Duration,
) -> (Vec<TorrentResult>, ProviderStatus) {
    let start = Instant::now();
    let result = tokio::time::timeout(timeout, provider.search(query)).await;
    let duration_ms = start.elapsed().as_millis();

    let (items, status, code) = match result {
        Ok(Ok(items)) => (items, "ok", None),
        Ok(Err(e)) => {
            let code = e.code();
            let status = if matches!(&e, ProviderError::Timeout) {
                "timeout"
            } else {
                "error"
            };
            tracing::warn!(provider = provider.id(), code, error = %e, "provider search failed");
            (Vec::new(), status, Some(code))
        }
        Err(_) => (Vec::new(), "timeout", Some(ProviderError::Timeout.code())),
    };

    (
        items,
        ProviderStatus {
            id: provider.id().to_string(),
            status: status.to_string(),
            duration_ms,
            error_code: code,
        },
    )
}

/// Drop later results that repeat an already-seen valid info hash. Results with
/// no/invalid info hash are kept (dedupe only on the validated hash).
fn dedupe(items: Vec<TorrentResult>) -> Vec<TorrentResult> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        if let Some(key) = item.info_hash.as_deref().and_then(normalize_info_hash) {
            if !seen.insert(key) {
                continue;
            }
        }
        out.push(item);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{
        fake::FakeProvider,
        model::{SearchQuery, TorrentResult},
    };

    fn query(q: &str) -> SearchQuery {
        SearchQuery::normalize(q.to_string(), None).unwrap()
    }

    #[tokio::test]
    async fn partial_results_when_one_provider_fails() {
        let registry = ProviderRegistry::default()
            .with_provider(AnyProvider::Fake(FakeProvider::new("good")))
            .with_provider(AnyProvider::Fake(FakeProvider::new("bad").with_fail()));
        let service = SearchService::new(registry, Duration::from_secs(5), 4);

        let resp = service.search(&query("matrix")).await;

        assert_eq!(
            resp.items.len(),
            2,
            "only the good provider's results survive"
        );
        assert_eq!(resp.providers.len(), 2);
        let bad = resp.providers.iter().find(|p| p.id == "bad").unwrap();
        assert_eq!(bad.status, "error");
        assert_eq!(bad.error_code, Some("PROVIDER_UNAVAILABLE"));
    }

    #[tokio::test]
    async fn provider_timeout_is_isolated() {
        let registry = ProviderRegistry::default()
            .with_provider(AnyProvider::Fake(FakeProvider::new("good")))
            .with_provider(AnyProvider::Fake(
                FakeProvider::new("slow").with_delay(Duration::from_secs(30)),
            ));
        let service = SearchService::new(registry, Duration::from_millis(100), 4);

        let resp = service.search(&query("matrix")).await;

        assert_eq!(resp.items.len(), 2);
        let slow = resp.providers.iter().find(|p| p.id == "slow").unwrap();
        assert_eq!(slow.status, "timeout");
        assert_eq!(slow.error_code, Some("PROVIDER_TIMEOUT"));
    }

    #[test]
    fn dedupes_by_validated_info_hash_only() {
        let mk = |id: &str, hash: Option<&str>| TorrentResult {
            id: id.to_string(),
            title: "t".into(),
            year: None,
            provider_id: "p".into(),
            size_bytes: None,
            seeders: None,
            leechers: None,
            info_hash: hash.map(str::to_string),
            magnet_available: false,
            quality: None,
            languages: vec![],
            source_url: None,
            magnet: None,
        };

        let items = vec![
            mk("a", Some("0123456789abcdef0123456789abcdef01234567")),
            mk("b", Some("0123456789ABCDEF0123456789ABCDEF01234567")), // same hash, different case
            mk("c", None),
            mk("d", Some("not-a-valid-hash")),
        ];
        let out = dedupe(items);
        assert_eq!(
            out.len(),
            3,
            "duplicate hash dropped; null/invalid hashes kept"
        );
        assert_eq!(
            out.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["a", "c", "d"]
        );
    }
}
