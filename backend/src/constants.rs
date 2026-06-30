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
/// The rate limiter uses a **token-bucket algorithm**. Each successful request
/// consumes one token; tokens are replenished at a constant rate derived from
/// `period / burst`. Once the bucket is empty, subsequent requests are rejected
/// with `HTTP 429` until tokens are replenished.
pub const RATE_LIMIT_BURST_SIZE: u32 = 100;

/// Replenishment period for the token-bucket rate limiter (15 minutes in seconds).
///
/// Replenishment rate: `RATE_LIMIT_PERIOD_SECS / RATE_LIMIT_BURST_SIZE` = 1 token every 9 s.
pub const RATE_LIMIT_PERIOD_SECS: u64 = 900;

// ── Per-route rate limit tiers ────────────────────────────────────────────────

/// **Read tier** — public read endpoints (`/pools`, `/stats`, `/leaderboard`, etc.).
/// 60 requests / 60 s window (~1 req/s sustained, burst up to 60).
pub const RATE_LIMIT_READ_BURST: u32 = 60;
pub const RATE_LIMIT_READ_PERIOD_SECS: u64 = 60;

/// **Write tier** — indexer ingest endpoints (`/indexer/*`).
/// 20 requests / 60 s window (~1 req/3 s sustained, burst up to 20).
pub const RATE_LIMIT_WRITE_BURST: u32 = 20;
pub const RATE_LIMIT_WRITE_PERIOD_SECS: u64 = 60;

/// **User tier** — per-user history / predictions endpoints.
/// 30 requests / 60 s window — slightly more permissive than writes.
pub const RATE_LIMIT_USER_BURST: u32 = 30;
pub const RATE_LIMIT_USER_PERIOD_SECS: u64 = 60;

/// **Light tier** — cheap, stateless endpoints (`/fees`, `/prices`, `/health`).
/// 120 requests / 60 s window — generous for polling-friendly endpoints.
pub const RATE_LIMIT_LIGHT_BURST: u32 = 120;
pub const RATE_LIMIT_LIGHT_PERIOD_SECS: u64 = 60;

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

/// Minimum length (in bytes) required for the JWT signing secret.
///
/// HS256 requires a sufficiently long secret to resist brute-force attacks.
pub const JWT_SECRET_MIN_LENGTH: usize = 32;

/// Default maximum number of events processed per indexer poll cycle.
pub const DEFAULT_INDEXER_MAX_BATCH_SIZE: usize = 500;

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
