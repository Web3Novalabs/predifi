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

/// RFC4648 base32 alphabet (no padding), as used by Stellar's strkey encoding.
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Strkey version byte for an ED25519 account ID (`G…` address).
///
/// `pub(crate)` so other test modules (e.g. `tests.rs`) can build
/// checksum-valid fixture addresses to exercise routes using
/// `Path<StellarAddress>`, without duplicating the strkey encoding logic.
pub(crate) const STRKEY_VERSION_ACCOUNT_ID: u8 = 6 << 3;
/// Strkey version byte for a contract address (`C…` address).
const STRKEY_VERSION_CONTRACT: u8 = 2 << 3;

/// Decode an unpadded RFC4648 base32 string into raw bytes.
///
/// Returns `None` if any character falls outside [`BASE32_ALPHABET`], or if
/// the leftover bits after the last full byte are non-zero (i.e. the input
/// encodes a fractional, invalid byte at the end).
fn base32_decode(input: &str) -> Option<Vec<u8>> {
    let mut bits: u32 = 0;
    let mut bit_count: u32 = 0;
    let mut out = Vec::with_capacity(input.len() * 5 / 8);
    for c in input.bytes() {
        let value = BASE32_ALPHABET.iter().position(|&b| b == c)? as u32;
        bits = (bits << 5) | value;
        bit_count += 5;
        if bit_count >= 8 {
            bit_count -= 8;
            out.push(((bits >> bit_count) & 0xFF) as u8);
        }
    }
    if bit_count > 0 && (bits & ((1 << bit_count) - 1)) != 0 {
        return None;
    }
    Some(out)
}

/// Encode raw bytes as an unpadded RFC4648 base32 string (used only by tests
/// to build known-valid strkey fixtures — production code only decodes).
#[cfg(test)]
pub(crate) fn base32_encode(data: &[u8]) -> String {
    let mut bits: u32 = 0;
    let mut bit_count: u32 = 0;
    let mut out = String::with_capacity((data.len() * 8 + 4) / 5);
    for &byte in data {
        bits = (bits << 8) | byte as u32;
        bit_count += 8;
        while bit_count >= 5 {
            bit_count -= 5;
            out.push(BASE32_ALPHABET[((bits >> bit_count) & 0x1F) as usize] as char);
        }
    }
    if bit_count > 0 {
        out.push(BASE32_ALPHABET[((bits << (5 - bit_count)) & 0x1F) as usize] as char);
    }
    out
}

/// CRC16/XMODEM checksum (poly `0x1021`, init `0x0000`), as used for the
/// trailing 2-byte checksum in Stellar's strkey format.
pub(crate) fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Verify the strkey format of a decoded Stellar address: version byte,
/// payload length, and trailing CRC16/XMODEM checksum. Catches typos and
/// bit-flips (e.g. a single swapped character) that a prefix+length+
/// alphanumeric check alone would let through.
fn verify_strkey(s: &str) -> bool {
    let Some(decoded) = base32_decode(s) else {
        return false;
    };
    // version byte (1) + ED25519 public key or contract ID (32) + checksum (2).
    if decoded.len() != 35 {
        return false;
    }
    let version = decoded[0];
    if version != STRKEY_VERSION_ACCOUNT_ID && version != STRKEY_VERSION_CONTRACT {
        return false;
    }
    let payload = &decoded[..33];
    let expected = crc16_xmodem(payload);
    let actual = u16::from_le_bytes([decoded[33], decoded[34]]);
    expected == actual
}

/// A Stellar account address (`G…`) or contract address (`C…`): 56 chars,
/// valid strkey base32 encoding, correct version byte, and correct trailing
/// CRC16/XMODEM checksum.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StellarAddress(String);

impl StellarAddress {
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        let shape_valid = (s.starts_with('G') || s.starts_with('C'))
            && s.len() == 56
            && s.chars().all(|c| c.is_ascii_alphanumeric());
        if shape_valid && verify_strkey(&s) {
            Ok(Self(s))
        } else {
            Err(ValidationError(
                "must be a valid Stellar address (G/C prefix, 56 chars, valid strkey checksum)"
                    .to_string(),
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

    /// Build a syntactically- and checksum-valid strkey address for the given
    /// version byte, so tests don't rely on a hand-typed address that would
    /// only pass the old, weaker prefix+length check.
    fn valid_strkey(version: u8, payload_byte: u8) -> String {
        let mut bytes = vec![version];
        bytes.extend(std::iter::repeat(payload_byte).take(32));
        let checksum = crc16_xmodem(&bytes);
        bytes.extend_from_slice(&checksum.to_le_bytes());
        base32_encode(&bytes)
    }

    #[test]
    fn stellar_address_rejects_invalid() {
        assert!(StellarAddress::new("").is_err());
        assert!(StellarAddress::new("GABC").is_err()); // too short
        assert!(StellarAddress::new("X".repeat(56)).is_err()); // wrong prefix
    }

    #[test]
    fn stellar_address_rejects_wrong_length() {
        let mut addr = valid_strkey(STRKEY_VERSION_ACCOUNT_ID, 0xAB);
        addr.push('A'); // now 57 chars
        assert!(StellarAddress::new(addr).is_err());

        let short: String = valid_strkey(STRKEY_VERSION_ACCOUNT_ID, 0xAB)
            .chars()
            .take(55)
            .collect();
        assert!(StellarAddress::new(short).is_err());
    }

    #[test]
    fn stellar_address_rejects_bad_checksum() {
        let mut addr = valid_strkey(STRKEY_VERSION_ACCOUNT_ID, 0xAB);
        // Flip the last character: same length and prefix, invalid checksum.
        let last = addr.pop().unwrap();
        let flipped = if last == 'A' { 'B' } else { 'A' };
        addr.push(flipped);
        assert!(
            StellarAddress::new(addr).is_err(),
            "a single flipped trailing character must fail the checksum check"
        );
    }

    #[test]
    fn stellar_address_accepts_valid_g_address() {
        let addr = valid_strkey(STRKEY_VERSION_ACCOUNT_ID, 0x01);
        assert!(addr.starts_with('G'));
        assert_eq!(addr.len(), 56);
        assert!(StellarAddress::new(addr).is_ok());
    }

    #[test]
    fn stellar_address_accepts_valid_c_address() {
        let addr = valid_strkey(STRKEY_VERSION_CONTRACT, 0x02);
        assert!(addr.starts_with('C'));
        assert_eq!(addr.len(), 56);
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
