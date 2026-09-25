#![cfg(test)]

use predifi_contract::{SafeMath, RoundingMode};
use predifi_errors::PrediFiError;

#[test]
fn test_proportion_zero_denominator_returns_arithmetic_error() {
    let result = SafeMath::proportion(100, 0, 1000, RoundingMode::Neutral);
    assert_eq!(result, Err(PrediFiError::ArithmeticError));
}

#[test]
fn test_proportion_zero_numerator_returns_zero() {
    let result = SafeMath::proportion(0, 1000, 5000, RoundingMode::Neutral);
    assert_eq!(result, Ok(0));
}

#[test]
fn test_proportion_part_equals_whole_returns_full_total() {
    let result = SafeMath::proportion(1000, 1000, 5000, RoundingMode::Neutral);
    assert_eq!(result, Ok(5000));
}

#[test]
fn test_proportion_zero_denominator_all_rounding_modes() {
    for rounding in [
        RoundingMode::ProtocolFavor,
        RoundingMode::Neutral,
        RoundingMode::UserFavor,
    ] {
        assert_eq!(
            SafeMath::proportion(100, 0, 1000, rounding),
            Err(PrediFiError::ArithmeticError)
        );
    }
}

#[test]
fn test_proportion_zero_numerator_nonzero_amount() {
    let result = SafeMath::proportion(0, 100, 5000, RoundingMode::Neutral);
    assert_eq!(result, Ok(0));
}

#[test]
fn test_proportion_equal_stakes_returns_full_amount() {
    assert_eq!(
        SafeMath::proportion(1000, 1000, 1000, RoundingMode::Neutral),
        Ok(1000)
    );
    assert_eq!(
        SafeMath::proportion(5_000_000_000_000, 5_000_000_000_000, 5_000_000_000_000, RoundingMode::Neutral),
        Ok(5_000_000_000_000)
    );
}
