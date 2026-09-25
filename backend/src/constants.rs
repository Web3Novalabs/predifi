//! Central constants for the PrediFi backend.
//!
//! All magic numbers and hardcoded values used across the application are
//! defined here so they can be referenced by name and changed in one place.
//!
//! The constants are grouped by concern:
//! - **Rate limiting**: the global IP-based token-bucket defaults.
//! - **Per-route rate-limit tiers**: `(burst, period)` pairs applied per
//!   route group via [`crate::rate_limit::RateLimitTier`].
//! - **Pagination**: defaults and caps for list endpoints.
//! - **JWT validation**: structural checks, secret constraints, and token
//!   lifetimes.
//! - **Indexer batching** and **graceful shutdown** settings.

// ── Rate limiting ─────────────────────────────────────────────────────────────

/// Maximum burst size for the IP-based rate limiter.
///
/// **Units:** Number of requests (count).
/// **Value:** 100 requests.
///
/// This is the maximum number of requests a single IP address can make within
/// the replenishment window (see [`RATE_LIMIT_PERIOD_SECS`]) before receiving a
/// **429 Too Many Requests** response.
///
/// The rate limiter uses a **token-bucket algorithm**. Each successful request
/// consumes one token; tokens are replenished at a constant rate derived from
/// `period / burst`. Once the bucket is empty, subsequent requests are rejected
/// with `HTTP 429` until tokens are replenished.
///
/// **Impact of changes:**
/// - Increasing this value raises the burst a client can send in a short window,
///   reducing false positives for legitimate traffic (proxies/NATs sharing one
///   IP) but also increasing the request pressure a single abusive IP can exert.
/// - Decreasing this value tightens the ceiling, protecting the server but
///   increasing the risk of 429s for well-behaved clients behind shared IPs.
/// - Route-specific tiers (see [`crate::rate_limit::RateLimitTier`]) use their
///   own burst/period pairs and take precedence over this default.
///
/// **Used for:** Default burst size for the IP-based rate limiter, passed to
/// [`crate::server::build_router`].
pub const RATE_LIMIT_BURST_SIZE: u32 = 100;

/// Replenishment period for the token-bucket rate limiter (15 minutes in seconds).
///
/// **Units:** Seconds.
/// **Value:** 900 seconds (15 minutes).
///
/// Replenishment rate: `RATE_LIMIT_PERIOD_SECS / RATE_LIMIT_BURST_SIZE` = 1 token every 9 s.
///
/// **Impact of changes:**
/// - Increasing this value makes tokens replenish more slowly, lowering sustained
///   request throughput and making abuse protection stricter.
/// - Decreasing this value refills the bucket faster, allowing higher sustained
///   request rates while keeping the same burst ceiling enforced by
///   [`RATE_LIMIT_BURST_SIZE`].
///
/// **Used for:** Default replenishment window for the IP-based rate limiter.
pub const RATE_LIMIT_PERIOD_SECS: u64 = 900;

// ── Per-route rate limit tiers ────────────────────────────────────────────────

/// **Read tier** — public read endpoints (`/pools`, `/stats`, `/leaderboard`, etc.).
/// 60 requests / 60 s window (~1 req/s sustained, burst up to 60).
///
/// **Units:** Burst in requests, period in seconds.
///
/// **Rationale:** Public read endpoints are the most heavily trafficked surface
/// of the API. A 60 req/60 s ceiling per IP keeps them responsive while still
/// allowing crawlers and aggregators to poll at 1 req/s.
///
/// **Impact of changes:**
/// - Increasing the burst or period lengthens the window, reducing 429s for
///   legitimate readers but admitting more abusive polling per IP.
/// - Decreasing tightens protection at the risk of throttling normal clients.
///
/// **Used for:** [`crate::rate_limit::RateLimitTier::Read`].
pub const RATE_LIMIT_READ_BURST: u32 = 60;
pub const RATE_LIMIT_READ_PERIOD_SECS: u64 = 60;

/// **Write tier** — indexer ingest endpoints (`/indexer/*`).
/// 20 requests / 60 s window (~1 req/3 s sustained, burst up to 20).
///
/// **Units:** Burst in requests, period in seconds.
///
/// **Rationale:** Ingest endpoints change state and are typically driven by
/// internal indexer workers rather than end users, so they need less headroom
/// than public reads. The 20 req/60 s ceiling still comfortably supports
/// batching (see [`DEFAULT_INDEXER_MAX_BATCH_SIZE`]) while keeping write load
/// predictable.
///
/// **Impact of changes:**
/// - Increasing accommodates larger or more frequent ingest batches but allows
///   more write pressure per IP.
/// - Decreasing throttles the indexer, which can backfill ingestion and cause
///   stale data if set too low.
///
/// **Used for:** [`crate::rate_limit::RateLimitTier::Write`].
pub const RATE_LIMIT_WRITE_BURST: u32 = 20;
pub const RATE_LIMIT_WRITE_PERIOD_SECS: u64 = 60;

/// **User tier** — per-user history / predictions endpoints.
/// 30 requests / 60 s window — slightly more permissive than writes.
///
/// **Units:** Burst in requests, period in seconds.
///
/// **Rationale:** Per-user history and prediction endpoints are hit by frontend
/// screens that often query several endpoints at once (wallet, predictions,
/// stats). 30 req/60 s gives a single user enough headroom for those parallel
/// calls without letting one IP sweep large amounts of user data quickly.
///
/// **Impact of changes:**
/// - Raising the ceiling reduces 429s during bursty UI navigation but increases
///   the data-sweeping rate available to a single IP.
/// - Lowering it narrows the per-user rate and may frustrate legitimate rapid
///   navigation.
///
/// **Used for:** [`crate::rate_limit::RateLimitTier::User`].
pub const RATE_LIMIT_USER_BURST: u32 = 30;
pub const RATE_LIMIT_USER_PERIOD_SECS: u64 = 60;

/// **Light tier** — cheap, stateless endpoints (`/fees`, `/prices`, `/health`).
/// 120 requests / 60 s window — generous for polling-friendly endpoints.
///
/// **Units:** Burst in requests, period in seconds.
///
/// **Rationale:** These endpoints perform minimal work (no database joins) and
/// are frequently polled by clients for prices and health checks. A high ceiling
/// keeps monitors, uptime probes, and price watchers happy without meaningful
/// load on the server.
///
/// **Impact of changes:**
/// - Increasing allows more aggressive polling and probing without 429s.
/// - Decreasing can throttle uptime monitors or price watchers, causing
///   spurious alerts.
///
/// **Used for:** [`crate::rate_limit::RateLimitTier::Light`].
pub const RATE_LIMIT_LIGHT_BURST: u32 = 120;
pub const RATE_LIMIT_LIGHT_PERIOD_SECS: u64 = 60;

// ── Pagination ────────────────────────────────────────────────────────────────

/// Default number of items returned per page when no `limit` is supplied.
///
/// **Units:** Items (count).
/// **Value:** 20 items.
///
/// **Rationale:** A moderate default keeps responses small enough to be fast and
/// cheap to serialize while returning enough rows to be useful in a single page.
///
/// **Impact of changes:**
/// - Increasing produces larger default payloads (higher bandwidth, latency, and
///   database work per request).
/// - Decreasing lightens each response but forces clients to make more requests
///   to page through large result sets.
///
/// **Used for:** Paginating list responses when the client omits `limit`.
pub const DEFAULT_PAGE_LIMIT: i64 = 20;

/// Hard cap on the number of items that can be requested in a single page.
///
/// **Units:** Items (count).
/// **Value:** 100 items.
///
/// **Rationale:** Clients can request any `limit <= 100`, but the cap prevents a
/// single request from triggering an unbounded database query or an overly large
/// response body, bounding memory, CPU, and bandwidth per request.
///
/// **Impact of changes:**
/// - Raising the cap lets clients fetch more rows per page but increases worst-case
///   per-request resource usage and response size.
/// - Lowering the cap hard-limits page size further, at the cost of more requests
///   for large result sets.
///
/// **Used for:** Clamping the `limit` query parameter on list endpoints.
pub const MAX_PAGE_LIMIT: i64 = 100;

// ── JWT validation ────────────────────────────────────────────────────────────

/// Number of dot-separated parts a well-formed JWT must have (header.payload.signature).
///
/// **Units:** Parts (count).
/// **Value:** 3 parts.
///
/// **Rationale:** Every compact-serialized JWT is exactly three base64url
/// segments joined by dots. Checking the part count is the cheapest possible
/// structural validation and rejects malformed tokens before any decoding.
///
/// **Impact of changes:**
/// - This is a property of the JWT format and must remain 3. Changing it would
///   either reject every valid token (if increased) or accept malformed tokens
///   (if decreased).
///
/// **Used for:** Pre-decode validation in [`crate::jwt`].
pub const JWT_PARTS_COUNT: usize = 3;

/// Minimum length (in bytes) of a plausible JWT string.
///
/// **Units:** Bytes.
/// **Value:** 20 bytes.
///
/// **Rationale:** A real JWT is at least three non-empty base64url segments separated by two
/// dots. Anything shorter is trivially invalid and can be rejected cheaply
/// before attempting base64 decoding.
///
/// **Impact of changes:**
/// - Raising the minimum rejects a few more malformed tokens at parse time but
///   risks rejecting unusually short (but valid) hand-crafted tokens — JWT
///   length is not otherwise bound by the specification.
/// - Lowering the minimum weakens the cheap pre-flight check.
///
/// **Used for:** Pre-decode validation in [`crate::jwt`].
pub const JWT_MIN_LENGTH: usize = 20;

/// Minimum length (in bytes) required for the JWT signing secret.
///
/// **Units:** Bytes.
/// **Value:** 32 bytes.
///
/// **Rationale:** HS256 requires a sufficiently long secret to resist brute-force attacks;
/// 32 bytes (256 bits) matches the HS256 key size and provides 256 bits of
/// entropy, the recommended minimum for HMAC-SHA256.
///
/// **Impact of changes:**
/// - Requiring a longer secret raises the bar against brute-force key recovery
///   but makes configuration more demanding for operators.
/// - A shorter minimum risk weak keys that can be brute-forced offline.
///
/// **Used for:** Validating the configured [`crate::config::Config`] signing
/// secret during startup.
pub const JWT_SECRET_MIN_LENGTH: usize = 32;

/// Access token lifetime in seconds (1 hour).
///
/// **Units:** Seconds.
/// **Value:** 3,600 seconds (1 hour).
///
/// **Rationale:** Short-lived access tokens limit the window in which a stolen
/// token can be replayed, which is the standard OAuth/JWT trade-off: the shorter
/// the lifetime, the smaller the exposure, but the more often clients must
/// refresh.
///
/// **Impact of changes:**
/// - Reducing the lifetime shrinks the stolen-token window but increases refresh
///   traffic and may interrupt long-lived client sessions.
/// - Increasing it reduces refresh overhead at the cost of a longer
///   compromise window.
///
/// **Used for:** Setting the `exp` claim on access tokens in [`crate::jwt`].
pub const JWT_ACCESS_TOKEN_EXPIRY_SECS: u64 = 3_600;

/// Refresh token lifetime in seconds (7 days).
///
/// **Units:** Seconds.
/// **Value:** 604,800 seconds (7 days).
///
/// **Rationale:** Refresh tokens are long-lived by design so users are not forced
/// to re-authenticate frequently; 7 days balances persistent sessions against the
/// risk of a stolen refresh token remaining usable indefinitely. The token-refresh
/// endpoint is rate-limited independently (see [`RATE_LIMIT_TOKEN_BURST`]).
///
/// **Impact of changes:**
/// - Shorter lifetimes force more frequent re-authentication and reduce the value
///   of a stolen refresh token.
/// - Longer lifetimes increase convenience but widen the abuse window for stolen
///   or leaked refresh tokens.
///
/// **Used for:** Setting the `exp` claim on refresh tokens in [`crate::jwt`].
pub const JWT_REFRESH_TOKEN_EXPIRY_SECS: u64 = 7 * 24 * 3_600;

/// Rate limit for token refresh endpoint: 10 requests per 60 seconds per IP.
/// Prevents brute-force attacks on refresh token rotation.
///
/// **Units:** Burst in requests, period in seconds.
/// **Value:** 10 requests / 60 seconds.
///
/// **Impact of changes:**
/// - Increasing the ceiling lowers friction for legitimate token rotation during
///   bursts but weakens the brute-force/abuse protection on an auth-critical path.
/// - Decreasing it hardens the endpoint but may 429 legitimate clients that
///   rotate several sessions at once (e.g. multiple devices behind one IP).
///
/// **Used for:** [`crate::rate_limit::RateLimitTier::Token`].
pub const RATE_LIMIT_TOKEN_BURST: u32 = 10;
pub const RATE_LIMIT_TOKEN_PERIOD_SECS: u64 = 60;

/// **WebSocket tier** — inbound messages per connection.
/// 10 messages / 10 s window (~1 msg/s sustained, burst up to 10).
/// Prevents a single WS client from flooding the server with messages.
///
/// **Units:** Burst in messages, period in seconds.
/// **Value:** 10 messages / 10 seconds per connection.
///
/// **Rationale:** Unlike the HTTP tiers, the WebSocket limit is applied per
/// connection (not per IP) so one flooded socket cannot monopolize the event bus,
/// while other connections remain unaffected.
///
/// **Impact of changes:**
/// - Raising the limit accommodates chatty clients that legitimately send bursts
///   but increases the fan-out pressure one connection can produce.
/// - Lowering it further throttles clients that send many small messages quickly.
///
/// **Used for:** Per-connection inbound message rate limiting in [`crate::ws`].
pub const RATE_LIMIT_WS_BURST: u32 = 10;
pub const RATE_LIMIT_WS_PERIOD_SECS: u64 = 10;

/// Default maximum number of ledger events processed per indexer batch.
///
/// **Units:** Events (count).
/// **Value:** 500 events.
///
/// **Rationale:** Processing ledger events in bounded batches keeps each ingest
/// iteration memory- and time-predictable, caps per-transaction DB work, and
/// allows the indexer to checkpoint progress frequently. 500 events is large
/// enough to stay efficient under normal ledger load while small enough to bound
/// any single batch's failure domain.
///
/// **Impact of changes:**
/// - Increasing the batch size reduces the number of round-trips and checkpoints
///   but increases per-batch memory usage and the amount of work that must be
///   retried if a batch fails.
/// - Decreasing it lowers per-batch resource usage at the cost of more frequent,
///   smaller processing loops.
/// - Operators can override the default via the `indexer_max_batch_size` setting
///   in [`crate::config::Config`].
///
/// **Used for:** Sizing the indexer listener batch in [`crate::server`].
pub const DEFAULT_INDEXER_MAX_BATCH_SIZE: usize = 500;

// ── Graceful shutdown ─────────────────────────────────────────────────────────

/// Maximum number of seconds the HTTP server is allowed to spend draining
/// in-flight requests after a shutdown signal has been received.
///
/// **Units:** Seconds.
/// **Value:** 30 seconds.
///
/// Once this interval elapses with requests still pending, the server stops
/// accepting new connections immediately, aborts the remaining handlers, and
/// proceeds to close the database pool and background workers so the process
/// can exit without leaking connections.
///
/// 30 s matches the default `terminationGracePeriodSeconds` for Kubernetes
/// pods and gives long-tail requests (e.g. external RPCs) room to finish.
///
/// **Impact of changes:**
/// - Increasing the timeout gives long-running requests more time to complete
///   cleanly but delays pod termination, which can stall rolling deploys.
/// - Decreasing it speeds up shutdown at the risk of cutting off legitimate
///   in-flight requests and aborting their handlers.
///
/// **Used for:** Configuring the graceful shutdown drain window on SIGTERM/SIGINT.
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 30;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_burst_constant_is_greater_than_zero() {
        assert!(RATE_LIMIT_BURST_SIZE > 0);
        assert!(RATE_LIMIT_READ_BURST > 0);
        assert!(RATE_LIMIT_WRITE_BURST > 0);
        assert!(RATE_LIMIT_USER_BURST > 0);
        assert!(RATE_LIMIT_LIGHT_BURST > 0);
        assert!(RATE_LIMIT_TOKEN_BURST > 0);
        assert!(RATE_LIMIT_WS_BURST > 0);
    }

    #[test]
    fn every_period_secs_constant_is_greater_than_zero() {
        assert!(RATE_LIMIT_PERIOD_SECS > 0);
        assert!(RATE_LIMIT_READ_PERIOD_SECS > 0);
        assert!(RATE_LIMIT_WRITE_PERIOD_SECS > 0);
        assert!(RATE_LIMIT_USER_PERIOD_SECS > 0);
        assert!(RATE_LIMIT_LIGHT_PERIOD_SECS > 0);
        assert!(RATE_LIMIT_TOKEN_PERIOD_SECS > 0);
        assert!(RATE_LIMIT_WS_PERIOD_SECS > 0);
    }

    #[test]
    fn default_page_limit_does_not_exceed_max_page_limit() {
        assert!(DEFAULT_PAGE_LIMIT <= MAX_PAGE_LIMIT);
    }

    #[test]
    fn jwt_parts_count_is_three() {
        assert_eq!(JWT_PARTS_COUNT, 3);
    }
}
