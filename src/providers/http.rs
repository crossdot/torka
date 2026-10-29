//! Shared HTTP client for providers: timeout, size cap, descriptive UA, and
//! SSRF-safe redirect validation.

use std::{net::IpAddr, time::Duration};

use reqwest::redirect::Policy;
use url::{Host, Url};

use crate::search::provider::ProviderError;

const DEFAULT_USER_AGENT: &str = concat!(
    "movie-streamer/",
    env!("CARGO_PKG_VERSION"),
    " (self-hosted media search; no scraping of protected resources)"
);

/// Maximum response body accepted from any provider.
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new(timeout: Duration) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent(DEFAULT_USER_AGENT)
            .redirect(Policy::custom(|attempt| {
                if is_safe_url(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()?;
        Ok(Self { client })
    }

    /// Fetch `url` into a UTF-8 string, enforcing [`MAX_RESPONSE_BYTES`].
    /// Redirects are SSRF-validated by the client's redirect policy; the initial
    /// URL is operator-configured and therefore trusted.
    pub async fn get(&self, url: &str) -> Result<String, ProviderError> {
        let mut resp = self.client.get(url).send().await.map_err(map_reqwest_err)?;

        let mut bytes = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(map_reqwest_err)? {
            if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err(ProviderError::Unavailable("response too large".into()));
            }
            bytes.extend_from_slice(&chunk);
        }

        String::from_utf8(bytes)
            .map_err(|_| ProviderError::Unavailable("response is not valid UTF-8".into()))
    }
}

fn map_reqwest_err(e: reqwest::Error) -> ProviderError {
    if e.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::Unavailable(format!("request failed: {e}"))
    }
}

/// Allow only http/https to a public, non-local host. Rejects loopback/private/
/// link-local IP literals and `localhost`-style hostnames. DNS rebinding of a
/// hostname to a private IP is a known limitation for the MVP (the two provider
/// hosts are fixed public services).
fn is_safe_url(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    match url.host() {
        Some(Host::Ipv4(ip)) => is_public_ip(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => is_public_ip(IpAddr::V6(ip)),
        Some(Host::Domain(host)) => {
            let h = host.to_ascii_lowercase();
            h != "localhost" && !h.ends_with(".localhost")
        }
        None => false,
    }
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_multicast()
                || v4.is_broadcast())
        }
        IpAddr::V6(v6) => {
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || v6.is_multicast())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_private_and_local_urls() {
        assert!(!is_safe_url(&Url::parse("http://127.0.0.1/").unwrap()));
        assert!(!is_safe_url(&Url::parse("http://localhost/").unwrap()));
        assert!(!is_safe_url(&Url::parse("http://10.0.0.1/").unwrap()));
        assert!(!is_safe_url(&Url::parse("http://192.168.1.1/").unwrap()));
        assert!(!is_safe_url(&Url::parse("file:///etc/passwd").unwrap()));
        assert!(!is_safe_url(&Url::parse("http://[::1]/").unwrap()));
    }

    #[test]
    fn allows_public_https() {
        assert!(is_safe_url(
            &Url::parse("https://feed.rutracker.cc/atom/f/0.atom").unwrap()
        ));
        assert!(is_safe_url(
            &Url::parse("https://rutor.info/search/x").unwrap()
        ));
    }
}
