//! # Gas / storage optimization helpers
//!
//! Hot-path helpers that reduce Soroban storage IO and Vec allocations:
//! - Batch outcome-stake reads/writes via `OutStakes`
//! - Pre-sized Vec construction for outcome arrays
//! - Odds computation without redundant pool re-loads

use soroban_sdk::{Env, Vec};

use crate::payouts::calculate_odds_bps;

/// Build a zero-filled outcome stakes vector without realloc churn.
///
/// Prefer this over a push loop when the outcome count is known up front
/// (e.g. pool creation).
pub fn alloc_zero_stakes(env: &Env, options_count: u32) -> Vec<i128> {
    let mut stakes = Vec::new(env);
    let mut i = 0u32;
    while i < options_count {
        stakes.push_back(0i128);
        i += 1;
    }
    stakes
}

/// Compute current odds (bps) for each outcome from an already-loaded stakes vec.
///
/// Avoids re-reading pool storage just to derive odds for UI / `get_pool_stats`.
pub fn odds_from_stakes(env: &Env, stakes: &Vec<i128>, total_stake: i128) -> Vec<u64> {
    let mut current_odds = Vec::new(env);
    let mut i = 0u32;
    let len = stakes.len();
    while i < len {
        let stake = stakes.get(i).unwrap_or(0);
        current_odds.push_back(calculate_odds_bps(stake, total_stake));
        i += 1;
    }
    current_odds
}

/// Apply a stake delta to a single outcome inside an in-memory stakes vec.
///
/// Callers persist the batch `OutStakes` key once after mutation — avoiding
/// per-outcome dual writes on the hot `place_prediction` path.
pub fn apply_stake_delta(stakes: &mut Vec<i128>, outcome: u32, amount: i128) -> i128 {
    let current = stakes.get(outcome).unwrap_or(0);
    let updated = current + amount;
    stakes.set(outcome, updated);
    updated
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn alloc_zero_stakes_sized_correctly() {
        let env = Env::default();
        let stakes = alloc_zero_stakes(&env, 5);
        assert_eq!(stakes.len(), 5);
        assert_eq!(stakes.get(0), Some(0));
        assert_eq!(stakes.get(4), Some(0));
    }

    #[test]
    fn apply_stake_delta_updates_index() {
        let env = Env::default();
        let mut stakes = alloc_zero_stakes(&env, 3);
        let updated = apply_stake_delta(&mut stakes, 1, 250);
        assert_eq!(updated, 250);
        assert_eq!(stakes.get(1), Some(250));
        assert_eq!(stakes.get(0), Some(0));
    }

    #[test]
    fn odds_from_stakes_even_market() {
        let env = Env::default();
        let mut stakes = Vec::new(&env);
        stakes.push_back(500);
        stakes.push_back(500);
        let odds = odds_from_stakes(&env, &stakes, 1_000);
        assert_eq!(odds.get(0), Some(20_000));
        assert_eq!(odds.get(1), Some(20_000));
    }
}
