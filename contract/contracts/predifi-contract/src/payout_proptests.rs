#[cfg(test)]
extern crate std;
#[cfg(test)]
use crate::payouts::{
    calculate_claim_payout, calculate_payout_pool, calculate_protocol_fee, PayoutInput,
};
#[cfg(test)]
use crate::safe_math::{RoundingMode, SafeMath};
#[cfg(test)]
use predifi_errors::PrediFiError;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use std::vec::Vec;

#[cfg(test)]
proptest! {
    #[test]
    fn test_payout_invariants(
        total_stake in 1..100_000_000_000_000i128, // Up to 10M tokens with 7 decimals
        fee_bps in 0..=10_000u32,
        winning_outcome_share_bps in 1..=10_000u32,
    ) {
        let fee_bps_i = fee_bps as i128;

        // 1. Calculate protocol fee
        let protocol_fee_total = SafeMath::percentage(total_stake, fee_bps_i, RoundingMode::ProtocolFavor).unwrap();
        let payout_pool = total_stake - protocol_fee_total;

        // 2. Calculate winning stake based on winning_outcome_share_bps
        // We ensure at least 1 unit is winning stake if total_stake > 0
        let winning_stake = (total_stake * winning_outcome_share_bps as i128) / 10_000;
        let winning_stake = if winning_stake == 0 { 1 } else { winning_stake };

        // 3. Simulate multiple winners staking on the winning outcome
        // For simplicity in proptest, we can just check if a single user with the full winning_stake gets exactly payout_pool
        // (Modulo rounding if winning_stake != total_stake, but here winning_stake is the total competing for payout_pool)

        // If one user has the entire winning_stake:
        let user_stake = winning_stake;
        let winnings = SafeMath::calculate_share(user_stake, winning_stake, payout_pool).unwrap();

        // Invariant: winnings + protocol_fee_total <= total_stake
        prop_assert!(winnings + protocol_fee_total <= total_stake);

        // Invariant: winnings should be equal to payout_pool if one user owns the whole winning side
        prop_assert_eq!(winnings, payout_pool);
    }

    #[test]
    fn test_distribution_sum_invariant(
        total_stake in 1000..100_000_000_000_000i128,
        fee_bps in 0..=5000u32, // Up to 50% fee
        num_winners in 1..20usize,
    ) {
        let fee_bps_i = fee_bps as i128;
        let protocol_fee_total = SafeMath::percentage(total_stake, fee_bps_i, RoundingMode::ProtocolFavor).unwrap();
        let payout_pool = total_stake - protocol_fee_total;

        // Distribute winning_stake among num_winners
        let winning_stake = total_stake / 2; // Assume 50% of total stake is winning
        if winning_stake == 0 { return Ok(()); }

        let mut individual_stakes = Vec::new();
        let mut remaining_winning_stake = winning_stake;

        for i in 0..num_winners {
            if i == num_winners - 1 {
                individual_stakes.push(remaining_winning_stake);
            } else {
                let stake = remaining_winning_stake / 2;
                if stake > 0 {
                    individual_stakes.push(stake);
                    remaining_winning_stake -= stake;
                } else {
                    individual_stakes.push(remaining_winning_stake);
                    break;
                }
            }
        }

        let mut total_winnings = 0i128;
        for &user_stake in individual_stakes.iter() {
            if user_stake > 0 {
                let winnings = SafeMath::calculate_share(user_stake, winning_stake, payout_pool).unwrap();
                total_winnings += winnings;
            }
        }

        // Sum of winnings + fee should be <= total_stake
        prop_assert!(total_winnings + protocol_fee_total <= total_stake);

        // The difference (dust) should be less than the number of winners
        prop_assert!(payout_pool - total_winnings < num_winners as i128);
    }

    /// Invariant: the protocol fee is always non-negative and never exceeds the
    /// total stake, regardless of fee rate.
    ///
    /// This is a separate, minimal property that gives the test runner maximum
    /// freedom to find corner cases (e.g. fee_bps = 0 or fee_bps = 10_000).
    #[test]
    fn test_fee_always_within_stake_bounds(
        total_stake in 1..100_000_000_000_000i128,
        fee_bps in 0..=10_000u32,
    ) {
        let fee_bps_i = fee_bps as i128;
        let protocol_fee = SafeMath::percentage(total_stake, fee_bps_i, RoundingMode::ProtocolFavor).unwrap();

        // Fee is non-negative — the protocol never owes money to participants
        prop_assert!(protocol_fee >= 0, "fee must be >= 0");

        // Fee never exceeds the stake pool — payouts would go negative otherwise
        prop_assert!(
            protocol_fee <= total_stake,
            "fee ({}) must not exceed total_stake ({})",
            protocol_fee,
            total_stake
        );

        // Payout pool is non-negative after fee deduction
        let payout_pool = total_stake - protocol_fee;
        prop_assert!(payout_pool >= 0, "payout_pool must be >= 0");
    }

    #[test]
    fn prop_total_payouts_never_exceed_pool_stake(
        total_stake in 1..100_000_000_000_000i128,
        fee_bps in 0..=10_000u32,
        winning_stake in 1..=100_000_000i128,
        first_winner_stake in 0..=100_000_000i128,
    ) {
        let protocol_fee = SafeMath::percentage(
            total_stake,
            fee_bps as i128,
            RoundingMode::ProtocolFavor,
        ).unwrap();
        let payout_pool = total_stake - protocol_fee;
        let first_winner_stake = first_winner_stake.min(winning_stake);
        let second_winner_stake = winning_stake - first_winner_stake;

        let first_payout = SafeMath::calculate_share(
            first_winner_stake,
            winning_stake,
            payout_pool,
        ).unwrap();
        let second_payout = SafeMath::calculate_share(
            second_winner_stake,
            winning_stake,
            payout_pool,
        ).unwrap();

        prop_assert!(first_payout + second_payout <= total_stake);
    }

    #[test]
    fn prop_fee_deduction_is_mathematically_consistent(
        total_stake in 1..100_000_000_000_000i128,
        fee_bps in 0..=10_000u32,
    ) {
        let fee = SafeMath::percentage(
            total_stake,
            fee_bps as i128,
            RoundingMode::ProtocolFavor,
        ).unwrap();
        let payout_pool = total_stake - fee;
        let expected_fee = (total_stake * fee_bps as i128) / 10_000;

        prop_assert_eq!(fee, expected_fee);
        prop_assert_eq!(payout_pool + fee, total_stake);
    }

    #[test]
    fn prop_no_user_receives_negative_payout(
        user_stake in 0..=100_000_000i128,
        winning_stake in 0..=100_000_000i128,
        payout_pool in 0..=100_000_000_000_000i128,
    ) {
        let user_stake = user_stake.min(winning_stake);
        let payout = SafeMath::calculate_share(user_stake, winning_stake, payout_pool).unwrap();

        prop_assert!(payout >= 0);
    }

    #[test]
    fn prop_all_claims_equal_pool_balance_minus_fees(
        total_stake in 1..100_000_000_000_000i128,
        fee_bps in 0..=10_000u32,
        first_winner_stake in 0..=100_000_000i128,
    ) {
        let fee = SafeMath::percentage(
            total_stake,
            fee_bps as i128,
            RoundingMode::ProtocolFavor,
        ).unwrap();
        let payout_pool = total_stake - fee;
        if payout_pool == 0 {
            return Ok(());
        }

        let winning_stake = payout_pool;
        let first_winner_stake = first_winner_stake.min(winning_stake);
        let second_winner_stake = winning_stake - first_winner_stake;

        let first_claim = SafeMath::calculate_share(
            first_winner_stake,
            winning_stake,
            payout_pool,
        ).unwrap();
        let second_claim = SafeMath::calculate_share(
            second_winner_stake,
            winning_stake,
            payout_pool,
        ).unwrap();

        prop_assert_eq!(first_claim + second_claim, payout_pool);
    }

    /// Issue #1753 — the sum of every winner's claim must never exceed what the
    /// pool actually holds, for an arbitrary number of winners splitting the
    /// winning side in an arbitrary way.
    ///
    /// This exercises `payouts::calculate_claim_payout` — the exact function
    /// `claim_winnings` calls in `lib.rs` — rather than re-deriving the math
    /// from `SafeMath` primitives directly, so a regression in how the
    /// production code wires fee deduction and per-winner rounding together
    /// (not just in the primitives themselves) would be caught here.
    ///
    /// Each winner's share is computed independently via truncating integer
    /// division, so floor(share_i) <= share_i for every winner and the sum of
    /// floors can never exceed the sum of the exact shares (which equals
    /// `payout_pool` by construction, since the generated stakes always sum to
    /// `winning_stake`). The bug this guards against is a change that makes an
    /// individual winner's calculation round up, or that double-counts the fee
    /// deduction — either of which would only surface with more than one or
    /// two winners and an uneven stake split, which is why this is generated
    /// rather than hard-coded.
    #[test]
    fn prop_sum_of_all_claims_never_exceeds_pool_via_production_payout_fn(
        winner_stakes in prop::collection::vec(1i128..=1_000_000_000i128, 1..25),
        losing_stake in 0i128..=1_000_000_000i128,
        fee_bps in 0..=10_000u32,
    ) {
        let winning_stake: i128 = winner_stakes.iter().sum();
        let total_stake = winning_stake + losing_stake;

        let payout_pool = calculate_payout_pool(total_stake, fee_bps as i128).unwrap();

        let mut total_claimed = 0i128;
        for &user_stake in winner_stakes.iter() {
            let breakdown = calculate_claim_payout(&PayoutInput {
                pool_total_stake: total_stake,
                fee_bps: fee_bps as i128,
                user_stake,
                winning_stake,
            }).unwrap();
            total_claimed += breakdown.winnings;
        }

        prop_assert!(
            total_claimed <= payout_pool,
            "sum of claims ({}) exceeded payout pool ({}) across {} winners",
            total_claimed,
            payout_pool,
            winner_stakes.len()
        );
        prop_assert!(
            total_claimed <= total_stake,
            "sum of claims ({}) exceeded total pool stake ({})",
            total_claimed,
            total_stake
        );
    }
}

// ─── Issue #1461 — Property-based Tests: Safe math operations ────────────────
//
// The following proptests target the SafeMath primitives directly and are
// independent of the pool lifecycle.  They validate arithmetic invariants
// that must hold for all representable input values.

#[cfg(test)]
proptest! {
    /// SafeMath::safe_add must be commutative: a + b == b + a.
    #[test]
    fn prop_safe_add_commutative(
        a in 0..i128::MAX / 2,
        b in 0..i128::MAX / 2,
    ) {
        let ab = SafeMath::safe_add(a, b).unwrap();
        let ba = SafeMath::safe_add(b, a).unwrap();
        prop_assert_eq!(ab, ba);
    }

    /// SafeMath::safe_add followed by SafeMath::safe_sub returns the original value.
    #[test]
    fn prop_safe_add_sub_roundtrip(
        a in 0..=i128::MAX / 2,
        b in 0..=i128::MAX / 2,
    ) {
        let sum = SafeMath::safe_add(a, b).unwrap();
        let back = SafeMath::safe_sub(sum, b).unwrap();
        prop_assert_eq!(back, a);
    }

    /// SafeMath::proportion output is always <= the `amount` argument for any valid inputs.
    ///
    /// A user's proportional share of a pool can never exceed the pool itself.
    #[test]
    fn prop_proportion_result_bounded_by_amount(
        numerator in 1i128..=1_000_000_000_000i128,
        denominator_extra in 0i128..=1_000_000_000_000i128,
        amount in 1i128..=1_000_000_000_000i128,
    ) {
        let denominator = numerator + denominator_extra; // guarantees numerator <= denominator
        let result = SafeMath::proportion(numerator, denominator, amount, RoundingMode::Neutral).unwrap();
        prop_assert!(result <= amount, "proportion result ({}) exceeded amount ({})", result, amount);
    }

    /// SafeMath::safe_mul result equals manual multiplication for inputs that do not overflow.
    #[test]
    fn prop_safe_mul_matches_checked_mul(
        a in 0i128..=1_000_000_000i128,
        b in 0i128..=1_000_000_000i128,
    ) {
        let expected = a.checked_mul(b).unwrap();
        let result = SafeMath::safe_mul(a, b).unwrap();
        prop_assert_eq!(result, expected);
    }
}

// ─── Issue #1898 — Payout invariants through the production payout functions ──
//
// The properties above re-derive the payout math from `SafeMath` primitives.
// The ones below drive `payouts::calculate_claim_payout`, the function
// `claim_winnings` actually calls, so a regression in how fee deduction and
// per-winner rounding are composed is caught as well.

#[cfg(test)]
proptest! {
    /// The breakdown's fee and payout pool always add back up to the pool's
    /// stake, and the fee is exactly the floored `fee_bps` share of it.
    #[test]
    fn prop_claim_breakdown_fee_and_pool_are_consistent(
        pool_total_stake in 0..=100_000_000_000_000i128,
        fee_bps in 0..=10_000i128,
        user_stake in 0..=1_000_000_000i128,
        extra_winning_stake in 0..=1_000_000_000i128,
    ) {
        let winning_stake = user_stake + extra_winning_stake;
        // The winning side is part of the pool, so the pool can't be smaller.
        let total = pool_total_stake.max(winning_stake);

        let breakdown = calculate_claim_payout(&PayoutInput {
            pool_total_stake: total,
            fee_bps,
            user_stake,
            winning_stake,
        }).unwrap();

        prop_assert_eq!(breakdown.protocol_fee + breakdown.payout_pool, total);
        prop_assert_eq!(breakdown.protocol_fee, calculate_protocol_fee(total, fee_bps).unwrap());
        prop_assert_eq!(breakdown.payout_pool, calculate_payout_pool(total, fee_bps).unwrap());

        // fee == floor(total * fee_bps / 10_000)
        prop_assert!(breakdown.protocol_fee * 10_000 <= total * fee_bps);
        prop_assert!(total * fee_bps < (breakdown.protocol_fee + 1) * 10_000);
    }

    /// A single claim is never negative and never more than the payout pool
    /// (and therefore never more than the pool's total stake).
    #[test]
    fn prop_claim_is_non_negative_and_bounded_by_payout_pool(
        pool_total_stake in 0..=100_000_000_000_000i128,
        fee_bps in 0..=10_000i128,
        user_stake in 0..=1_000_000_000i128,
        extra_winning_stake in 0..=1_000_000_000i128,
    ) {
        let winning_stake = user_stake + extra_winning_stake;
        let total = pool_total_stake.max(winning_stake);

        let breakdown = calculate_claim_payout(&PayoutInput {
            pool_total_stake: total,
            fee_bps,
            user_stake,
            winning_stake,
        }).unwrap();

        prop_assert!(breakdown.winnings >= 0);
        prop_assert!(breakdown.winnings <= breakdown.payout_pool);
        prop_assert!(breakdown.winnings <= total);
        // No stake, or no winning side, means nothing to claim.
        if user_stake == 0 || winning_stake == 0 {
            prop_assert_eq!(breakdown.winnings, 0);
        }
    }

    /// Once every winner has claimed, fee + claims + leftover dust accounts for
    /// the whole pool. The dust is non-negative and smaller than the number of
    /// winners (each floored claim loses less than one unit), and a lone
    /// winner collects the entire payout pool.
    #[test]
    fn prop_all_claims_plus_fee_account_for_whole_pool(
        winner_stakes in prop::collection::vec(1i128..=1_000_000_000i128, 1..25),
        loser_stakes in prop::collection::vec(1i128..=1_000_000_000i128, 0..10),
        fee_bps in 0..=10_000i128,
    ) {
        let winning_stake: i128 = winner_stakes.iter().sum();
        let losing_stake: i128 = loser_stakes.iter().sum();
        let total = winning_stake + losing_stake;

        let mut protocol_fee = 0i128;
        let mut payout_pool = 0i128;
        let mut total_claimed = 0i128;
        for &user_stake in winner_stakes.iter() {
            let breakdown = calculate_claim_payout(&PayoutInput {
                pool_total_stake: total,
                fee_bps,
                user_stake,
                winning_stake,
            }).unwrap();
            prop_assert!(breakdown.winnings >= 0);
            // Every claimant sees the same fee and pool.
            protocol_fee = breakdown.protocol_fee;
            payout_pool = breakdown.payout_pool;
            total_claimed += breakdown.winnings;
        }

        let dust = payout_pool - total_claimed;
        prop_assert!(dust >= 0, "claims ({}) exceeded payout pool ({})", total_claimed, payout_pool);
        prop_assert!(
            dust < winner_stakes.len() as i128,
            "dust ({}) must be smaller than the winner count ({})",
            dust,
            winner_stakes.len()
        );
        prop_assert_eq!(protocol_fee + total_claimed + dust, total);

        if winner_stakes.len() == 1 {
            prop_assert_eq!(total_claimed, payout_pool);
        }
    }

    /// With a 100% fee nothing is left for winners, whatever the split.
    #[test]
    fn prop_full_fee_leaves_nothing_to_claim(
        user_stake in 1i128..=1_000_000_000i128,
        extra_winning_stake in 0i128..=1_000_000_000i128,
        losing_stake in 0i128..=1_000_000_000i128,
    ) {
        let winning_stake = user_stake + extra_winning_stake;
        let breakdown = calculate_claim_payout(&PayoutInput {
            pool_total_stake: winning_stake + losing_stake,
            fee_bps: 10_000,
            user_stake,
            winning_stake,
        }).unwrap();

        prop_assert_eq!(breakdown.payout_pool, 0);
        prop_assert_eq!(breakdown.winnings, 0);
        prop_assert_eq!(breakdown.protocol_fee, winning_stake + losing_stake);
    }

    /// Out-of-range fees and negative stakes are rejected rather than producing
    /// a payout.
    #[test]
    fn prop_claim_payout_rejects_invalid_inputs(
        bad_fee_bps in 10_001i128..=1_000_000i128,
        negative in i128::MIN..=-1i128,
        stake in 0i128..=1_000_000i128,
    ) {
        let valid = PayoutInput {
            pool_total_stake: stake + 1,
            fee_bps: 100,
            user_stake: stake,
            winning_stake: stake,
        };

        prop_assert_eq!(
            calculate_claim_payout(&PayoutInput { fee_bps: bad_fee_bps, ..valid }),
            Err(PrediFiError::InvalidFeeBps)
        );
        prop_assert_eq!(
            calculate_claim_payout(&PayoutInput { fee_bps: negative, ..valid }),
            Err(PrediFiError::InvalidFeeBps)
        );
        prop_assert_eq!(
            calculate_claim_payout(&PayoutInput { user_stake: negative, ..valid }),
            Err(PrediFiError::ArithmeticError)
        );
        prop_assert_eq!(
            calculate_claim_payout(&PayoutInput { pool_total_stake: negative, ..valid }),
            Err(PrediFiError::ArithmeticError)
        );
    }
}
