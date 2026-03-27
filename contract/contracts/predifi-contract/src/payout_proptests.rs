#[cfg(test)]
extern crate std;
#[cfg(test)]
use crate::safe_math::{RoundingMode, SafeMath};
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
}
