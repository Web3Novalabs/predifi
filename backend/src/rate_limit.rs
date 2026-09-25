//! Per-route rate limiting helpers for the backend API.
//!
//! The server applies one of four tiers defined in [`crate::constants`] to each
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
//!
//! Each route group in `routes/v1.rs` wraps its sub-router with the matching
//! tier from [`RateLimitTier`]. In test builds the layer is disabled so parallel
//! unit tests do not share token buckets or interfere with each other.
//!
//! Rate limiting is keyed by client IP address, with support for forwarded proxy
//! headers (`X-Forwarded-For`, `X-Real-IP`) when the backend sits behind a load
//! balancer or reverse proxy.
//!
//! When a client exceeds its configured burst window, the server replies with
//! HTTP 429 and the standardized JSON body returned by [`crate::response`]:
//!
//! ```json
//! {
//!   "status": "error",
//!   "error": "Too many requests"
//! }
//! ```
//! 
//! This body comes from [`crate::response::rate_limit_error_response`].

use tower_governor::key_extractor::SmartIpKeyExtractor;

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
}

/// Wrap `router` with a `GovernorLayer` configured for `tier`.
///
/// In `#[cfg(test)]` builds this is a no-op — the router is returned as-is so
/// parallel unit tests do not rate-limit each other.
#[cfg(not(test))]
pub fn with_rate_limit(router: axum::Router, tier: RateLimitTier) -> axum::Router {
    use std::sync::Arc;
    use tower_governor::governor::GovernorConfigBuilder;

    let (burst_size, period_secs) = tier.burst_and_period();

    let config = Arc::new(
        GovernorConfigBuilder::default()
            .period(std::time::Duration::from_secs(period_secs))
            .burst_size(burst_size)
            .key_extractor(SmartIpKeyExtractor)
            .error_handler(|_| crate::response::rate_limit_error_response())
            .finish()
            .expect("invalid governor config"),
    );

    router.layer(tower_governor::GovernorLayer { config })
}

#[cfg(test)]
pub fn with_rate_limit(router: axum::Router, _tier: RateLimitTier) -> axum::Router {
    router
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
