use predifi_backend::constants::{DEFAULT_PAGE_LIMIT, MAX_PAGE_LIMIT};

#[test]
fn constants_have_expected_values() {
    assert_eq!(DEFAULT_PAGE_LIMIT, 20);
    assert_eq!(MAX_PAGE_LIMIT, 100);
}

#[test]
fn limit_above_max_is_clamped_to_max() {
    let limit: i64 = 99999;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, MAX_PAGE_LIMIT);
}

#[test]
fn missing_limit_falls_back_to_default() {
    let limit: Option<i64> = None;
    let resolved = limit.unwrap_or(DEFAULT_PAGE_LIMIT).clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(resolved, DEFAULT_PAGE_LIMIT);
}

#[test]
fn zero_limit_is_clamped_to_one() {
    let limit: i64 = 0;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, 1);
}

#[test]
fn negative_limit_is_clamped_to_one() {
    let limit: i64 = -10;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, 1);
}

#[test]
fn limit_at_max_boundary_is_unchanged() {
    let limit: i64 = MAX_PAGE_LIMIT;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, MAX_PAGE_LIMIT);
}

#[test]
fn limit_at_default_is_unchanged() {
    let limit: i64 = DEFAULT_PAGE_LIMIT;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, DEFAULT_PAGE_LIMIT);
}

#[test]
fn limit_just_above_max_is_clamped() {
    let limit: i64 = MAX_PAGE_LIMIT + 1;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, MAX_PAGE_LIMIT);
}

#[test]
fn limit_just_below_max_is_unchanged() {
    let limit: i64 = MAX_PAGE_LIMIT - 1;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, MAX_PAGE_LIMIT - 1);
}

#[test]
fn limit_one_is_unchanged() {
    let limit: i64 = 1;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, 1);
}

#[test]
fn large_negative_limit_is_clamped_to_one() {
    let limit: i64 = i64::MIN;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, 1);
}

#[test]
fn very_large_limit_is_clamped_to_max() {
    let limit: i64 = i64::MAX;
    let clamped = limit.clamp(1, MAX_PAGE_LIMIT);
    assert_eq!(clamped, MAX_PAGE_LIMIT);
}
