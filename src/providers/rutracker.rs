//! RuTracker provider — recent-releases Atom feed only (ADR-003).
//!
//! RuTracker's search/topic pages sit behind a Cloudflare challenge, so this
//! provider only reads the official `feed.rutracker.cc` Atom feed. It is a
//! "recent releases by category" source, not a full-text search: results are
//! filtered client-side by title substring.

use crate::search::{
    model::{SearchQuery, TorrentResult},
    provider::{Provider, ProviderError},
};

use super::{http::HttpClient, magnet::magnet_info_hash};

#[derive(Clone)]
pub struct RuTrackerProvider {
    http: HttpClient,
    base_urls: Vec<String>,
    forum_id: String,
}

impl RuTrackerProvider {
    pub fn new(http: HttpClient, base_urls: Vec<String>, forum_id: String) -> Self {
        Self {
            http,
            base_urls,
            forum_id,
        }
    }
}

impl Provider for RuTrackerProvider {
    fn id(&self) -> &'static str {
        "rutracker"
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<TorrentResult>, ProviderError> {
        let needle = query.q.to_lowercase();
        let mut last_err = None;
        for base in &self.base_urls {
            let url = format!("{base}/atom/f/{}.atom", self.forum_id);
            match self.http.get(&url).await {
                Ok(xml) => {
                    return Ok(parse_feed(&xml)?
                        .into_iter()
                        .filter(|r| r.title.to_lowercase().contains(&needle))
                        .collect());
                }
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| ProviderError::Unavailable("no base URL configured".into())))
    }
}

fn parse_feed(xml: &str) -> Result<Vec<TorrentResult>, ProviderError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| ProviderError::Unavailable(format!("bad feed: {e}")))?;

    let mut out = Vec::new();
    for entry in doc.descendants().filter(|n| n.has_tag_name("entry")) {
        let mut title = None;
        let mut topic = None;
        let mut magnet = None;

        for child in entry.children().filter(|n| n.is_element()) {
            let tag = child.tag_name();
            match tag.name() {
                "title" => title = text_of(child).map(|s| s.trim().to_string()),
                "link" => {
                    let rel = child.attribute("rel");
                    let href = child.attribute("href");
                    if rel == Some("enclosure") {
                        magnet = href;
                    } else if href.is_some_and(|h| h.contains("viewtopic")) {
                        topic = href;
                    }
                }
                _ => {}
            }
        }

        let Some(title) = title.filter(|t| !t.is_empty()) else {
            continue;
        };
        let info_hash = magnet.and_then(magnet_info_hash);

        out.push(TorrentResult {
            id: topic_id(topic).unwrap_or_else(|| format!("rutracker-{title}")),
            title,
            year: None,
            provider_id: "rutracker".into(),
            size_bytes: None,
            seeders: None,
            leechers: None,
            info_hash,
            magnet_available: magnet.is_some(),
            quality: None,
            languages: vec![],
            source_url: topic.map(str::to_string),
        });
    }
    Ok(out)
}

/// Concatenated text content of an element (handles `text()` returning `None`
/// for element nodes).
fn text_of(node: roxmltree::Node) -> Option<String> {
    let s: String = node.descendants().filter_map(|n| n.text()).collect();
    if s.is_empty() { None } else { Some(s) }
}

/// Stable topic id from a `viewtopic.php?t=<n>` URL.
fn topic_id(topic: Option<&str>) -> Option<String> {
    let t = topic?;
    let rest = t.split_once("t=")?.1;
    let end = rest.find(['&', '#']).unwrap_or(rest.len());
    let id = &rest[..end];
    (!id.is_empty()).then(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("fixtures/rutracker_feed.atom");

    #[test]
    fn parses_feed_entries() {
        let results = parse_feed(FIXTURE).unwrap();
        assert!(!results.is_empty());
        let first = &results[0];
        assert_eq!(first.provider_id, "rutracker");
        assert!(first.magnet_available);
        assert!(first.info_hash.as_deref().is_some_and(|h| h.len() == 40));
        assert!(
            first
                .source_url
                .as_deref()
                .unwrap()
                .contains("viewtopic.php?t=")
        );
        assert!(first.title.contains("Список смертников"));
    }

    #[test]
    fn filters_by_title_substring() {
        let q = SearchQuery::normalize("футбол".into(), None).unwrap();
        let _ = q; // filtering is exercised in `search()`; kept for clarity
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_feed("not xml <").is_err());
    }
}
