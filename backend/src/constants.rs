//! Central constants for the PrediFi backend.
//!
//! All magic numbers and hardcoded values used across the application are
//! defined here so they can be referenced by name and changed in one place.

// ── Rate limiting ─────────────────────────────────────────────────────────────

/// Maximum burst size for the IP-based rate limiter.
///
/// This is the number of requests a single IP address can make within the
/// replenishment window (see [`RATE_LIMIT_PERIOD_SECS`]) before receiving a
/// **429 Too Many Requests** response.
///
/// ## Configuration
///
/// - **Limit:** 100 requests
/// - **Window:** 15 minutes (900 seconds)
/// - **Rate:** Approximately 1 token replenished every 9 seconds
///
/// The rate limiter uses a **token-bucket algorithm**. Each successful request
/// consumes one token; tokens are replenished at a constant rate derived from
/// `period / burst`. Once the bucket is empty, subsequent requests are rejected
/// with `HTTP 429` until tokens are replenished.
///
/// ## Use Case
///
/// This conservative default protects public endpoints from brute-force attacks,
/// credential stuffing, DDoS attempts, and accidental API abuse, without
/// significantly impacting legitimate users.
pub const RATE_LIMIT_BURST_SIZE: u32 = 100;

/// Replenishment period for the token-bucket rate limiter (15 minutes in seconds).
///
/// This defines the time window over which [`RATE_LIMIT_BURST_SIZE`] tokens are
/// made available. The replenishment rate is calculated as:
///
/// ```text
/// tokens_per_second = RATE_LIMIT_PERIOD_SECS / RATE_LIMIT_BURST_SIZE
///                   = 900 / 100
///                   = 1 token every 9 seconds
/// ```
///
/// ## Rationale
///
/// A 15-minute window strikes a balance between:
/// - **Allowing legitimate bursts** (e.g., a user rapidly navigating the UI)
/// - **Preventing sustained abuse** (e.g., scrapers or DDoS traffic)
///
/// The window is long enough to prevent false positives from legitimate users
/// while short enough to quickly mitigate attacks.
pub const RATE_LIMIT_PERIOD_SECS: u64 = 900;

// ── Pagination ────────────────────────────────────────────────────────────────

/// Default number of items returned per page when no `limit` is supplied.
pub const DEFAULT_PAGE_LIMIT: i64 = 20;

/// Hard cap on the number of items that can be requested in a single page.
pub const MAX_PAGE_LIMIT: i64 = 100;

// ── JWT validation ────────────────────────────────────────────────────────────

/// Number of dot-separated parts a well-formed JWT must have (header.payload.signature).
pub const JWT_PARTS_COUNT: usize = 3;

/// Minimum length (in bytes) of a plausible JWT string.
///
/// A real JWT is at least three non-empty base64url segments separated by two
/// dots.  Anything shorter is trivially invalid and can be rejected cheaply
/// before attempting base64 decoding.
pub const JWT_MIN_LENGTH: usize = 20;

// ── Graceful shutdown ─────────────────────────────────────────────────────────

/// Maximum number of seconds the HTTP server is allowed to spend draining
/// in-flight requests after a shutdown signal has been received.
///
/// Once this interval elapses with requests still pending, the server stops
/// accepting new connections immediately, aborts the remaining handlers, and
/// proceeds to close the database pool and background workers so the process
/// can exit without leaking connections.
///
/// 30 s matches the default `terminationGracePeriodSeconds` for Kubernetes
/// pods and gives long-tail requests (e.g. external RPCs) room to finish.
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 30;
