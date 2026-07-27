//! # Payout calculation module
//!
//! Pure payout math extracted from the main contract so `lib.rs` stays focused
//! on storage, auth, and orchestration. All functions are `no_std`-safe and
//! rely on [`SafeMath`] for overflow-checked arithmetic.
//!
//! ## Invariants
//!
//! - **INV-4**: Winnings never exceed `pool_total_stake`
//! - Protocol fee uses `RoundingMode::ProtocolFavor`
//! - Winner share is proportional: `(user_stake * payout_pool) / winning_stake`

use crate::safe_math::{RoundingMode, SafeMath};
use predifi_errors::PrediFiError;

/// Inputs required to compute a single winner's payout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayoutInput {
    /// Total stake locked in the pool (all outcomes).
    pub pool_total_stake: i128,
    /// Protocol fee in basis points (0–10_000).
    pub fee_bps: i128,
    /// Caller's stake on the winning outcome.
    pub user_stake: i128,
    /// Aggregate stake on the winning outcome.
    pub winning_stake: i128,
}

/// Fully computed payout breakdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayoutBreakdown {
    pub protocol_fee: i128,
    pub payout_pool: i128,
    pub winnings: i128,
}

/// Calculate protocol fee from total stake and fee bps (protocol-favor rounding).
pub fn calculate_protocol_fee(total_stake: i128, fee_bps: i128) -> Result<i128, PrediFiError> {
    SafeMath::percentage(total_stake, fee_bps, RoundingMode::ProtocolFavor)
}

/// Amount remaining for winners after protocol fee is deducted.
pub fn calculate_payout_pool(total_stake: i128, fee_bps: i128) -> Result<i128, PrediFiError> {
    let fee = calculate_protocol_fee(total_stake, fee_bps)?;
    total_stake
        .checked_sub(fee)
        .ok_or(PrediFiError::ArithmeticError)
}

/// Proportional winnings for one winner.
///
/// Returns `0` when there is no winning stake or the user staked nothing.
pub fn calculate_winnings(
    user_stake: i128,
    winning_stake: i128,
    payout_pool: i128,
) -> Result<i128, PrediFiError> {
    SafeMath::calculate_share(user_stake, winning_stake, payout_pool)
}

/// Full payout for a claim: fee → payout pool → user share.
pub fn calculate_claim_payout(input: &PayoutInput) -> Result<PayoutBreakdown, PrediFiError> {
    if input.pool_total_stake < 0 || input.user_stake < 0 || input.winning_stake < 0 {
        return Err(PrediFiError::ArithmeticError);
    }
    if input.fee_bps < 0 || input.fee_bps > 10_000 {
        return Err(PrediFiError::InvalidFeeBps);
    }

    let protocol_fee = calculate_protocol_fee(input.pool_total_stake, input.fee_bps)?;
    let payout_pool = input
        .pool_total_stake
        .checked_sub(protocol_fee)
        .ok_or(PrediFiError::ArithmeticError)?;

    let winnings = if input.winning_stake == 0 || input.user_stake == 0 {
        0
    } else {
        calculate_winnings(input.user_stake, input.winning_stake, payout_pool)?
    };

    // INV-4: no value creation
    if winnings > input.pool_total_stake {
        return Err(PrediFiError::RewardError);
    }

    Ok(PayoutBreakdown {
        protocol_fee,
        payout_pool,
        winnings,
    })
}

/// Referral cut taken from the claimer's proportional share of the protocol fee.
///
/// `protocol_fee_share` is the portion of the total protocol fee attributable to
/// this user's stake; `referral_cut_bps` is the configured referral percentage.
pub fn calculate_referral_amount(
    user_stake: i128,
    pool_total_stake: i128,
    protocol_fee_total: i128,
    referral_cut_bps: i128,
) -> Result<i128, PrediFiError> {
    if protocol_fee_total <= 0 || pool_total_stake <= 0 || user_stake <= 0 {
        return Ok(0);
    }
    let protocol_fee_share = SafeMath::proportion(
        user_stake,
        pool_total_stake,
        protocol_fee_total,
        RoundingMode::Neutral,
    )?;
    SafeMath::percentage(protocol_fee_share, referral_cut_bps, RoundingMode::Neutral)
}

/// Compute odds in basis points: `(total_stake * 10_000) / outcome_stake`.
///
/// Returns `0` when the outcome has no stake or total stake is non-positive.
pub fn calculate_odds_bps(outcome_stake: i128, total_stake: i128) -> u64 {
    if outcome_stake <= 0 || total_stake <= 0 {
        return 0;
    }
    total_stake
        .checked_mul(10_000)
        .and_then(|v| v.checked_div(outcome_stake))
        .unwrap_or(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sole_winner_receives_full_payout_pool() {
        let input = PayoutInput {
            pool_total_stake: 1_000,
            fee_bps: 100, // 1%
            user_stake: 400,
            winning_stake: 400,
        };
        let out = calculate_claim_payout(&input).unwrap();
        assert_eq!(out.protocol_fee, 10);
        assert_eq!(out.payout_pool, 990);
        assert_eq!(out.winnings, 990);
    }

    #[test]
    fn proportional_split_among_winners() {
        let input = PayoutInput {
            pool_total_stake: 1_000,
            fee_bps: 0,
            user_stake: 250,
            winning_stake: 500,
        };
        let out = calculate_claim_payout(&input).unwrap();
        assert_eq!(out.winnings, 500); // half of payout pool
    }

    #[test]
    fn zero_winning_stake_yields_zero() {
        let input = PayoutInput {
            pool_total_stake: 1_000,
            fee_bps: 100,
            user_stake: 100,
            winning_stake: 0,
        };
        let out = calculate_claim_payout(&input).unwrap();
        assert_eq!(out.winnings, 0);
    }

    #[test]
    fn rejects_fee_above_max_bps() {
        let input = PayoutInput {
            pool_total_stake: 1_000,
            fee_bps: 10_001,
            user_stake: 100,
            winning_stake: 100,
        };
        assert_eq!(
            calculate_claim_payout(&input),
            Err(PrediFiError::InvalidFeeBps)
        );
    }

    #[test]
    fn rejects_negative_stakes() {
        let input = PayoutInput {
            pool_total_stake: -1,
            fee_bps: 100,
            user_stake: 100,
            winning_stake: 100,
        };
        assert_eq!(
            calculate_claim_payout(&input),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn referral_cut_is_proportional() {
        // User has 50% of pool; fee total = 100; referral cut = 10% → 5
        let amount = calculate_referral_amount(500, 1_000, 100, 1_000).unwrap();
        assert_eq!(amount, 5);
    }

    #[test]
    fn referral_zero_when_no_fee() {
        assert_eq!(calculate_referral_amount(500, 1_000, 0, 1_000).unwrap(), 0);
    }

    #[test]
    fn odds_bps_even_split() {
        // 50/50 → odds = 2.0x = 20_000 bps
        assert_eq!(calculate_odds_bps(500, 1_000), 20_000);
    }

    #[test]
    fn odds_bps_zero_stake() {
        assert_eq!(calculate_odds_bps(0, 1_000), 0);
    }

    #[test]
    fn payout_pool_subtracts_fee() {
        assert_eq!(calculate_payout_pool(10_000, 250).unwrap(), 9_750);
    }
}
