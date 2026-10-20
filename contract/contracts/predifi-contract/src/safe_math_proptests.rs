//! Property-based tests for the public operations in `safe_math.rs` (Issue #1899).
//!
//! These complement the in-file `safe_math::tests::property_tests` module by
//! sweeping the full `i128` range for the checked primitives, and by covering
//! the input-validation, zero-denominator, rounding-order and
//! `PayoutRoundingAudit` paths.

use crate::safe_math::{PayoutRoundingAudit, RoundingMode, SafeMath};
use predifi_errors::PrediFiError;
use proptest::prelude::*;

fn rounding_modes() -> impl Strategy<Value = RoundingMode> {
    prop_oneof![
        Just(RoundingMode::ProtocolFavor),
        Just(RoundingMode::Neutral),
        Just(RoundingMode::UserFavor),
    ]
}

// ─── safe_add / safe_sub / safe_mul ──────────────────────────────────────────

proptest! {
    /// Over the whole `i128` domain the helpers agree with `checked_*`: the
    /// same value when it fits, and a typed error (never a wrap or panic) when
    /// it does not.
    #[test]
    fn prop_checked_primitives_match_std_over_full_range(a in any::<i128>(), b in any::<i128>()) {
        prop_assert_eq!(
            SafeMath::safe_add(a, b),
            a.checked_add(b).ok_or(PrediFiError::ArithmeticError)
        );
        prop_assert_eq!(
            SafeMath::safe_sub(a, b),
            a.checked_sub(b).ok_or(PrediFiError::ArithmeticError)
        );
        prop_assert_eq!(
            SafeMath::safe_mul(a, b),
            a.checked_mul(b).ok_or(PrediFiError::InvalidAmount)
        );
    }

    /// Adding past `i128::MAX` or subtracting past `i128::MIN` always errors.
    #[test]
    fn prop_overflow_and_underflow_are_rejected(
        distance in 0i128..=1_000_000i128,
        extra in 1i128..=1_000_000i128,
    ) {
        // `i128::MAX - distance + (distance + extra)` is `i128::MAX + extra`.
        prop_assert_eq!(
            SafeMath::safe_add(i128::MAX - distance, distance + extra),
            Err(PrediFiError::ArithmeticError)
        );
        // `i128::MIN + distance - (distance + extra)` is `i128::MIN - extra`.
        prop_assert_eq!(
            SafeMath::safe_sub(i128::MIN + distance, distance + extra),
            Err(PrediFiError::ArithmeticError)
        );
        // Landing exactly on the limit is still fine.
        prop_assert_eq!(SafeMath::safe_add(i128::MAX - distance, distance), Ok(i128::MAX));
        prop_assert_eq!(SafeMath::safe_sub(i128::MIN + distance, distance), Ok(i128::MIN));
    }

    /// Algebraic identities that hold wherever the operation does not overflow.
    #[test]
    fn prop_primitive_identities(a in any::<i128>(), b in any::<i128>()) {
        prop_assert_eq!(SafeMath::safe_add(a, 0), Ok(a));
        prop_assert_eq!(SafeMath::safe_sub(a, 0), Ok(a));
        prop_assert_eq!(SafeMath::safe_sub(a, a), Ok(0));
        prop_assert_eq!(SafeMath::safe_mul(a, 1), Ok(a));
        prop_assert_eq!(SafeMath::safe_mul(a, 0), Ok(0));
        prop_assert_eq!(SafeMath::safe_add(a, b), SafeMath::safe_add(b, a));
        prop_assert_eq!(SafeMath::safe_mul(a, b), SafeMath::safe_mul(b, a));
    }
}

// ─── percentage ──────────────────────────────────────────────────────────────

proptest! {
    /// Basis points outside `0..=10_000` are rejected, even for a zero amount.
    #[test]
    fn prop_percentage_rejects_out_of_range_bps(
        amount in 0i128..=1_000_000_000_000i128,
        too_high in 10_001i128..=1_000_000i128,
        negative in i128::MIN..=-1i128,
        rounding in rounding_modes(),
    ) {
        prop_assert_eq!(
            SafeMath::percentage(amount, too_high, rounding),
            Err(PrediFiError::InvalidFeeBps)
        );
        prop_assert_eq!(
            SafeMath::percentage(amount, negative, rounding),
            Err(PrediFiError::InvalidFeeBps)
        );
    }

    #[test]
    fn prop_percentage_rejects_negative_amount(
        negative in i128::MIN..=-1i128,
        bps in 0i128..=10_000i128,
        rounding in rounding_modes(),
    ) {
        prop_assert_eq!(
            SafeMath::percentage(negative, bps, rounding),
            Err(PrediFiError::ArithmeticError)
        );
    }

    /// `amount * bps` overflowing `i128` is reported, not wrapped.
    #[test]
    fn prop_percentage_rejects_product_overflow(
        amount in (i128::MAX / 2 + 1)..=i128::MAX,
        bps in 2i128..=10_000i128,
        rounding in rounding_modes(),
    ) {
        prop_assert_eq!(
            SafeMath::percentage(amount, bps, rounding),
            Err(PrediFiError::InvalidAmount)
        );
    }

    /// A percentage of an amount is never larger than the amount itself, and
    /// 0 bps / 10_000 bps give exactly nothing / everything.
    #[test]
    fn prop_percentage_is_bounded_by_amount(
        amount in 0i128..=1_000_000_000_000i128,
        bps in 0i128..=10_000i128,
        rounding in rounding_modes(),
    ) {
        let result = SafeMath::percentage(amount, bps, rounding).unwrap();
        prop_assert!((0..=amount).contains(&result));
        prop_assert_eq!(SafeMath::percentage(amount, 0, rounding), Ok(0));
        prop_assert_eq!(SafeMath::percentage(amount, 10_000, rounding), Ok(amount));
    }

    /// Floor <= nearest <= ceiling, and the floor/ceiling gap is at most one
    /// unit (so rounding can never move more than one unit of dust).
    #[test]
    fn prop_percentage_rounding_modes_are_ordered(
        amount in 0i128..=1_000_000_000_000i128,
        bps in 0i128..=10_000i128,
    ) {
        let floor = SafeMath::percentage(amount, bps, RoundingMode::ProtocolFavor).unwrap();
        let nearest = SafeMath::percentage(amount, bps, RoundingMode::Neutral).unwrap();
        let ceiling = SafeMath::percentage(amount, bps, RoundingMode::UserFavor).unwrap();

        prop_assert!(floor <= nearest && nearest <= ceiling);
        prop_assert!(ceiling - floor <= 1);
        // The floor mode is plain integer division.
        prop_assert_eq!(floor, amount * bps / 10_000);
    }
}

// ─── proportion ──────────────────────────────────────────────────────────────

proptest! {
    /// A zero or negative denominator is reported as an error rather than a
    /// division by zero panic, whatever the other arguments are.
    #[test]
    fn prop_proportion_rejects_non_positive_denominator(
        numerator in 0i128..=i128::MAX,
        denominator in i128::MIN..=0i128,
        amount in 0i128..=i128::MAX,
        rounding in rounding_modes(),
    ) {
        prop_assert_eq!(
            SafeMath::proportion(numerator, denominator, amount, rounding),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn prop_proportion_rejects_negative_numerator_or_amount(
        negative in i128::MIN..=-1i128,
        denominator in 1i128..=1_000_000_000i128,
        amount in 0i128..=1_000_000_000i128,
        rounding in rounding_modes(),
    ) {
        prop_assert_eq!(
            SafeMath::proportion(negative, denominator, amount, rounding),
            Err(PrediFiError::ArithmeticError)
        );
        prop_assert_eq!(
            SafeMath::proportion(1, denominator, negative, rounding),
            Err(PrediFiError::ArithmeticError)
        );
    }

    /// A share larger than the whole (numerator > denominator) is rejected.
    #[test]
    fn prop_proportion_rejects_share_above_whole(
        denominator in 1i128..=1_000_000_000i128,
        excess in 1i128..=1_000_000_000i128,
        amount in 1i128..=1_000_000_000i128,
        rounding in rounding_modes(),
    ) {
        prop_assert_eq!(
            SafeMath::proportion(denominator + excess, denominator, amount, rounding),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn prop_proportion_rejects_product_overflow(
        numerator in 2i128..=1_000_000i128,
        amount in (i128::MAX / 2 + 1)..=i128::MAX,
        rounding in rounding_modes(),
    ) {
        prop_assert_eq!(
            SafeMath::proportion(numerator, numerator, amount, rounding),
            Err(PrediFiError::InvalidAmount)
        );
    }

    /// Results never exceed the amount being split, and owning the whole
    /// denominator returns the whole amount, in every rounding mode.
    #[test]
    fn prop_proportion_is_bounded_and_whole_share_is_exact(
        numerator in 0i128..=1_000_000_000i128,
        extra in 0i128..=1_000_000_000i128,
        amount in 0i128..=1_000_000_000_000i128,
        rounding in rounding_modes(),
    ) {
        // The whole-share check below needs a denominator of at least 2; see
        // `prop_proportion_rounding_modes_are_ordered`.
        let denominator = numerator + extra + 2;

        let part = SafeMath::proportion(numerator, denominator, amount, rounding).unwrap();
        prop_assert!((0..=amount).contains(&part));
        prop_assert_eq!(
            SafeMath::proportion(denominator, denominator, amount, rounding),
            Ok(amount)
        );
    }

    /// Flooring is monotonic: a bigger stake never gets a smaller share.
    #[test]
    fn prop_proportion_floor_is_monotonic_in_numerator(
        small in 0i128..=1_000_000i128,
        gap in 0i128..=1_000_000i128,
        extra in 0i128..=1_000_000i128,
        amount in 0i128..=1_000_000_000_000i128,
    ) {
        let large = small + gap;
        let denominator = large + extra + 1;
        let low = SafeMath::proportion(small, denominator, amount, RoundingMode::ProtocolFavor).unwrap();
        let high = SafeMath::proportion(large, denominator, amount, RoundingMode::ProtocolFavor).unwrap();
        prop_assert!(low <= high);
    }

    /// Floor <= nearest <= ceiling with at most one unit between floor and
    /// ceiling. Denominators start at 2: that is the range where "nearest"
    /// behaves as a rounding step between the other two modes.
    #[test]
    fn prop_proportion_rounding_modes_are_ordered(
        numerator in 0i128..=1_000_000_000i128,
        extra in 0i128..=1_000_000_000i128,
        amount in 0i128..=1_000_000_000i128,
    ) {
        let denominator = numerator + extra + 2;
        let floor = SafeMath::proportion(numerator, denominator, amount, RoundingMode::ProtocolFavor).unwrap();
        let nearest = SafeMath::proportion(numerator, denominator, amount, RoundingMode::Neutral).unwrap();
        let ceiling = SafeMath::proportion(numerator, denominator, amount, RoundingMode::UserFavor).unwrap();

        prop_assert!(floor <= nearest && nearest <= ceiling);
        prop_assert!(ceiling - floor <= 1);
        prop_assert_eq!(floor, numerator * amount / denominator);
    }
}

// ─── calculate_share ─────────────────────────────────────────────────────────

proptest! {
    /// Having no winning stake is a no-op (0), not a division by zero.
    #[test]
    fn prop_calculate_share_with_no_winning_stake_is_zero(
        user_stake in 0i128..=i128::MAX,
        payout_pool in 0i128..=i128::MAX,
    ) {
        prop_assert_eq!(SafeMath::calculate_share(user_stake, 0, payout_pool), Ok(0));
    }

    #[test]
    fn prop_calculate_share_rejects_negative_inputs(
        negative in i128::MIN..=-1i128,
        value in 0i128..=1_000_000_000i128,
    ) {
        prop_assert_eq!(
            SafeMath::calculate_share(negative, value, value),
            Err(PrediFiError::ArithmeticError)
        );
        prop_assert_eq!(
            SafeMath::calculate_share(value, negative, value),
            Err(PrediFiError::ArithmeticError)
        );
        prop_assert_eq!(
            SafeMath::calculate_share(value, value, negative),
            Err(PrediFiError::ArithmeticError)
        );
    }

    /// A user cannot hold more of the winning side than the whole winning side.
    #[test]
    fn prop_calculate_share_rejects_user_stake_above_winning_stake(
        winning_stake in 1i128..=1_000_000_000i128,
        excess in 1i128..=1_000_000_000i128,
        payout_pool in 1i128..=1_000_000_000i128,
    ) {
        prop_assert_eq!(
            SafeMath::calculate_share(winning_stake + excess, winning_stake, payout_pool),
            Err(PrediFiError::ArithmeticError)
        );
    }

    /// A share is never above the pool, and the whole winning side takes the
    /// whole pool.
    #[test]
    fn prop_calculate_share_is_bounded_by_pool(
        user_stake in 0i128..=1_000_000_000i128,
        extra in 0i128..=1_000_000_000i128,
        payout_pool in 0i128..=1_000_000_000_000i128,
    ) {
        let winning_stake = user_stake + extra;
        let share = SafeMath::calculate_share(user_stake, winning_stake, payout_pool).unwrap();
        prop_assert!((0..=payout_pool).contains(&share));

        if winning_stake > 0 {
            prop_assert_eq!(
                SafeMath::calculate_share(winning_stake, winning_stake, payout_pool),
                Ok(payout_pool)
            );
        }
    }
}

// ─── PayoutRoundingAudit ─────────────────────────────────────────────────────

proptest! {
    /// For consistent inputs (user <= winning side <= pool) the audit passes,
    /// and its fee and payout pool add back up to the pool.
    #[test]
    fn prop_audit_passes_for_consistent_inputs(
        user_stake in 0i128..=1_000_000_000i128,
        extra_winning in 0i128..=1_000_000_000i128,
        extra_total in 0i128..=1_000_000_000_000i128,
        fee_bps in 0i128..=10_000i128,
    ) {
        let winning_stake = user_stake + extra_winning;
        let pool_total_stake = winning_stake + extra_total;
        let audit = PayoutRoundingAudit::new(pool_total_stake, fee_bps, user_stake, winning_stake).unwrap();

        let fee = audit.calculate_protocol_fee().unwrap();
        let payout_pool = audit.calculate_payout_pool().unwrap();
        let payout = audit.calculate_user_payout().unwrap();

        prop_assert_eq!(fee + payout_pool, pool_total_stake);
        prop_assert!((0..=payout_pool).contains(&payout));
        prop_assert_eq!(audit.validate_payout(), Ok(()));
    }

    /// An empty winning side pays nothing and does not divide by zero.
    #[test]
    fn prop_audit_with_no_winning_stake_pays_zero(
        pool_total_stake in 0i128..=1_000_000_000_000i128,
        fee_bps in 0i128..=10_000i128,
        user_stake in 0i128..=1_000_000_000i128,
    ) {
        let audit = PayoutRoundingAudit::new(pool_total_stake, fee_bps, user_stake, 0).unwrap();
        prop_assert_eq!(audit.calculate_user_payout(), Ok(0));
        prop_assert_eq!(audit.validate_payout(), Ok(()));
    }

    /// The constructor rejects fees above 100% and negative amounts.
    #[test]
    fn prop_audit_rejects_invalid_inputs(
        bad_fee_bps in 10_001i128..=1_000_000i128,
        negative in i128::MIN..=-1i128,
        value in 0i128..=1_000_000_000i128,
    ) {
        prop_assert_eq!(
            PayoutRoundingAudit::new(value, bad_fee_bps, value, value),
            Err(PrediFiError::InvalidFeeBps)
        );
        prop_assert_eq!(
            PayoutRoundingAudit::new(negative, 100, value, value),
            Err(PrediFiError::ArithmeticError)
        );
        prop_assert_eq!(
            PayoutRoundingAudit::new(value, 100, negative, value),
            Err(PrediFiError::ArithmeticError)
        );
        prop_assert_eq!(
            PayoutRoundingAudit::new(value, 100, value, negative),
            Err(PrediFiError::ArithmeticError)
        );
    }

    /// A batch whose stakes add up to the winning side pays out no more than
    /// the payout pool, and the dust left over is smaller than the batch.
    #[test]
    fn prop_audit_batch_never_exceeds_payout_pool(
        user_stakes in prop::collection::vec(1i128..=1_000_000_000i128, 1..20),
        extra_total in 0i128..=1_000_000_000_000i128,
        fee_bps in 0i128..=10_000i128,
    ) {
        let winning_stake: i128 = user_stakes.iter().sum();
        let pool_total_stake = winning_stake + extra_total;

        let total_payout = PayoutRoundingAudit::validate_batch_payouts(
            pool_total_stake,
            fee_bps,
            &user_stakes,
            winning_stake,
        ).unwrap();

        let fee = SafeMath::percentage(pool_total_stake, fee_bps, RoundingMode::ProtocolFavor).unwrap();
        let payout_pool = pool_total_stake - fee;
        prop_assert!(total_payout <= payout_pool);
        prop_assert!(payout_pool - total_payout < user_stakes.len() as i128);
    }

    /// A winning side larger than the whole pool is impossible and rejected.
    #[test]
    fn prop_audit_batch_rejects_winning_stake_above_pool(
        pool_total_stake in 0i128..=1_000_000_000i128,
        excess in 1i128..=1_000_000_000i128,
        fee_bps in 0i128..=10_000i128,
    ) {
        prop_assert_eq!(
            PayoutRoundingAudit::validate_batch_payouts(
                pool_total_stake,
                fee_bps,
                &[1],
                pool_total_stake + excess,
            ),
            Err(PrediFiError::ArithmeticError)
        );
    }
}
