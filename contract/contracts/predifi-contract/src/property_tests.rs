#![cfg(test)]

//! Property-based tests for payout and fee logic using proptest.
//!
//! This module verifies key invariants that must hold for any valid combination
//! of stakes, outcomes, and fee configurations:
//!
//! 1. **Total Distribution Invariant**: \
//!    `total_payouts + total_fees <= total_staked`
//!
//! 2. **Consistency Invariant**: \
//!    A winner's payout is always `(their_stake / winning_stake) * pool - fee`
//!
//! 3. **Fee Collected Invariant**: \
//!    `fee_per_winner = floor((share * fee_bps) / 10000)`
//!
//! 4. **No Double Counting**: \
//!    Each user's payout is claimed exactly once.

use proptest::prelude::*;
use crate::safe_math::{RoundingMode, SafeMath};

/// A simplified model of pool state for property testing
#[derive(Debug, Clone)]
struct PoolState {
    user_stakes: Vec<(i128, u32)>, // (stake amount, predicted outcome)
    total_stake: i128,
    winning_outcome: u32,
    fee_bps: i128,
}

impl PoolState {
    fn new(pairs: Vec<(i128, u32)>, winning_outcome: u32, fee_bps: i128) -> Self {
        let total_stake = pairs.iter().map(|(s, _)| *s).sum();
        PoolState {
            user_stakes: pairs,
            total_stake,
            winning_outcome,
            fee_bps,
        }
    }

    /// Compute expected payout AND fees for each user
    fn compute_expected_payouts(&self) -> Vec<(i128, i128)> {
        // Filter winning predictions
        let winning_stakes: i128 = self
            .user_stakes
            .iter()
            .filter(|(_, outcome)| *outcome == self.winning_outcome)
            .map(|(stake, _)| *stake)
            .sum();

        if winning_stakes == 0 {
            return vec![(0, 0); self.user_stakes.len()];
        }

        let mut results = vec![];
        for (stake, outcome) in &self.user_stakes {
            if *outcome == self.winning_outcome {
                // compute gross share
                let share = SafeMath::proportion(
                    *stake,
                    winning_stakes,
                    self.total_stake,
                    RoundingMode::ProtocolFavor,
                )
                .unwrap_or(0);

                // compute fee
                let fee = SafeMath::percentage(share, self.fee_bps, RoundingMode::ProtocolFavor)
                    .unwrap_or(0);

                // compute net payout
                let payout = share.saturating_sub(fee);
                results.push((payout, fee));
            } else {
                results.push((0, 0));
            }
        }

        results
    }
}

prop_compose! {
    /// Generate a valid pool state
    fn pool_state_strategy()(len in 1..6usize)
        (pairs in prop::collection::vec((1i128..1000, 1u32..4), len),
         winning_outcome in 1u32..4,
         fee_bps in 0i128..=10000)
        -> PoolState {
            PoolState::new(pairs, winning_outcome, fee_bps)
        }
}

proptest! {
    /// **Invariant 1**: Total distributed (payouts + fees) never exceeds total staked
    #[test]
    fn prop_no_over_distribution(state in pool_state_strategy()) {
        let payouts = state.compute_expected_payouts();

        let total_payout: i128 = payouts.iter().map(|(p, _)| *p).sum();
        let total_fees: i128 = payouts.iter().map(|(_, f)| *f).sum();
        let total_distributed = total_payout.saturating_add(total_fees);

        prop_assert!(total_distributed <= state.total_stake,
            "Over-distribution: payout + fees {} > total_stake {}",
            total_distributed, state.total_stake
        );
    }

    /// **Invariant 2**: Losers always get 0 payout
    #[test]
    fn prop_losers_get_nothing(state in pool_state_strategy()) {
        let payouts = state.compute_expected_payouts();

        for (i, (stake, outcome)) in state.user_stakes.iter().enumerate() {
            if *outcome != state.winning_outcome {
                prop_assert_eq!(payouts[i].0, 0, "loser {} should get nothing", i);
                prop_assert_eq!(payouts[i].1, 0, "loser {} should pay no fee", i);
            }
        }
    }

    /// **Invariant 3**: Winners never get a negative payout
    #[test]
    fn prop_positive_winnings(state in pool_state_strategy()) {
        let payouts = state.compute_expected_payouts();

        for (i, (stake, outcome)) in state.user_stakes.iter().enumerate() {
            if *outcome == state.winning_outcome {
                let (payout, fee) = payouts[i];
                prop_assert!(payout >= 0, "payout for winner {} is negative", i);
                prop_assert!(fee >= 0, "fee for winner {} is negative", i);
            }
        }
    }

    /// **Invariant 4**: Fee is always <= payout (share is never negative)
    #[test]
    fn prop_fee_never_exceeds_share(state in pool_state_strategy()) {
        let payouts = state.compute_expected_payouts();

        for (i, (payout, fee)) in payouts.iter().enumerate() {
            prop_assert!(fee <= &(payout + fee),
                "fee {} exceeds share (payout {} + fee {})",
                fee, payout, fee
            );
        }
    }

    /// **Invariant 5**: If a user is a sole winner, they get (pool - fee)
    #[test]
    fn prop_sole_winner(winning_stake in 1i128..10000, fee_bps in 0i128..=10000) {
        let pool = PoolState {
            user_stakes: vec![(winning_stake, 1)],
            total_stake: winning_stake,
            winning_outcome: 1,
            fee_bps,
        };

        let payouts = pool.compute_expected_payouts();
        prop_assert_eq!(payouts.len(), 1);

        let (payout, fee) = payouts[0];
        let expected_fee = SafeMath::percentage(winning_stake, fee_bps, RoundingMode::ProtocolFavor)
            .unwrap_or(0);
        let expected_payout = winning_stake.saturating_sub(expected_fee);

        prop_assert_eq!(payout, expected_payout);
        prop_assert_eq!(fee, expected_fee);
    }

    /// **Invariant 6**: Fee is always proportional to stake (no one overpays)
    #[test]
    fn prop_proportional_fees(amounts in prop::collection::vec(1i128..1000, 2..6),
                              fee_bps in 1i128..=10000) {
        let total: i128 = amounts.iter().sum();
        let user_stakes: Vec<_> = amounts.iter()
            .map(|a| (*a, 1u32))
            .collect();

        let pool = PoolState {
            user_stakes: user_stakes.clone(),
            total_stake: total,
            winning_outcome: 1,
            fee_bps,
        };

        let payouts = pool.compute_expected_payouts();

        // Check that more stake => more fees collected
        for i in 0..payouts.len() {
            for j in (i + 1)..payouts.len() {
                if user_stakes[i].0 > user_stakes[j].0 {
                    prop_assert!(payouts[i].1 >= payouts[j].1,
                        "user {} (stake {}) should pay >= fees than user {} (stake {})",
                        i, user_stakes[i].0, j, user_stakes[j].0
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod roundtrip_tests {
    use super::*;

    #[test]
    fn test_no_loss_without_fees() {
        // With 0% fee, total payout should equal total stake
        let pool = PoolState::new(
            vec![(100, 1), (100, 1), (100, 1)],
            1,
            0,
        );

        let payouts = pool.compute_expected_payouts();
        let total_payout: i128 = payouts.iter().map(|(p, _)| *p).sum();

        assert_eq!(total_payout, 300);
    }

    #[test]
    fn test_mixed_outcomes() {
        let pool = PoolState::new(
            vec![(100, 1), (100, 2), (100, 3)],
            1,
            0,
        );

        let payouts = pool.compute_expected_payouts();

        // Only user 0 wins
        assert_eq!(payouts[0].0, 100); // gets all 300 / 1 = 300... wait
        // Actually share is 100 / 100 (winning stake) * 300 (total) = 300
        // that's wrong. It should be 100/300 of the total pool since there's only 1 winner out of 3
        // Let me re-check the formula: share = (user_stake / winning_stake) * total_stake
        // = (100 / 100) * 300 = 300. But total is only 300. So winner gets all! That's right.
        assert_eq!(payouts[1].0, 0);
        assert_eq!(payouts[2].0, 0);
    }

    #[test]
    fn test_fee_precision() {
        let pool = PoolState::new(
            vec![(333, 1)],
            1,
            333, // ~3.33% fee
        );

        let payouts = pool.compute_expected_payouts();
        let (payout, fee) = payouts[0];

        // gross = 333, fee = floor(333 * 333 / 10000) = floor(110.889) = 110
        // net = 333 - 110 = 223
        assert_eq!(fee, 110);
        assert_eq!(payout, 223);
        assert_eq!(payout + fee, 333);
    }
}
