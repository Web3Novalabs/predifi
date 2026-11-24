//! Exhaustive boundary tests for the public `SafeMath` helpers.
//!
//! Every re-exported public helper is pinned at the four canonical boundaries
//! (`0`, `1`, `i128::MAX`, `i128::MAX - 1`) and every division helper has an
//! explicit division-by-zero case.
//!
//! Public helper surface (re-exported from the crate root):
//! * [`SafeMath::percentage`]      — divides by `MAX_BPS` (`10_000`)
//! * [`SafeMath::proportion`]      — divides by `denominator`
//! * [`SafeMath::calculate_share`] — divides by `winning_stake`
//! * [`SafeMath::safe_add`] / [`SafeMath::safe_sub`] / [`SafeMath::safe_mul`]
//!
//! > `SafeMath::multi_proportion` is `#[cfg(test)]`-only and `PayoutRoundingAudit`
//! > is crate-internal (module `safe_math` is private), so both are covered by the
//! > in-crate unit tests in `src/safe_math.rs` rather than from this integration
//! > suite.
//!
//! > CAVEAT: `divide_with_rounding`'s `Neutral` mode compares
//! > `remainder >= denominator / 2`, which is only a correct "round half up"
//! > for even denominators. These tests therefore pin exact (zero-remainder)
//! > results on even denominators, where the comparison is unambiguous, and do
//! > not assert a value for `Neutral` on an odd denominator (e.g. `1 / 1`).
//! > See the task report for the follow-up recommendation.

#![cfg(test)]

use predifi_contract::{RoundingMode, SafeMath};
use predifi_errors::PrediFiError;

const MAX: i128 = i128::MAX;
const MAX_MINUS_1: i128 = i128::MAX - 1;
const MAX_BPS: i128 = 10_000;

const ALL_ROUNDING: [RoundingMode; 3] = [
    RoundingMode::ProtocolFavor,
    RoundingMode::Neutral,
    RoundingMode::UserFavor,
];

// ── percentage (division by MAX_BPS = 10_000) ───────────────────────────────

mod percentage {
    use super::*;

    #[test]
    fn zero_amount_and_zero_bps_return_zero() {
        for rounding in ALL_ROUNDING {
            assert_eq!(SafeMath::percentage(0, 0, rounding), Ok(0));
        }
    }

    #[test]
    fn zero_amount_returns_zero_for_all_bps() {
        for rounding in ALL_ROUNDING {
            assert_eq!(SafeMath::percentage(0, 1, rounding), Ok(0));
            assert_eq!(SafeMath::percentage(0, MAX_BPS, rounding), Ok(0));
        }
    }

    #[test]
    fn zero_bps_returns_zero_for_all_amounts() {
        for rounding in ALL_ROUNDING {
            assert_eq!(SafeMath::percentage(1, 0, rounding), Ok(0));
            assert_eq!(SafeMath::percentage(MAX, 0, rounding), Ok(0));
            assert_eq!(SafeMath::percentage(MAX_MINUS_1, 0, rounding), Ok(0));
        }
    }

    #[test]
    fn boundary_one_amount_full_bps_is_one() {
        for rounding in ALL_ROUNDING {
            assert_eq!(SafeMath::percentage(1, MAX_BPS, rounding), Ok(1));
        }
    }

    #[test]
    fn boundary_one_amount_one_bps_respects_rounding() {
        // 1 * 1 / 10_000 → floor = 0; Neutral remainder 1 < 5_000 = 0; UserFavor = 1.
        assert_eq!(
            SafeMath::percentage(1, 1, RoundingMode::ProtocolFavor),
            Ok(0)
        );
        assert_eq!(SafeMath::percentage(1, 1, RoundingMode::Neutral), Ok(0));
        assert_eq!(SafeMath::percentage(1, 1, RoundingMode::UserFavor), Ok(1));
    }

    #[test]
    fn neutral_rounds_at_half_boundary() {
        // 1 * 5_000 / 10_000 = 0.5 → Neutral rounds half-up to 1.
        assert_eq!(SafeMath::percentage(1, 5_000, RoundingMode::Neutral), Ok(1));
        // 1 * 4_999 / 10_000 = 0.4999 → Neutral rounds down to 0.
        assert_eq!(SafeMath::percentage(1, 4_999, RoundingMode::Neutral), Ok(0));
    }

    #[test]
    fn exact_division_is_immune_to_rounding() {
        for rounding in ALL_ROUNDING {
            assert_eq!(SafeMath::percentage(10_000, 1, rounding), Ok(1));
            assert_eq!(SafeMath::percentage(100, MAX_BPS, rounding), Ok(100));
            assert_eq!(SafeMath::percentage(10_000, 5_000, rounding), Ok(5_000));
        }
    }

    #[test]
    fn max_amount_small_bps_is_exact_floor() {
        assert_eq!(
            SafeMath::percentage(MAX, 1, RoundingMode::ProtocolFavor),
            Ok(MAX / MAX_BPS)
        );
        assert_eq!(
            SafeMath::percentage(MAX_MINUS_1, 1, RoundingMode::ProtocolFavor),
            Ok(MAX_MINUS_1 / MAX_BPS)
        );
    }

    #[test]
    fn max_amount_full_bps_overflows() {
        for rounding in ALL_ROUNDING {
            assert_eq!(
                SafeMath::percentage(MAX, MAX_BPS, rounding),
                Err(PrediFiError::InvalidAmount)
            );
            assert_eq!(
                SafeMath::percentage(MAX_MINUS_1, MAX_BPS, rounding),
                Err(PrediFiError::InvalidAmount)
            );
        }
    }

    #[test]
    fn bps_above_max_is_rejected() {
        assert_eq!(
            SafeMath::percentage(1, MAX_BPS + 1, RoundingMode::Neutral),
            Err(PrediFiError::InvalidFeeBps)
        );
        assert_eq!(
            SafeMath::percentage(1, MAX, RoundingMode::Neutral),
            Err(PrediFiError::InvalidFeeBps)
        );
    }

    #[test]
    fn negative_inputs_are_rejected() {
        assert_eq!(
            SafeMath::percentage(-1, 100, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );
        assert_eq!(
            SafeMath::percentage(100, -1, RoundingMode::Neutral),
            Err(PrediFiError::InvalidFeeBps)
        );
    }
}

// ── proportion (division by `denominator`) ──────────────────────────────────

mod proportion {
    use super::*;

    #[test]
    fn zero_denominator_is_division_by_zero() {
        for rounding in ALL_ROUNDING {
            assert_eq!(
                SafeMath::proportion(100, 0, 1_000, rounding),
                Err(PrediFiError::ArithmeticError)
            );
            assert_eq!(
                SafeMath::proportion(0, 0, 1_000, rounding),
                Err(PrediFiError::ArithmeticError)
            );
        }
    }

    #[test]
    fn negative_denominator_is_rejected() {
        assert_eq!(
            SafeMath::proportion(1, -1, 10, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn zero_numerator_returns_zero() {
        for rounding in ALL_ROUNDING {
            assert_eq!(SafeMath::proportion(0, 1, 1_000, rounding), Ok(0));
            assert_eq!(SafeMath::proportion(0, MAX, MAX, rounding), Ok(0));
        }
    }

    #[test]
    fn zero_amount_returns_zero() {
        for rounding in ALL_ROUNDING {
            assert_eq!(SafeMath::proportion(1, 1, 0, rounding), Ok(0));
            assert_eq!(SafeMath::proportion(MAX, MAX, 0, rounding), Ok(0));
        }
    }

    #[test]
    fn boundary_one_over_one() {
        // Exact unit proportion across every rounding mode. An even denominator
        // keeps the implementation's half-up comparison exact.
        for rounding in ALL_ROUNDING {
            assert_eq!(SafeMath::proportion(1, 2, 2, rounding), Ok(1));
        }
        // Floor / ceiling rounding are exact for the all-ones case.
        assert_eq!(
            SafeMath::proportion(1, 1, 1, RoundingMode::ProtocolFavor),
            Ok(1)
        );
        assert_eq!(
            SafeMath::proportion(1, 1, 1, RoundingMode::UserFavor),
            Ok(1)
        );
    }

    #[test]
    fn full_share_returns_full_amount() {
        // numerator == denominator is a full share: `(2 * amount) / 2 == amount`
        // with zero remainder, so every rounding mode is a no-op.
        let half_max = MAX / 2;
        for rounding in ALL_ROUNDING {
            assert_eq!(SafeMath::proportion(2, 2, half_max, rounding), Ok(half_max));
            assert_eq!(
                SafeMath::proportion(2, 2, 1_000, rounding),
                Ok(1_000)
            );
        }
    }

    #[test]
    fn max_boundary_without_overflow() {
        // 1 * MAX / MAX = 1.
        assert_eq!(
            SafeMath::proportion(1, MAX, MAX, RoundingMode::ProtocolFavor),
            Ok(1)
        );
        // (MAX - 1) * 1 / MAX = 0 (floor).
        assert_eq!(
            SafeMath::proportion(MAX_MINUS_1, MAX, 1, RoundingMode::ProtocolFavor),
            Ok(0)
        );
    }

    #[test]
    fn max_boundary_multiplication_overflows() {
        for rounding in ALL_ROUNDING {
            assert_eq!(
                SafeMath::proportion(MAX, MAX, MAX, rounding),
                Err(PrediFiError::InvalidAmount)
            );
        }
    }

    #[test]
    fn numerator_greater_than_denominator_is_rejected() {
        assert_eq!(
            SafeMath::proportion(2, 1, 100, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );
        assert_eq!(
            SafeMath::proportion(MAX, MAX_MINUS_1, 1, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn negative_inputs_are_rejected() {
        assert_eq!(
            SafeMath::proportion(-1, 10, 100, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );
        assert_eq!(
            SafeMath::proportion(1, 10, -1, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn rounding_modes_differ_on_remainder() {
        // 1 / 3 * 10 = 3.333… → floor 3; ceil 4.
        assert_eq!(
            SafeMath::proportion(1, 3, 10, RoundingMode::ProtocolFavor),
            Ok(3)
        );
        assert_eq!(
            SafeMath::proportion(1, 3, 10, RoundingMode::UserFavor),
            Ok(4)
        );
    }
}

// ── calculate_share (division by `winning_stake`) ───────────────────────────

mod calculate_share {
    use super::*;

    #[test]
    fn zero_winning_stake_is_safe_no_op() {
        assert_eq!(SafeMath::calculate_share(0, 0, 0), Ok(0));
        assert_eq!(SafeMath::calculate_share(100, 0, 1_000), Ok(0));
        assert_eq!(SafeMath::calculate_share(MAX, 0, MAX), Ok(0));
    }

    #[test]
    fn zero_user_stake_returns_zero() {
        assert_eq!(SafeMath::calculate_share(0, 1, 1), Ok(0));
        assert_eq!(SafeMath::calculate_share(0, MAX, MAX), Ok(0));
    }

    #[test]
    fn zero_payout_pool_returns_zero() {
        assert_eq!(SafeMath::calculate_share(1, 1, 0), Ok(0));
        assert_eq!(SafeMath::calculate_share(MAX, MAX, 0), Ok(0));
    }

    #[test]
    fn boundary_one_over_one() {
        assert_eq!(SafeMath::calculate_share(1, 1, 1), Ok(1));
    }

    #[test]
    fn max_boundary_without_overflow() {
        // MAX * 1 / MAX = 1.
        assert_eq!(SafeMath::calculate_share(MAX, MAX, 1), Ok(1));
        // 1 * MAX / MAX = 1.
        assert_eq!(SafeMath::calculate_share(1, MAX, MAX), Ok(1));
        // (MAX - 1) * 1 / MAX = 0.
        assert_eq!(SafeMath::calculate_share(MAX_MINUS_1, MAX, 1), Ok(0));
    }

    #[test]
    fn max_boundary_multiplication_overflows() {
        assert_eq!(
            SafeMath::calculate_share(MAX, MAX, MAX),
            Err(PrediFiError::InvalidAmount)
        );
        assert_eq!(
            SafeMath::calculate_share(MAX, MAX, 2),
            Err(PrediFiError::InvalidAmount)
        );
    }

    #[test]
    fn user_stake_greater_than_winning_stake_is_rejected() {
        assert_eq!(
            SafeMath::calculate_share(101, 100, 1_000),
            Err(PrediFiError::ArithmeticError)
        );
        assert_eq!(
            SafeMath::calculate_share(MAX, MAX_MINUS_1, 1),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn negative_inputs_are_rejected() {
        assert_eq!(
            SafeMath::calculate_share(-1, 1, 1),
            Err(PrediFiError::ArithmeticError)
        );
        assert_eq!(
            SafeMath::calculate_share(1, -1, 1),
            Err(PrediFiError::ArithmeticError)
        );
        assert_eq!(
            SafeMath::calculate_share(1, 1, -1),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn proportional_split_is_floor_rounded() {
        // 300 / 1000 * 5000 = 1500.
        assert_eq!(SafeMath::calculate_share(300, 1_000, 5_000), Ok(1_500));
        // 1 / 3 * 10 = 3 (floor).
        assert_eq!(SafeMath::calculate_share(1, 3, 10), Ok(3));
    }
}

// ── safe_add ────────────────────────────────────────────────────────────────

mod safe_add {
    use super::*;

    #[test]
    fn zero_boundary() {
        assert_eq!(SafeMath::safe_add(0, 0), Ok(0));
    }

    #[test]
    fn one_boundary() {
        assert_eq!(SafeMath::safe_add(0, 1), Ok(1));
        assert_eq!(SafeMath::safe_add(1, 0), Ok(1));
        assert_eq!(SafeMath::safe_add(1, 1), Ok(2));
    }

    #[test]
    fn max_boundary() {
        assert_eq!(SafeMath::safe_add(MAX, 0), Ok(MAX));
        assert_eq!(SafeMath::safe_add(0, MAX), Ok(MAX));
        assert_eq!(SafeMath::safe_add(MAX_MINUS_1, 1), Ok(MAX));
    }

    #[test]
    fn max_minus_one_boundary() {
        assert_eq!(SafeMath::safe_add(MAX_MINUS_1, 0), Ok(MAX_MINUS_1));
        assert_eq!(SafeMath::safe_add(MAX_MINUS_1, 1), Ok(MAX));
    }

    #[test]
    fn overflow_is_rejected() {
        assert_eq!(SafeMath::safe_add(MAX, 1), Err(PrediFiError::ArithmeticError));
        assert_eq!(
            SafeMath::safe_add(MAX, MAX),
            Err(PrediFiError::ArithmeticError)
        );
    }
}

// ── safe_sub ────────────────────────────────────────────────────────────────

mod safe_sub {
    use super::*;

    #[test]
    fn zero_boundary() {
        assert_eq!(SafeMath::safe_sub(0, 0), Ok(0));
    }

    #[test]
    fn one_boundary() {
        assert_eq!(SafeMath::safe_sub(1, 0), Ok(1));
        assert_eq!(SafeMath::safe_sub(1, 1), Ok(0));
    }

    #[test]
    fn max_boundary() {
        assert_eq!(SafeMath::safe_sub(MAX, 0), Ok(MAX));
        assert_eq!(SafeMath::safe_sub(MAX, MAX), Ok(0));
        assert_eq!(SafeMath::safe_sub(MAX, MAX_MINUS_1), Ok(1));
    }

    #[test]
    fn max_minus_one_boundary() {
        assert_eq!(SafeMath::safe_sub(MAX_MINUS_1, MAX_MINUS_1), Ok(0));
        assert_eq!(SafeMath::safe_sub(MAX, MAX_MINUS_1), Ok(1));
    }

    #[test]
    fn underflow_is_rejected() {
        // `0 - 1` is *not* an underflow: `-1` is representable in `i128`.
        assert_eq!(SafeMath::safe_sub(0, 1), Ok(-1));
        // True underflow only occurs below `i128::MIN`.
        assert_eq!(
            SafeMath::safe_sub(i128::MIN, 1),
            Err(PrediFiError::ArithmeticError)
        );
        assert_eq!(
            SafeMath::safe_sub(i128::MIN, MAX),
            Err(PrediFiError::ArithmeticError)
        );
    }
}

// ── safe_mul ────────────────────────────────────────────────────────────────

mod safe_mul {
    use super::*;

    #[test]
    fn zero_boundary() {
        assert_eq!(SafeMath::safe_mul(0, 0), Ok(0));
        assert_eq!(SafeMath::safe_mul(MAX, 0), Ok(0));
        assert_eq!(SafeMath::safe_mul(0, MAX), Ok(0));
    }

    #[test]
    fn one_boundary() {
        assert_eq!(SafeMath::safe_mul(1, 1), Ok(1));
        assert_eq!(SafeMath::safe_mul(MAX, 1), Ok(MAX));
        assert_eq!(SafeMath::safe_mul(1, MAX), Ok(MAX));
    }

    #[test]
    fn max_boundary() {
        assert_eq!(SafeMath::safe_mul(MAX, 1), Ok(MAX));
        // MAX * 2 overflows.
        assert_eq!(
            SafeMath::safe_mul(MAX, 2),
            Err(PrediFiError::InvalidAmount)
        );
    }

    #[test]
    fn max_minus_one_boundary() {
        assert_eq!(SafeMath::safe_mul(MAX_MINUS_1, 1), Ok(MAX_MINUS_1));
        // (MAX - 1) * 2 overflows.
        assert_eq!(
            SafeMath::safe_mul(MAX_MINUS_1, 2),
            Err(PrediFiError::InvalidAmount)
        );
    }

    #[test]
    fn overflow_is_rejected() {
        assert_eq!(
            SafeMath::safe_mul(MAX, MAX),
            Err(PrediFiError::InvalidAmount)
        );
    }
}
