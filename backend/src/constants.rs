//! Central constants for the PrediFi backend.
//!
//! All magic numbers and hardcoded values used across the application are
//! defined here so they can be referenced by name and changed in one place.

// ── Rate limiting ─────────────────────────────────────────────────────────────

/// Number of requests allowed per second per IP before rate limiting kicks in.
pub const RATE_LIMIT_PER_SECOND: u64 = 5;

/// Maximum burst size for the token-bucket rate limiter.
pub const RATE_LIMIT_BURST_SIZE: u32 = 50;

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
