//! Per-IP sliding-window rate limiter + Axum middleware.

use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::error::AppError;

/// Fixed-capacity sliding-window limiter keyed by client IP.
pub struct RateLimiter {
    limit: usize,
    window: Duration,
    entries: Mutex<HashMap<IpAddr, VecDeque<Instant>>>,
}

impl RateLimiter {
    pub fn new(limit: usize, window: Duration) -> Self {
        Self {
            limit,
            window,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if the request is allowed (and records it), false if the
    /// IP has already made `limit` requests within the window. `limit == 0`
    /// disables limiting (always allowed).
    pub fn check(&self, ip: IpAddr) -> bool {
        if self.limit == 0 {
            return true;
        }
        let now = Instant::now();
        let mut entries = self.entries.lock().unwrap();
        let q = entries.entry(ip).or_default();
        while q
            .front()
            .is_some_and(|t| now.duration_since(*t) >= self.window)
        {
            q.pop_front();
        }
        if q.len() >= self.limit {
            return false;
        }
        q.push_back(now);
        true
    }
}

/// Middleware: reject requests over the configured rate limit with 429.
/// No connect info (e.g. in tests / non-TCP transports) → skip limiting.
pub async fn rate_limit(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip());
    match ip {
        Some(ip) if !limiter.check(ip) => {
            AppError::rate_limited("rate limit exceeded").into_response()
        }
        _ => next.run(request).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_requests_per_ip() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip = "10.0.0.1".parse().unwrap();
        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
        assert!(!limiter.check(ip));
    }

    #[test]
    fn window_expiry_allows_again() {
        let limiter = RateLimiter::new(1, Duration::from_millis(10));
        let ip = "10.0.0.2".parse().unwrap();
        assert!(limiter.check(ip));
        assert!(!limiter.check(ip));
        std::thread::sleep(Duration::from_millis(20));
        assert!(limiter.check(ip));
    }

    #[test]
    fn zero_limit_disables() {
        let limiter = RateLimiter::new(0, Duration::from_secs(60));
        let ip = "10.0.0.3".parse().unwrap();
        for _ in 0..100 {
            assert!(limiter.check(ip));
        }
    }

    #[tokio::test]
    async fn middleware_returns_429_when_over_limit() {
        use axum::{Router, body::Body, http::Request, http::StatusCode, middleware, routing::get};
        use tower::ServiceExt;

        let limiter = Arc::new(RateLimiter::new(1, Duration::from_secs(60)));
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(limiter, rate_limit));

        let mk = |ip: [u8; 4]| {
            let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
            req.extensions_mut()
                .insert(ConnectInfo(SocketAddr::from((ip, 8080))));
            req
        };

        let resp = app.clone().oneshot(mk([10, 0, 0, 1])).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let resp = app.clone().oneshot(mk([10, 0, 0, 1])).await.unwrap();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        // A different IP is unaffected.
        let resp = app.oneshot(mk([10, 0, 0, 2])).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
