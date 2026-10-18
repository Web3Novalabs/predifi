use predifi_backend::validated_types::{BoundedI64, NonEmptyString, PoolSortBy, PoolStatus, StellarAddress};

// ── NonEmptyString ────────────────────────────────────────────────────────────
// Smallest valid value is a single non-whitespace character; there is no
// upper bound. The boundary just outside "valid" is the empty string.

#[test]
fn non_empty_string_smallest_valid_value() {
    assert!(NonEmptyString::new("a").is_ok());
}

#[test]
fn non_empty_string_just_outside_valid_empty() {
    assert!(NonEmptyString::new("").is_err());
}

#[test]
fn non_empty_string_just_outside_valid_whitespace_only() {
    assert!(NonEmptyString::new(" ").is_err());
}

// ── BoundedI64 ────────────────────────────────────────────────────────────────

#[test]
fn bounded_i64_smallest_valid_value() {
    assert_eq!(BoundedI64::<1, 100>::new(1).unwrap().get(), 1);
}

#[test]
fn bounded_i64_largest_valid_value() {
    assert_eq!(BoundedI64::<1, 100>::new(100).unwrap().get(), 100);
}

#[test]
fn bounded_i64_just_below_min() {
    assert!(BoundedI64::<1, 100>::new(0).is_err());
}

#[test]
fn bounded_i64_just_above_max() {
    assert!(BoundedI64::<1, 100>::new(101).is_err());
}

// ── StellarAddress ────────────────────────────────────────────────────────────
// Valid addresses are exactly 56 alphanumeric characters starting with G or C.

#[test]
fn stellar_address_smallest_valid_length_g_prefix() {
    let addr = format!("G{}", "A".repeat(55));
    assert_eq!(addr.len(), 56);
    assert!(StellarAddress::new(addr).is_ok());
}

#[test]
fn stellar_address_smallest_valid_length_c_prefix() {
    let addr = format!("C{}", "A".repeat(55));
    assert_eq!(addr.len(), 56);
    assert!(StellarAddress::new(addr).is_ok());
}

#[test]
fn stellar_address_just_below_length() {
    let addr = format!("G{}", "A".repeat(54));
    assert_eq!(addr.len(), 55);
    assert!(StellarAddress::new(addr).is_err());
}

#[test]
fn stellar_address_just_above_length() {
    let addr = format!("G{}", "A".repeat(56));
    assert_eq!(addr.len(), 57);
    assert!(StellarAddress::new(addr).is_err());
}

// ── PoolSortBy ────────────────────────────────────────────────────────────────
// A fixed enumeration; every documented member must deserialize, and any
// value outside that set must be rejected.

#[test]
fn pool_sort_by_accepts_every_valid_member() {
    let v: PoolSortBy = serde_json::from_str("\"popular\"").unwrap();
    assert_eq!(v, PoolSortBy::Popular);
    let v: PoolSortBy = serde_json::from_str("\"ending_soon\"").unwrap();
    assert_eq!(v, PoolSortBy::EndingSoon);
    let v: PoolSortBy = serde_json::from_str("\"new\"").unwrap();
    assert_eq!(v, PoolSortBy::New);
}

#[test]
fn pool_sort_by_rejects_value_outside_the_set() {
    let result: Result<PoolSortBy, _> = serde_json::from_str("\"trending\"");
    assert!(result.is_err());
}

// ── PoolStatus ────────────────────────────────────────────────────────────────

#[test]
fn pool_status_accepts_every_valid_member() {
    let v: PoolStatus = serde_json::from_str("\"active\"").unwrap();
    assert_eq!(v, PoolStatus::Active);
    let v: PoolStatus = serde_json::from_str("\"closed\"").unwrap();
    assert_eq!(v, PoolStatus::Closed);
    let v: PoolStatus = serde_json::from_str("\"settled\"").unwrap();
    assert_eq!(v, PoolStatus::Settled);
}

#[test]
fn pool_status_rejects_value_outside_the_set() {
    let result: Result<PoolStatus, _> = serde_json::from_str("\"pending\"");
    assert!(result.is_err());
}
