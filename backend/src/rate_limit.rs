//! Per-route rate limiting helpers for the backend API.
//!
//! The server applies one of six tiers defined in [`crate::constants`] to each
//! route group:
//!
//! - `Light` — cheap, stateless endpoints such as `/fees`, `/prices`, and
//!   `/health`; these are the most permissive and are intended for polling and
//!   health checks.
//! - `Read` — public read-only endpoints such as `/pools`, `/stats`, and
//!   `/leaderboard`; these are cached and human-readable data views.
//! - `User` — per-user history and prediction endpoints that are scoped to a
//!   specific wallet (for example user prediction history and user profile
//!   reads); these are limited more tightly than general reads.
//! - `Write` — mutating or indexer-ingest routes under `/indexer/*`; these are
//!   the strictest user-facing write paths and protect backend ingestion from
//!   abusive bursts.
//! - `Token` — authentication endpoints such as `/auth/refresh`; stricter limits
//!   to prevent brute-force attacks on token operations.
//! - `WebSocket` — WebSocket message inbound limits; stricter to prevent abuse.
//!
//! Each route group in `routes/v1.rs` wraps its sub-router with the matching
//! tier from [`RateLimitTier`]. In test builds the layer is disabled so parallel
//! unit tests do not share token buckets or interfere with each other.
//!
//! Rate limiting is keyed by client IP address, with support for forwarded proxy
//! headers (`X-Forwarded-For`, `X-Real-IP`) when the backend sits behind a load
//! balancer or reverse proxy.
//!
//! ## HTTP 429 Responses with Retry-After Header
//!
//! When a client exceeds its configured burst window, the server replies with
//! HTTP 429 and includes a `Retry-After` header (in seconds) computed from the
//! active rate limit tier's window duration. This tells clients when they can
//! safely retry without encountering another 429. The response body is:
//!
//! ```json
//! {
//!   "status": "error",
//!   "error": "Too many requests"
//! }
//! ```
//!
//! The body comes from [`crate::response::rate_limit_error_response`], which
//! automatically attaches the `Retry-After` header. Clients should parse this
//! header and respect it to avoid further rate limit violations.
//!
//! **Example response:**
//! ```text
//! HTTP/1.1 429 Too Many Requests
//! Retry-After: 60
//! Content-Type: application/json
//!
//! {"status": "error", "error": "Too many requests"}
//! ```

use tower_governor::key_extractor::{PeerIpKeyExtractor, SmartIpKeyExtractor};

/// Rate-limit tier — maps to a `(burst_size, period_secs)` pair.
#[derive(Clone, Copy, Debug)]
pub enum RateLimitTier {
    /// Cheap stateless endpoints (`/fees`, `/prices`, `/health`). 120 req / 60 s.
    Light,
    /// Public read endpoints (`/pools`, `/stats`, `/leaderboard`). 60 req / 60 s.
    Read,
    /// Per-user history / prediction endpoints. 30 req / 60 s.
    User,
    /// Indexer ingest endpoints (`/indexer/*`). 20 req / 60 s.
    Write,
    /// Token operations (`/auth/refresh`). 10 req / 60 s (stricter to prevent brute force).
    Token,
    /// WebSocket inbound messages. 10 msg / 10 s per connection (stricter to prevent abuse).
    WebSocket,
}

impl RateLimitTier {
    fn burst_and_period(self) -> (u32, u64) {
        use crate::constants::*;
        match self {
            RateLimitTier::Light => (RATE_LIMIT_LIGHT_BURST, RATE_LIMIT_LIGHT_PERIOD_SECS),
            RateLimitTier::Read => (RATE_LIMIT_READ_BURST, RATE_LIMIT_READ_PERIOD_SECS),
            RateLimitTier::User => (RATE_LIMIT_USER_BURST, RATE_LIMIT_USER_PERIOD_SECS),
            RateLimitTier::Write => (RATE_LIMIT_WRITE_BURST, RATE_LIMIT_WRITE_PERIOD_SECS),
            RateLimitTier::Token => (RATE_LIMIT_TOKEN_BURST, RATE_LIMIT_TOKEN_PERIOD_SECS),
            RateLimitTier::WebSocket => (RATE_LIMIT_WS_BURST, RATE_LIMIT_WS_PERIOD_SECS),
        }
    }

    /// Return the window duration for this tier in seconds.
    /// Used to compute the `Retry-After` response header on 429.
    pub fn period_secs(self) -> u64 {
        self.burst_and_period().1
    }
}

/// Wrap `router` with a `GovernorLayer` configured for `tier`.
///
/// When `trust_proxy_headers` is false (the default), requests are keyed by the
/// TCP peer IP. Forwarded headers are used only when explicitly trusted via
/// `PREDIFI_TRUST_PROXY_HEADERS` (#2072).
///
/// In `#[cfg(test)]` builds this is a no-op — the router is returned as-is so
/// parallel unit tests do not rate-limit each other.
#[cfg(not(test))]
pub fn with_rate_limit(
    router: axum::Router,
    tier: RateLimitTier,
    trust_proxy_headers: bool,
) -> axum::Router {
    rate_limited_router(router, tier, trust_proxy_headers)
}

#[cfg(test)]
pub fn with_rate_limit(
    router: axum::Router,
    _tier: RateLimitTier,
    _trust_proxy_headers: bool,
) -> axum::Router {
    router
}

/// Build a rate-limited router; also available to tests that need to exercise
/// the real governor layer in isolation.
pub(crate) fn rate_limited_router(
    router: axum::Router,
    tier: RateLimitTier,
    trust_proxy_headers: bool,
) -> axum::Router {
    use std::sync::Arc;
    use tower_governor::governor::GovernorConfigBuilder;

    let (burst_size, period_secs) = tier.burst_and_period();
    let period = std::time::Duration::from_secs(period_secs);

    if trust_proxy_headers {
        let config = Arc::new(
            GovernorConfigBuilder::default()
                .period(period)
                .burst_size(burst_size)
                .key_extractor(SmartIpKeyExtractor)
                .error_handler(|_| crate::response::rate_limit_error_response())
                .finish()
                .expect("RateLimitTier burst and period constants must be non-zero"),
        );
        router.layer(tower_governor::GovernorLayer { config })
    } else {
        let config = Arc::new(
            GovernorConfigBuilder::default()
                .period(period)
                .burst_size(burst_size)
                .key_extractor(PeerIpKeyExtractor)
                .error_handler(|_| crate::response::rate_limit_error_response())
                .finish()
                .expect("RateLimitTier burst and period constants must be non-zero"),
        );
        router.layer(tower_governor::GovernorLayer { config })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_burst_and_period_values() {
        use crate::constants::*;

        let (b, p) = RateLimitTier::Light.burst_and_period();
        assert_eq!(
            (b, p),
            (RATE_LIMIT_LIGHT_BURST, RATE_LIMIT_LIGHT_PERIOD_SECS)
        );

        let (b, p) = RateLimitTier::Read.burst_and_period();
        assert_eq!((b, p), (RATE_LIMIT_READ_BURST, RATE_LIMIT_READ_PERIOD_SECS));

        let (b, p) = RateLimitTier::User.burst_and_period();
        assert_eq!((b, p), (RATE_LIMIT_USER_BURST, RATE_LIMIT_USER_PERIOD_SECS));

        let (b, p) = RateLimitTier::Write.burst_and_period();
        assert_eq!(
            (b, p),
            (RATE_LIMIT_WRITE_BURST, RATE_LIMIT_WRITE_PERIOD_SECS)
        );

        let (b, p) = RateLimitTier::Token.burst_and_period();
        assert_eq!(
            (b, p),
            (RATE_LIMIT_TOKEN_BURST, RATE_LIMIT_TOKEN_PERIOD_SECS)
        );

        let (b, p) = RateLimitTier::WebSocket.burst_and_period();
        assert_eq!((b, p), (RATE_LIMIT_WS_BURST, RATE_LIMIT_WS_PERIOD_SECS));
    }

    #[tokio::test]
    async fn rotating_forwarded_header_does_not_reset_bucket_when_proxy_headers_untrusted() {
        use axum::{extract::ConnectInfo, routing::get, Router};
        use tower::ServiceExt;

        let app = rate_limited_router(
            Router::new().route("/ping", get(|| async { "pong" })),
            RateLimitTier::Token,
            false, // PREDIFI_TRUST_PROXY_HEADERS is off
        );

        let mut statuses = Vec::new();
        for i in 0..=10 {
            let request = axum::http::Request::builder()
                .method("GET")
                .uri("/ping")
                .header("x-forwarded-for", format!("10.0.0.{i}"))
                // `oneshot` bypasses `into_make_service_with_connect_info`, so
                // simulate the socket peer address supplied by axum::serve.
                .extension(ConnectInfo(
                    "127.0.0.1:1234"
                        .parse::<std::net::SocketAddr>()
                        .expect("valid socket address"),
                ))
                .body(axum::body::Body::empty())
                .expect("request builds");
            statuses.push(app.clone().oneshot(request).await.expect("request").status());
        }

        assert!(
            statuses[..10]
                .iter()
                .all(|status| *status == axum::http::StatusCode::OK),
            "first ten requests should pass: {statuses:?}"
        );
        assert_eq!(
            statuses[10],
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "rotating X-Forwarded-For must not reset the peer-IP bucket: {statuses:?}"
        );
    }

    #[test]
    fn tiers_have_distinct_configs() {
        let configs = [
            RateLimitTier::Light.burst_and_period(),
            RateLimitTier::Read.burst_and_period(),
            RateLimitTier::User.burst_and_period(),
            RateLimitTier::Write.burst_and_period(),
            RateLimitTier::Token.burst_and_period(),
            RateLimitTier::WebSocket.burst_and_period(),
        ];
        // Each tier should be unique
        for i in 0..configs.len() {
            for j in (i + 1)..configs.len() {
                assert_ne!(
                    configs[i], configs[j],
                    "tiers {i} and {j} share the same config"
                );
            }
        }
    }
}
