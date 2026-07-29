//! # Validated Request Types
//!
//! Newtype wrappers that enforce domain invariants at deserialization time.
//! Any payload or query parameter using these types is validated before the
//! handler runs — invalid input returns HTTP 400 automatically via Axum's
//! rejection handling.

use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// Error returned when a validated type fails its invariant check.
#[derive(Debug, PartialEq)]
pub struct ValidationError(pub String);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ── NonEmptyString ────────────────────────────────────────────────────────────

/// A `String` that must contain at least one non-whitespace character.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        if s.trim().is_empty() {
            Err(ValidationError("must not be empty".to_string()))
        } else {
            Ok(Self(s))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NonEmptyString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<NonEmptyString> for String {
    fn from(v: NonEmptyString) -> Self {
        v.0
    }
}

impl<'de> Deserialize<'de> for NonEmptyString {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        NonEmptyString::new(s).map_err(serde::de::Error::custom)
    }
}

// ── BoundedI64 ────────────────────────────────────────────────────────────────

/// An `i64` clamped to `[min, max]` at deserialization time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BoundedI64<const MIN: i64, const MAX: i64>(i64);

impl<const MIN: i64, const MAX: i64> BoundedI64<MIN, MAX> {
    pub fn new(n: i64) -> Result<Self, ValidationError> {
        if n < MIN || n > MAX {
            Err(ValidationError(format!("must be between {MIN} and {MAX}")))
        } else {
            Ok(Self(n))
        }
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

impl<'de, const MIN: i64, const MAX: i64> Deserialize<'de> for BoundedI64<MIN, MAX> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let n = i64::deserialize(de)?;
        BoundedI64::<MIN, MAX>::new(n).map_err(serde::de::Error::custom)
    }
}

// ── StellarAddress ────────────────────────────────────────────────────────────

/// A Stellar account address (G… or C… prefix, 56 chars, base32 alphanumeric).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StellarAddress(String);

impl StellarAddress {
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        let valid = (s.starts_with('G') || s.starts_with('C'))
            && s.len() == 56
            && s.chars().all(|c| c.is_ascii_alphanumeric());
        if valid {
            Ok(Self(s))
        } else {
            Err(ValidationError(
                "must be a valid Stellar address (G/C prefix, 56 chars)".to_string(),
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StellarAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<StellarAddress> for String {
    fn from(v: StellarAddress) -> Self {
        v.0
    }
}

impl<'de> Deserialize<'de> for StellarAddress {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        StellarAddress::new(s).map_err(serde::de::Error::custom)
    }
}

// ── PoolSortBy ────────────────────────────────────────────────────────────────

/// Allowed sort values for the pools listing endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolSortBy {
    Popular,
    EndingSoon,
    New,
}

impl PoolSortBy {
    pub fn as_str(self) -> &'static str {
        match self {
            PoolSortBy::Popular => "popular",
            PoolSortBy::EndingSoon => "ending_soon",
            PoolSortBy::New => "new",
        }
    }
}

impl<'de> Deserialize<'de> for PoolSortBy {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        match s.as_str() {
            "popular" => Ok(PoolSortBy::Popular),
            "ending_soon" => Ok(PoolSortBy::EndingSoon),
            "new" => Ok(PoolSortBy::New),
            other => Err(serde::de::Error::custom(format!(
                "invalid sort_by value '{other}': must be one of popular, ending_soon, new"
            ))),
        }
    }
}

// ── PoolStatus ────────────────────────────────────────────────────────────────

/// Allowed status values for the pools listing / stats endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolStatus {
    Active,
    Closed,
    Settled,
}

impl PoolStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PoolStatus::Active => "active",
            PoolStatus::Closed => "closed",
            PoolStatus::Settled => "settled",
        }
    }
}

impl<'de> Deserialize<'de> for PoolStatus {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        match s.as_str() {
            "active" => Ok(PoolStatus::Active),
            "closed" => Ok(PoolStatus::Closed),
            "settled" => Ok(PoolStatus::Settled),
            other => Err(serde::de::Error::custom(format!(
                "invalid status '{other}': must be one of active, closed, settled"
            ))),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_empty_string_rejects_blank() {
        assert!(NonEmptyString::new("").is_err());
        assert!(NonEmptyString::new("   ").is_err());
    }

    #[test]
    fn non_empty_string_accepts_valid() {
        let s = NonEmptyString::new("hello").unwrap();
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn non_empty_string_deserialize_rejects_blank() {
        let result: Result<NonEmptyString, _> = serde_json::from_str("\"\"");
        assert!(result.is_err());
    }

    #[test]
    fn bounded_i64_rejects_out_of_range() {
        assert!(BoundedI64::<1, 100>::new(0).is_err());
        assert!(BoundedI64::<1, 100>::new(101).is_err());
    }

    #[test]
    fn bounded_i64_accepts_in_range() {
        assert_eq!(BoundedI64::<1, 100>::new(50).unwrap().get(), 50);
        assert_eq!(BoundedI64::<1, 100>::new(1).unwrap().get(), 1);
        assert_eq!(BoundedI64::<1, 100>::new(100).unwrap().get(), 100);
    }

    #[test]
    fn bounded_i64_deserialize_rejects_out_of_range() {
        let result: Result<BoundedI64<1, 100>, _> = serde_json::from_str("0");
        assert!(result.is_err());
    }

    #[test]
    fn stellar_address_rejects_invalid() {
        assert!(StellarAddress::new("").is_err());
        assert!(StellarAddress::new("GABC").is_err()); // too short
        assert!(StellarAddress::new("X".repeat(56)).is_err()); // wrong prefix
    }

    #[test]
    fn stellar_address_accepts_valid_g_address() {
        // 56-char G-prefixed base32 address
        let addr = format!("G{}", "A".repeat(55));
        assert!(StellarAddress::new(addr).is_ok());
    }

    #[test]
    fn stellar_address_accepts_valid_c_address() {
        let addr = format!("C{}", "A".repeat(55));
        assert!(StellarAddress::new(addr).is_ok());
    }

    #[test]
    fn pool_sort_by_rejects_invalid() {
        let result: Result<PoolSortBy, _> = serde_json::from_str("\"invalid\"");
        assert!(result.is_err());
    }

    #[test]
    fn pool_sort_by_accepts_valid() {
        let v: PoolSortBy = serde_json::from_str("\"popular\"").unwrap();
        assert_eq!(v, PoolSortBy::Popular);
        let v: PoolSortBy = serde_json::from_str("\"ending_soon\"").unwrap();
        assert_eq!(v, PoolSortBy::EndingSoon);
        let v: PoolSortBy = serde_json::from_str("\"new\"").unwrap();
        assert_eq!(v, PoolSortBy::New);
    }

    #[test]
    fn pool_status_rejects_invalid() {
        let result: Result<PoolStatus, _> = serde_json::from_str("\"pending\"");
        assert!(result.is_err());
    }

    #[test]
    fn pool_status_accepts_valid() {
        let v: PoolStatus = serde_json::from_str("\"active\"").unwrap();
        assert_eq!(v, PoolStatus::Active);
        let v: PoolStatus = serde_json::from_str("\"closed\"").unwrap();
        assert_eq!(v, PoolStatus::Closed);
        let v: PoolStatus = serde_json::from_str("\"settled\"").unwrap();
        assert_eq!(v, PoolStatus::Settled);
    }
}
