//! Rutor provider — HTML search (ADR-003: `robots.txt` allows crawling with
//! `Crawl-delay: 3`; no login/CAPTCHA on search). The HTML contract is treated
//! as unstable; parsing is fixture-tested and malformed input degrades to an
//! error rather than crashing.

use crate::search::{
    model::{SearchQuery, TorrentResult},
    provider::{Provider, ProviderError},
};

use super::{http::HttpClient, magnet::magnet_info_hash};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

#[derive(Clone)]
pub struct RutorProvider {
    http: HttpClient,
}

impl RutorProvider {
    pub fn new(http: HttpClient) -> Self {
        Self { http }
    }

    fn search_url(&self, query: &str) -> String {
        let encoded = utf8_percent_encode(query, NON_ALPHANUMERIC);
        format!("https://rutor.info/search/{encoded}")
    }
}

impl Provider for RutorProvider {
    fn id(&self) -> &'static str {
        "rutor"
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<TorrentResult>, ProviderError> {
        let url = self.search_url(&query.q);
        let html = self.http.get(&url).await?;
        parse_search(&html)
    }
}

fn parse_search(html: &str) -> Result<Vec<TorrentResult>, ProviderError> {
    let doc = scraper::Html::parse_document(html);
    let err = |e| ProviderError::Unavailable(format!("selector error: {e:?}"));
    let row = scraper::Selector::parse("tr").map_err(err)?;
    let title = scraper::Selector::parse("a[href^='/torrent/']").map_err(err)?;
    let magnet = scraper::Selector::parse("a[href^='magnet:']").map_err(err)?;
    let size = scraper::Selector::parse("td[align='right']").map_err(err)?;
    let seed = scraper::Selector::parse("span.green").map_err(err)?;
    let leech = scraper::Selector::parse("span.red").map_err(err)?;

    let mut out = Vec::new();
    for tr in doc.select(&row) {
        let Some(magnet_el) = tr.select(&magnet).next() else {
            continue;
        };
        let Some(title_el) = tr.select(&title).next() else {
            continue;
        };

        let title_text = title_el.text().collect::<String>();
        let title_text = title_text.trim();
        if title_text.is_empty() {
            continue;
        }

        let magnet_href = magnet_el.value().attr("href");
        let topic = title_el.value().attr("href");
        let info_hash = magnet_href.and_then(magnet_info_hash);

        out.push(TorrentResult {
            id: topic
                .and_then(|t| t.strip_prefix("/torrent/"))
                .and_then(|t| t.split('/').next())
                .map(|id| format!("rutor-{id}"))
                .unwrap_or_else(|| format!("rutor-{title_text}")),
            title: title_text.to_string(),
            year: None,
            provider_id: "rutor".into(),
            size_bytes: tr
                .select(&size)
                .next()
                .and_then(|e| parse_size(&elem_text(&e))),
            seeders: tr
                .select(&seed)
                .next()
                .and_then(|e| parse_u32(&elem_text(&e))),
            leechers: tr
                .select(&leech)
                .next()
                .and_then(|e| parse_u32(&elem_text(&e))),
            info_hash,
            magnet_available: magnet_href.is_some(),
            quality: None,
            languages: vec![],
            source_url: topic.map(|t| format!("https://rutor.info{t}")),
        });
    }
    Ok(out)
}

fn elem_text(e: &scraper::ElementRef<'_>) -> String {
    e.text().collect()
}

fn parse_u32(s: &str) -> Option<u32> {
    let digits: String = s.chars().filter(char::is_ascii_digit).collect();
    digits.parse().ok()
}

fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let split = s
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != ',')
        .unwrap_or(s.len());
    let value: f64 = s[..split].replace(',', ".").parse().ok()?;
    let unit = s[split..].trim().to_ascii_uppercase();
    let mult = match unit.as_str() {
        "B" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024.0_f64.powi(2),
        "GB" => 1024.0_f64.powi(3),
        "TB" => 1024.0_f64.powi(4),
        _ => return None,
    };
    Some((value * mult) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("fixtures/rutor_search.html");

    #[test]
    fn parses_search_rows() {
        let results = parse_search(FIXTURE).unwrap();
        assert!(!results.is_empty());
        let first = &results[0];
        assert_eq!(first.provider_id, "rutor");
        assert!(first.magnet_available);
        assert_eq!(
            first.info_hash.as_deref(),
            Some("ae0cf422fa0bc9669ad605ea564999a8fbe5aa4b")
        );
        assert!(first.size_bytes.unwrap() > 8_000_000_000); // ~8.28 GB
        assert_eq!(first.seeders, Some(8));
        assert_eq!(first.leechers, Some(0));
        assert!(
            first
                .source_url
                .as_deref()
                .unwrap()
                .starts_with("https://rutor.info/torrent/")
        );
    }

    #[test]
    fn size_parsing() {
        assert_eq!(
            parse_size("8.28 GB"),
            Some((8.28f64 * 1024.0f64.powi(3)) as u64)
        );
        assert_eq!(
            parse_size("134.1 MB"),
            Some((134.1f64 * 1024.0f64.powi(2)) as u64)
        );
        assert_eq!(parse_size("garbage"), None);
    }

    #[test]
    fn returns_empty_on_no_rows() {
        assert!(
            parse_search("<html><body></body></html>")
                .unwrap()
                .is_empty()
        );
    }
}
