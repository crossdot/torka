//! Normalized search models shared across providers and the HTTP layer.

use serde::Serialize;

/// A normalized torrent search result. Fields a provider does not expose stay `None`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TorrentResult {
    /// Opaque, stable result id; passed back as `source_id` on playback creation.
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    pub provider_id: String,
    pub size_bytes: Option<u64>,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    pub info_hash: Option<String>,
    pub magnet_available: bool,
    pub quality: Option<String>,
    pub languages: Vec<String>,
    pub source_url: Option<String>,
}

/// Per-provider outcome in a search response.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderStatus {
    pub id: String,
    /// `"ok"`, `"error"`, or `"timeout"`.
    pub status: String,
    pub duration_ms: u128,
    pub error_code: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SearchResponse {
    pub items: Vec<TorrentResult>,
    pub providers: Vec<ProviderStatus>,
}

/// Validated, normalized search query.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchQuery {
    pub q: String,
    pub year: Option<u16>,
}

impl SearchQuery {
    /// Trims `q` and rejects empty or over-long input.
    pub fn normalize(q: String, year: Option<u16>) -> Result<Self, String> {
        let q = q.trim().to_string();
        if q.is_empty() {
            return Err("search query `q` must not be empty".to_string());
        }
        if q.chars().count() > 200 {
            return Err("search query `q` is too long".to_string());
        }
        Ok(Self { q, year })
    }
}

/// Lowercased 40-char hex info hash for dedupe, or `None` if the value is not a
/// valid BTIH. Invalid hashes are not used as a dedupe key.
pub fn normalize_info_hash(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() != 40 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(s.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_and_rejects_empty() {
        assert_eq!(
            SearchQuery::normalize("  matrix  ".into(), Some(2020)).unwrap(),
            SearchQuery {
                q: "matrix".into(),
                year: Some(2020)
            }
        );
        assert!(SearchQuery::normalize("   ".into(), None).is_err());
        assert!(SearchQuery::normalize("".into(), None).is_err());
    }

    #[test]
    fn info_hash_normalization() {
        let h = "ABCDEF0123456789ABCDEF0123456789ABCDEF01";
        assert_eq!(
            normalize_info_hash(h).as_deref(),
            Some(&h.to_ascii_lowercase()[..])
        );
        assert_eq!(normalize_info_hash("short"), None);
        assert_eq!(normalize_info_hash(&"z".repeat(40)), None);
    }
}
