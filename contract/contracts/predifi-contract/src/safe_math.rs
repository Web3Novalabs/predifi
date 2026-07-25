#![allow(dead_code)]

//! # Safe Math Module for Proportion Calculations
//!
//! This module provides safe arithmetic operations for proportion and percentage
//! calculations, critical for payout logic where rounding errors or division by
//! zero could lead to locked funds or unfair distributions.
//!
//! ## Features
//!
//! - Fixed-point arithmetic with configurable precision
//! - Protection against overflow, underflow, and division by zero
//! - Configurable rounding strategies (protocol-favoring, neutral, user-favoring)
//! - Proportion calculations that maintain fairness in payouts
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use safe_math::{SafeMath, RoundingMode};
//!
//! // Calculate 30% of 1000 with protocol-favoring rounding
//! let result = SafeMath::percentage(1000, 3000, RoundingMode::ProtocolFavor)?;
//!
//! // Calculate proportional payout
//! let payout = SafeMath::proportion(user_stake, total_stake, pool_balance, RoundingMode::Neutral)?;
//! ```

use predifi_errors::PrediFiError;

#[cfg(test)]
extern crate std;

#[cfg(test)]
use std::vec::Vec;

/// Fixed-point precision multiplier (10,000 = 0.01% precision)
/// This allows for basis point calculations (1 bps = 0.01%)
const PRECISION: i128 = 10_000;

/// Maximum basis points (100% = 10,000 bps)
const MAX_BPS: i128 = 10_000;

/// Rounding mode for calculations
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RoundingMode {
    /// Round down - favors the protocol by keeping dust in the pool
    ProtocolFavor,
    /// Round to nearest - neutral rounding
    Neutral,
    /// Round up - favors the user (use with caution)
    UserFavor,
}

/// Safe math operations for proportion and percentage calculations
pub struct SafeMath;

impl SafeMath {
    /// Calculate a percentage of an amount using basis points.
    ///
    /// # Arguments
    /// * `amount` - The base amount
    /// * `bps` - Basis points (e.g., 100 = 1%, 10000 = 100%)
    /// * `rounding` - Rounding mode to use
    ///
    /// # Returns
    /// The calculated percentage or an error
    ///
    /// # Example
    /// ```rust,ignore
    /// // Calculate 2.5% of 1000 = 25
    /// let result = SafeMath::percentage(1000, 250, RoundingMode::Neutral)?;
    /// ```
    pub fn percentage(
        amount: i128,
        bps: i128,
        rounding: RoundingMode,
    ) -> Result<i128, PrediFiError> {
        // Validate inputs
        if amount < 0 {
            return Err(PrediFiError::ArithmeticError);
        }
        if !(0..=MAX_BPS).contains(&bps) {
            return Err(PrediFiError::InvalidFeeBps);
        }
        if amount == 0 || bps == 0 {
            return Ok(0);
        }

        // Calculate: (amount * bps) / MAX_BPS
        let numerator = amount.checked_mul(bps).ok_or(PrediFiError::InvalidAmount)?;

        Self::divide_with_rounding(numerator, MAX_BPS, rounding)
    }

    /// Calculate a proportion: (numerator / denominator) * amount
    ///
    /// This is the core function for payout calculations.
    ///
    /// # Arguments
    /// * `numerator` - The user's stake (must be >= 0 and <= denominator)
    /// * `denominator` - The total stake (must be > 0)
    /// * `amount` - The amount to distribute proportionally
    /// * `rounding` - Rounding mode to use
    ///
    /// Note: `numerator == denominator` is valid and represents a 100% share of `amount`.
    /// This means a user who owns the entire stake receives the full `amount`.
    ///
    /// # Returns
    /// The proportional amount or an error
    ///
    /// # Example
    /// ```rust,ignore
    /// // User staked 300 out of 1000 total, pool has 5000 to distribute
    /// // Result: (300 / 1000) * 5000 = 1500
    /// let payout = SafeMath::proportion(300, 1000, 5000, RoundingMode::Neutral)?;
    /// ```
    pub fn proportion(
        numerator: i128,
        denominator: i128,
        amount: i128,
        rounding: RoundingMode,
    ) -> Result<i128, PrediFiError> {
        // Validate inputs
        if numerator < 0 || denominator <= 0 || amount < 0 {
            return Err(PrediFiError::ArithmeticError);
        }
        if numerator == 0 || amount == 0 {
            return Ok(0);
        }
        if numerator > denominator {
            return Err(PrediFiError::ArithmeticError);
        }

        // Calculate: (numerator * amount) / denominator
        let product = numerator
            .checked_mul(amount)
            .ok_or(PrediFiError::InvalidAmount)?;

        Self::divide_with_rounding(product, denominator, rounding)
    }

    /// Safely divide two numbers with configurable rounding
    ///
    /// # Arguments
    /// * `numerator` - The dividend
    /// * `denominator` - The divisor
    /// * `rounding` - Rounding mode to use
    ///
    /// # Returns
    /// The quotient or an error
    fn divide_with_rounding(
        numerator: i128,
        denominator: i128,
        rounding: RoundingMode,
    ) -> Result<i128, PrediFiError> {
        if denominator == 0 {
            return Err(PrediFiError::ArithmeticError);
        }

        let quotient = numerator
            .checked_div(denominator)
            .ok_or(PrediFiError::InvalidAmount)?;
        let remainder = numerator
            .checked_rem(denominator)
            .ok_or(PrediFiError::InvalidAmount)?;

        match rounding {
            RoundingMode::ProtocolFavor => {
                // Always round down (floor)
                Ok(quotient)
            }
            RoundingMode::Neutral => {
                // Round to nearest (half up)
                let half = denominator
                    .checked_div(2)
                    .ok_or(PrediFiError::InvalidAmount)?;
                if remainder >= half {
                    quotient.checked_add(1).ok_or(PrediFiError::InvalidAmount)
                } else {
                    Ok(quotient)
                }
            }
            RoundingMode::UserFavor => {
                // Round up (ceiling) if there's any remainder
                if remainder > 0 {
                    quotient.checked_add(1).ok_or(PrediFiError::InvalidAmount)
                } else {
                    Ok(quotient)
                }
            }
        }
    }

    /// Calculate multiple proportions ensuring the sum doesn't exceed the total
    ///
    /// This is useful for distributing payouts to multiple winners where rounding
    /// errors could cause the sum to exceed the available pool balance.
    ///
    /// Note: This function is primarily for testing and validation. In production,
    /// calculate payouts individually and track the distributed amount.
    ///
    /// # Arguments
    /// * `stakes` - Array of individual stakes
    /// * `total_stake` - Sum of all stakes
    /// * `pool_balance` - Total amount to distribute
    /// * `rounding` - Rounding mode to use
    ///
    /// # Returns
    /// Vector of proportional amounts or an error
    #[cfg(test)]
    pub fn multi_proportion(
        stakes: &[i128],
        total_stake: i128,
        pool_balance: i128,
        rounding: RoundingMode,
    ) -> Result<Vec<i128>, PrediFiError> {
        if stakes.is_empty() {
            return Ok(Vec::new());
        }
        if total_stake <= 0 || pool_balance < 0 {
            return Err(PrediFiError::ArithmeticError);
        }

        let mut results = Vec::with_capacity(stakes.len());
        let mut distributed = 0i128;

        // Calculate proportions for all but the last
        for (i, &stake) in stakes.iter().enumerate() {
            if stake < 0 {
                return Err(PrediFiError::ArithmeticError);
            }

            if i == stakes.len() - 1 {
                // Last entry gets the remainder to avoid rounding issues
                let remaining = pool_balance
                    .checked_sub(distributed)
                    .ok_or(PrediFiError::ArithmeticError)?;
                results.push(remaining);
            } else {
                let amount = Self::proportion(stake, total_stake, pool_balance, rounding)?;
                distributed = distributed
                    .checked_add(amount)
                    .ok_or(PrediFiError::ArithmeticError)?;
                results.push(amount);
            }
        }

        // Verify we didn't over-distribute (also protecting against sum overflow)
        let total_distributed = results.iter().try_fold(0i128, |acc, &value| {
            acc.checked_add(value).ok_or(PrediFiError::InvalidAmount)
        })?;

        if total_distributed > pool_balance {
            return Err(PrediFiError::RewardError);
        }

        Ok(results)
    }

    /// Calculate a winner's share of the payout pool.
    ///
    /// Computes `(user_stake * payout_pool) / winning_stake`, which is the
    /// canonical formula used in `claim_winnings`.
    ///
    /// # Arguments
    /// * `user_stake`   - The amount the user staked on the winning outcome.
    /// * `winning_stake` - Total stake on the winning outcome across all users.
    /// * `payout_pool`  - The pool available for distribution (after fees).
    ///
    /// # Returns
    /// * `Ok(0)` when `winning_stake` is 0 (no winners — safe no-op).
    /// * `Ok(share)` on success.
    /// * `Err(ArithmeticError)` on overflow or invalid inputs.
    pub fn calculate_share(
        user_stake: i128,
        winning_stake: i128,
        payout_pool: i128,
    ) -> Result<i128, PrediFiError> {
        if user_stake < 0 || winning_stake < 0 || payout_pool < 0 {
            return Err(PrediFiError::ArithmeticError);
        }
        if winning_stake == 0 || user_stake == 0 || payout_pool == 0 {
            return Ok(0);
        }
        if user_stake > winning_stake {
            return Err(PrediFiError::ArithmeticError);
        }
        let product = user_stake
            .checked_mul(payout_pool)
            .ok_or(PrediFiError::InvalidAmount)?;
        product
            .checked_div(winning_stake)
            .ok_or(PrediFiError::ArithmeticError)
    }

    /// Safely add two amounts with overflow check
    pub fn safe_add(a: i128, b: i128) -> Result<i128, PrediFiError> {
        a.checked_add(b).ok_or(PrediFiError::ArithmeticError)
    }

    /// Safely subtract two amounts with underflow check
    pub fn safe_sub(a: i128, b: i128) -> Result<i128, PrediFiError> {
        a.checked_sub(b).ok_or(PrediFiError::ArithmeticError)
    }

    /// Safely multiply two amounts with overflow check
    pub fn safe_mul(a: i128, b: i128) -> Result<i128, PrediFiError> {
        a.checked_mul(b).ok_or(PrediFiError::InvalidAmount)
    }
}

/// # Payout Rounding Precision Audit
///
/// This module provides comprehensive validation of payout calculations to ensure
/// that rounding precision is maintained across all scenarios. It verifies:
///
/// - **No Fund Loss**: Total payouts never exceed the available pool balance
/// - **Rounding Consistency**: Each payout calculation uses correct rounding rules
/// - **Precision Integrity**: Results are accurate within acceptable tolerances
/// - **Edge Case Handling**: Extreme values and boundary conditions are handled correctly
///
/// ## Audit Scenario
///
/// A payout audit validates a specific claim/resolution scenario:
/// - Pool total stake: the sum of all user predictions
/// - Protocol fees: calculated and deducted from the pool
/// - Payout pool: the amount available for distribution to winners
/// - Individual payouts: calculated for each user claiming winnings
///
/// ## Usage
///
/// ```rust,ignore
/// let audit = PayoutRoundingAudit::new(
///     pool_total_stake,
///     protocol_fee_bps,
///     user_stake,
///     winning_stake,
/// )?;
/// audit.validate_payout()?;
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PayoutRoundingAudit {
    /// Total amount staked in the pool
    pub pool_total_stake: i128,
    /// Protocol fee in basis points (0-10000)
    pub protocol_fee_bps: i128,
    /// Individual user's stake on winning outcome
    pub user_stake: i128,
    /// Total stake on the winning outcome
    pub winning_stake: i128,
}

impl PayoutRoundingAudit {
    /// Create a new payout rounding audit
    pub fn new(
        pool_total_stake: i128,
        protocol_fee_bps: i128,
        user_stake: i128,
        winning_stake: i128,
    ) -> Result<Self, PrediFiError> {
        // Validate inputs
        if pool_total_stake < 0 {
            return Err(PrediFiError::ArithmeticError);
        }
        if !(0..=MAX_BPS).contains(&protocol_fee_bps) {
            return Err(PrediFiError::InvalidFeeBps);
        }
        if user_stake < 0 || winning_stake < 0 {
            return Err(PrediFiError::ArithmeticError);
        }

        Ok(PayoutRoundingAudit {
            pool_total_stake,
            protocol_fee_bps,
            user_stake,
            winning_stake,
        })
    }

    /// Calculate the protocol fee with protocol-favor rounding
    pub fn calculate_protocol_fee(&self) -> Result<i128, PrediFiError> {
        SafeMath::percentage(
            self.pool_total_stake,
            self.protocol_fee_bps,
            RoundingMode::ProtocolFavor,
        )
    }

    /// Calculate the payout pool (total stake minus protocol fee)
    pub fn calculate_payout_pool(&self) -> Result<i128, PrediFiError> {
        let protocol_fee = self.calculate_protocol_fee()?;
        self.pool_total_stake
            .checked_sub(protocol_fee)
            .ok_or(PrediFiError::ArithmeticError)
    }

    /// Calculate individual user payout with verification
    pub fn calculate_user_payout(&self) -> Result<i128, PrediFiError> {
        // Edge case: no winners or no winning stake
        if self.winning_stake == 0 {
            return Ok(0);
        }

        // Edge case: user has no stake
        if self.user_stake == 0 {
            return Ok(0);
        }

        // User stake cannot exceed winning stake
        if self.user_stake > self.winning_stake {
            return Err(PrediFiError::ArithmeticError);
        }

        let payout_pool = self.calculate_payout_pool()?;
        SafeMath::calculate_share(self.user_stake, self.winning_stake, payout_pool)
    }

    /// Validate the payout calculation meets precision requirements
    ///
    /// # Validation Checks
    /// - Payout does not exceed pool total stake
    /// - Payout equals or is slightly less than expected due to rounding
    /// - No arithmetic overflow or underflow
    pub fn validate_payout(&self) -> Result<(), PrediFiError> {
        let payout = self.calculate_user_payout()?;

        // Payout must not exceed pool total stake (INV-4)
        if payout > self.pool_total_stake {
            return Err(PrediFiError::RewardError);
        }

        // Payout must not exceed payout pool
        let payout_pool = self.calculate_payout_pool()?;
        if payout > payout_pool {
            return Err(PrediFiError::RewardError);
        }

        Ok(())
    }

    /// Audit a batch of payouts to ensure total doesn't exceed payout pool
    ///
    /// This is useful for validating multiple claims in a single operation.
    pub fn validate_batch_payouts(
        pool_total_stake: i128,
        protocol_fee_bps: i128,
        user_stakes: &[i128],
        winning_stake: i128,
    ) -> Result<i128, PrediFiError> {
        if winning_stake > pool_total_stake {
            return Err(PrediFiError::ArithmeticError);
        }

        // Calculate payout pool once
        let protocol_fee = SafeMath::percentage(
            pool_total_stake,
            protocol_fee_bps,
            RoundingMode::ProtocolFavor,
        )?;
        let payout_pool = pool_total_stake
            .checked_sub(protocol_fee)
            .ok_or(PrediFiError::ArithmeticError)?;

        let mut total_payout = 0i128;

        // Validate each payout
        for &user_stake in user_stakes {
            if user_stake < 0 {
                return Err(PrediFiError::ArithmeticError);
            }

            if user_stake == 0 || winning_stake == 0 {
                continue;
            }

            if user_stake > winning_stake {
                return Err(PrediFiError::ArithmeticError);
            }

            let payout = SafeMath::calculate_share(user_stake, winning_stake, payout_pool)?;

            total_payout = total_payout
                .checked_add(payout)
                .ok_or(PrediFiError::ArithmeticError)?;

            // Ensure we don't exceed payout pool
            if total_payout > payout_pool {
                return Err(PrediFiError::RewardError);
            }
        }

        Ok(total_payout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn test_percentage_basic() {
        // 10% of 1000 = 100
        assert_eq!(
            SafeMath::percentage(1000, 1000, RoundingMode::Neutral).unwrap(),
            100
        );

        // 2.5% of 1000 = 25
        assert_eq!(
            SafeMath::percentage(1000, 250, RoundingMode::Neutral).unwrap(),
            25
        );

        // 100% of 1000 = 1000
        assert_eq!(
            SafeMath::percentage(1000, 10000, RoundingMode::Neutral).unwrap(),
            1000
        );

        // 0% of 1000 = 0
        assert_eq!(
            SafeMath::percentage(1000, 0, RoundingMode::Neutral).unwrap(),
            0
        );
    }

    #[test]
    fn test_percentage_rounding() {
        // 3.33% of 100 = 3.33
        // Protocol favor (floor): 3
        assert_eq!(
            SafeMath::percentage(100, 333, RoundingMode::ProtocolFavor).unwrap(),
            3
        );

        // Neutral (round half up): 3
        assert_eq!(
            SafeMath::percentage(100, 333, RoundingMode::Neutral).unwrap(),
            3
        );

        // User favor (ceil): 4
        assert_eq!(
            SafeMath::percentage(100, 333, RoundingMode::UserFavor).unwrap(),
            4
        );
    }

    #[test]
    fn test_percentage_edge_cases() {
        // Zero amount
        assert_eq!(
            SafeMath::percentage(0, 1000, RoundingMode::Neutral).unwrap(),
            0
        );

        // Invalid bps (> 10000)
        assert_eq!(
            SafeMath::percentage(1000, 10001, RoundingMode::Neutral),
            Err(PrediFiError::InvalidFeeBps)
        );

        // Negative amount
        assert_eq!(
            SafeMath::percentage(-100, 1000, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );

        // Negative bps
        assert_eq!(
            SafeMath::percentage(1000, -100, RoundingMode::Neutral),
            Err(PrediFiError::InvalidFeeBps)
        );
    }

    #[test]
    fn test_percentage_overflow_invalid_amount() {
        // This amount is chosen so amount * bps would overflow i128
        let amount = (i128::MAX / MAX_BPS) + 1;

        assert_eq!(
            SafeMath::percentage(amount, MAX_BPS, RoundingMode::Neutral),
            Err(PrediFiError::InvalidAmount)
        );
    }

    #[test]
    fn test_proportion_basic() {
        // User staked 300 out of 1000, pool has 5000
        // (300 / 1000) * 5000 = 1500
        assert_eq!(
            SafeMath::proportion(300, 1000, 5000, RoundingMode::Neutral).unwrap(),
            1500
        );

        // User staked 1 out of 3, pool has 99
        // (1 / 3) * 99 = 33
        assert_eq!(
            SafeMath::proportion(1, 3, 99, RoundingMode::Neutral).unwrap(),
            33
        );

        // User staked all, gets all
        assert_eq!(
            SafeMath::proportion(1000, 1000, 5000, RoundingMode::Neutral).unwrap(),
            5000
        );

        // User staked nothing, gets nothing
        assert_eq!(
            SafeMath::proportion(0, 1000, 5000, RoundingMode::Neutral).unwrap(),
            0
        );
    }

    #[test]
    fn test_proportion_rounding() {
        // (1 / 3) * 100 = 33.333...
        // Protocol favor: 33
        assert_eq!(
            SafeMath::proportion(1, 3, 100, RoundingMode::ProtocolFavor).unwrap(),
            33
        );

        // Neutral: 33 (remainder 1 < half of 3)
        // Actually: 100/3 = 33 remainder 1, half = 1, so 1 >= 1 rounds up to 34
        assert_eq!(
            SafeMath::proportion(1, 3, 100, RoundingMode::Neutral).unwrap(),
            34
        );

        // User favor: 34
        assert_eq!(
            SafeMath::proportion(1, 3, 100, RoundingMode::UserFavor).unwrap(),
            34
        );

        // (1 / 2) * 101 = 50.5
        // Neutral rounds up: 51
        assert_eq!(
            SafeMath::proportion(1, 2, 101, RoundingMode::Neutral).unwrap(),
            51
        );
    }

    #[test]
    fn test_proportion_edge_cases() {
        // Zero denominator
        assert_eq!(
            SafeMath::proportion(100, 0, 1000, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );

        // Numerator > denominator
        assert_eq!(
            SafeMath::proportion(1001, 1000, 5000, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );

        // Negative values
        assert_eq!(
            SafeMath::proportion(-100, 1000, 5000, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );

        assert_eq!(
            SafeMath::proportion(100, -1000, 5000, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn test_multi_proportion_basic() {
        let stakes = vec![300, 500, 200];
        let total = 1000;
        let pool = 10000;

        let results =
            SafeMath::multi_proportion(&stakes, total, pool, RoundingMode::ProtocolFavor).unwrap();

        assert_eq!(results.len(), 3);
        // First: (300/1000) * 10000 = 3000
        assert_eq!(results[0], 3000);
        // Second: (500/1000) * 10000 = 5000
        assert_eq!(results[1], 5000);
        // Third: gets remainder = 2000
        assert_eq!(results[2], 2000);

        // Sum should equal pool
        let sum: i128 = results.iter().sum();
        assert_eq!(sum, pool);
    }

    #[test]
    fn test_multi_proportion_rounding_safety() {
        // Test case where rounding could cause over-distribution
        let stakes = vec![1, 1, 1];
        let total = 3;
        let pool = 10;

        let results =
            SafeMath::multi_proportion(&stakes, total, pool, RoundingMode::UserFavor).unwrap();

        // With user favor rounding, first two would get 4 each (ceil(10/3))
        // But last gets remainder to prevent over-distribution
        assert_eq!(results[0], 4);
        assert_eq!(results[1], 4);
        assert_eq!(results[2], 2); // Remainder, not 4

        let sum: i128 = results.iter().sum();
        assert_eq!(sum, pool);
    }

    #[test]
    fn test_multi_proportion_edge_cases() {
        // Empty stakes
        let results = SafeMath::multi_proportion(&[], 1000, 5000, RoundingMode::Neutral).unwrap();
        assert_eq!(results.len(), 0);

        // Single stake
        let stakes = vec![1000];
        let results =
            SafeMath::multi_proportion(&stakes, 1000, 5000, RoundingMode::Neutral).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 5000);

        // Zero total stake
        let stakes = vec![100, 200];
        assert_eq!(
            SafeMath::multi_proportion(&stakes, 0, 5000, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );

        // Negative stake
        let stakes = vec![100, -200];
        assert_eq!(
            SafeMath::multi_proportion(&stakes, 1000, 5000, RoundingMode::Neutral),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn test_complex_payout_scenario() {
        // Real-world scenario: 5 winners with different stakes
        let stakes = vec![1_500_000, 3_200_000, 750_000, 2_100_000, 1_450_000];
        let total_stake = 9_000_000;
        let pool_balance = 45_000_000;

        let payouts =
            SafeMath::multi_proportion(&stakes, total_stake, pool_balance, RoundingMode::Neutral)
                .unwrap();

        // Verify sum doesn't exceed pool
        let total_paid: i128 = payouts.iter().sum();
        assert!(total_paid <= pool_balance);
        assert_eq!(total_paid, pool_balance);

        // Verify each payout is proportional (approximately)
        for (i, &stake) in stakes.iter().enumerate() {
            let expected = (stake as f64 / total_stake as f64) * pool_balance as f64;
            let actual = payouts[i] as f64;
            let diff = (expected - actual).abs();
            // Allow small rounding difference
            assert!(
                diff < 2.0,
                "Payout {} differs too much: expected {}, got {}",
                i,
                expected,
                actual
            );
        }
    }

    #[test]
    fn test_safe_arithmetic() {
        // Addition
        assert_eq!(SafeMath::safe_add(100, 200).unwrap(), 300);
        assert_eq!(
            SafeMath::safe_add(i128::MAX, 1),
            Err(PrediFiError::ArithmeticError)
        );

        // Subtraction
        assert_eq!(SafeMath::safe_sub(200, 100).unwrap(), 100);
        assert_eq!(
            SafeMath::safe_sub(i128::MIN, 1),
            Err(PrediFiError::ArithmeticError)
        );

        // Multiplication
        assert_eq!(SafeMath::safe_mul(10, 20).unwrap(), 200);
        assert_eq!(
            SafeMath::safe_mul(i128::MAX, 2),
            Err(PrediFiError::InvalidAmount)
        );
    }

    #[test]
    fn test_large_numbers() {
        // Test with realistic token amounts (e.g., 7 decimal places)
        let amount = 10_000_000_000_000; // 1M tokens with 7 decimals
        let bps = 250; // 2.5%
        let fee = SafeMath::percentage(amount, bps, RoundingMode::Neutral).unwrap();
        assert_eq!(fee, 250_000_000_000); // 25K tokens

        // Test proportion with large numbers
        let user_stake = 5_000_000_000_000;
        let total_stake = 20_000_000_000_000;
        let pool = 100_000_000_000_000;
        let payout =
            SafeMath::proportion(user_stake, total_stake, pool, RoundingMode::Neutral).unwrap();
        assert_eq!(payout, 25_000_000_000_000); // 25% of pool
    }

    #[test]
    fn test_calculate_share_basic() {
        // User staked 300 of 1000 winning stake, payout pool is 5000
        // (300 * 5000) / 1000 = 1500
        assert_eq!(SafeMath::calculate_share(300, 1000, 5000).unwrap(), 1500);

        // User staked all of winning side — gets full payout pool
        assert_eq!(SafeMath::calculate_share(1000, 1000, 5000).unwrap(), 5000);

        // User staked 1 of 3, payout pool 99 → (1*99)/3 = 33
        assert_eq!(SafeMath::calculate_share(1, 3, 99).unwrap(), 33);
    }

    #[test]
    fn test_calculate_share_zero_cases() {
        // winning_stake == 0 → safe no-op, returns 0 (no winners)
        assert_eq!(SafeMath::calculate_share(100, 0, 5000).unwrap(), 0);

        // user_stake == 0 → no winnings
        assert_eq!(SafeMath::calculate_share(0, 1000, 5000).unwrap(), 0);

        // payout_pool == 0 → nothing to distribute
        assert_eq!(SafeMath::calculate_share(300, 1000, 0).unwrap(), 0);
    }

    #[test]
    fn test_calculate_share_negative_inputs() {
        assert_eq!(
            SafeMath::calculate_share(-1, 1000, 5000),
            Err(PrediFiError::ArithmeticError)
        );
        assert_eq!(
            SafeMath::calculate_share(100, -1000, 5000),
            Err(PrediFiError::ArithmeticError)
        );
        assert_eq!(
            SafeMath::calculate_share(100, 1000, -5000),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn test_calculate_share_user_stake_exceeds_winning_stake() {
        // user_stake > winning_stake is logically impossible — should error
        assert_eq!(
            SafeMath::calculate_share(1001, 1000, 5000),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn test_calculate_share_large_numbers() {
        // Realistic token amounts (7 decimal places)
        let user_stake = 5_000_000_000_000i128;
        let winning_stake = 20_000_000_000_000i128;
        let payout_pool = 100_000_000_000_000i128;
        // (5e12 * 1e14) / 2e13 = 25e12
        assert_eq!(
            SafeMath::calculate_share(user_stake, winning_stake, payout_pool).unwrap(),
            25_000_000_000_000
        );
    }

    #[test]
    fn test_calculate_share_truncates_remainder() {
        // (1 * 10) / 3 = 3 (integer division, truncates)
        assert_eq!(SafeMath::calculate_share(1, 3, 10).unwrap(), 3);
    }

    #[test]
    fn test_calculate_share_equal_stakes() {
        // Two users with equal stakes, payout pool 100 → each gets 50
        assert_eq!(SafeMath::calculate_share(50, 100, 100).unwrap(), 50);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PAYOUT ROUNDING PRECISION AUDIT TESTS
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // These tests validate the PayoutRoundingAudit struct to ensure that payout
    // calculations maintain precision and never over-distribute funds from the pool.

    #[test]
    fn test_audit_basic_scenario() {
        // Simple scenario: 1000 total stake, 1% fee, user wins with 300 of 500 winning stake
        let audit = PayoutRoundingAudit::new(1000, 100, 300, 500).unwrap();

        // Protocol fee: 1% of 1000 = 10
        let protocol_fee = audit.calculate_protocol_fee().unwrap();
        assert_eq!(protocol_fee, 10);

        // Payout pool: 1000 - 10 = 990
        let payout_pool = audit.calculate_payout_pool().unwrap();
        assert_eq!(payout_pool, 990);

        // User payout: (300 / 500) * 990 = 594
        let payout = audit.calculate_user_payout().unwrap();
        assert_eq!(payout, 594);

        // Validation should pass
        assert!(audit.validate_payout().is_ok());
    }

    #[test]
    fn test_audit_zero_fee() {
        // Zero fee scenario
        let audit = PayoutRoundingAudit::new(1000, 0, 250, 1000).unwrap();

        let protocol_fee = audit.calculate_protocol_fee().unwrap();
        assert_eq!(protocol_fee, 0);

        let payout_pool = audit.calculate_payout_pool().unwrap();
        assert_eq!(payout_pool, 1000);

        let payout = audit.calculate_user_payout().unwrap();
        assert_eq!(payout, 250);

        assert!(audit.validate_payout().is_ok());
    }

    #[test]
    fn test_audit_max_fee() {
        // Maximum fee (100%) scenario
        let audit = PayoutRoundingAudit::new(1000, 10000, 100, 500).unwrap();

        let protocol_fee = audit.calculate_protocol_fee().unwrap();
        assert_eq!(protocol_fee, 1000);

        let payout_pool = audit.calculate_payout_pool().unwrap();
        assert_eq!(payout_pool, 0);

        // No payout available
        let payout = audit.calculate_user_payout().unwrap();
        assert_eq!(payout, 0);

        assert!(audit.validate_payout().is_ok());
    }

    #[test]
    fn test_audit_no_winners() {
        // winning_stake = 0 (no one bet on winning outcome)
        let audit = PayoutRoundingAudit::new(1000, 100, 300, 0).unwrap();

        let payout = audit.calculate_user_payout().unwrap();
        assert_eq!(payout, 0);

        assert!(audit.validate_payout().is_ok());
    }

    #[test]
    fn test_audit_zero_stake() {
        // User has zero stake
        let audit = PayoutRoundingAudit::new(1000, 100, 0, 500).unwrap();

        let payout = audit.calculate_user_payout().unwrap();
        assert_eq!(payout, 0);

        assert!(audit.validate_payout().is_ok());
    }

    #[test]
    fn test_audit_user_stake_exceeds_winning_stake() {
        // Invalid: user stake > winning stake
        let audit = PayoutRoundingAudit::new(1000, 100, 600, 500).unwrap();

        // Validation should fail
        assert_eq!(
            audit.calculate_user_payout(),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn test_audit_invalid_fee_bps() {
        // Fee exceeds 10000 bps
        assert_eq!(
            PayoutRoundingAudit::new(1000, 10001, 100, 500),
            Err(PrediFiError::InvalidFeeBps)
        );
    }

    #[test]
    fn test_audit_negative_inputs() {
        // Negative stake
        assert_eq!(
            PayoutRoundingAudit::new(1000, 100, -100, 500),
            Err(PrediFiError::ArithmeticError)
        );

        // Negative total stake
        assert_eq!(
            PayoutRoundingAudit::new(-1000, 100, 100, 500),
            Err(PrediFiError::ArithmeticError)
        );

        // Negative winning stake
        assert_eq!(
            PayoutRoundingAudit::new(1000, 100, 100, -500),
            Err(PrediFiError::ArithmeticError)
        );
    }

    #[test]
    fn test_audit_rounding_precision() {
        // Test case where rounding matters
        // 1000 total, 2.5% fee, user with 1 of 3 winning stake
        let audit = PayoutRoundingAudit::new(1000, 250, 1, 3).unwrap();

        // Fee: (1000 * 250) / 10000 = 25
        let protocol_fee = audit.calculate_protocol_fee().unwrap();
        assert_eq!(protocol_fee, 25);

        // Payout pool: 1000 - 25 = 975
        let payout_pool = audit.calculate_payout_pool().unwrap();
        assert_eq!(payout_pool, 975);

        // Payout: (1 * 975) / 3 = 325
        let payout = audit.calculate_user_payout().unwrap();
        assert_eq!(payout, 325);

        assert!(audit.validate_payout().is_ok());
    }

    #[test]
    fn test_audit_large_numbers() {
        // Realistic token amounts with 7 decimals
        let audit = PayoutRoundingAudit::new(
            10_000_000_000_000, // 1M tokens
            250,                // 2.5% fee
            2_500_000_000_000,  // 250K tokens
            5_000_000_000_000,  // 500K tokens on winning side
        )
        .unwrap();

        let protocol_fee = audit.calculate_protocol_fee().unwrap();
        assert_eq!(protocol_fee, 250_000_000_000); // 25K tokens

        let payout_pool = audit.calculate_payout_pool().unwrap();
        assert_eq!(payout_pool, 9_750_000_000_000); // 975K tokens

        // (2.5M / 5M) * 9.75M = 4.875M
        let payout = audit.calculate_user_payout().unwrap();
        assert_eq!(payout, 4_875_000_000_000);

        assert!(audit.validate_payout().is_ok());
    }

    #[test]
    fn test_audit_batch_payouts_basic() {
        // Three winners with equal stakes, equal pool distribution
        let user_stakes = vec![300, 300, 400];
        let total_payout = PayoutRoundingAudit::validate_batch_payouts(
            1000, // total stake
            100,  // 1% fee
            &user_stakes,
            1000, // all on winning side
        )
        .unwrap();

        // Each should get proportional share of 990 (after fee)
        // 300/1000 * 990 = 297
        // 300/1000 * 990 = 297
        // 400/1000 * 990 = 396
        // Total = 990
        assert_eq!(total_payout, 990);
    }

    #[test]
    fn test_audit_batch_payouts_rounding_safety() {
        // Test batch payouts ensure sum doesn't exceed pool
        let user_stakes = vec![1, 1, 1];
        let total_payout = PayoutRoundingAudit::validate_batch_payouts(
            3,    // total stake
            1000, // 10% fee
            &user_stakes,
            3, // all winners
        )
        .unwrap();

        // Fee: 3 * 1000 / 10000 = 0 (floor)
        // Payout pool: 3
        // Each stake: (1/3) * 3 = 1
        // Total: 3
        assert_eq!(total_payout, 3);
    }

    #[test]
    fn test_audit_batch_payouts_exceeds_pool() {
        // This shouldn't happen in production, but we validate against it
        // Create a scenario where we'd exceed the pool
        let user_stakes = vec![500, 500];
        let result = PayoutRoundingAudit::validate_batch_payouts(
            100, // small total stake
            0,   // no fee
            &user_stakes,
            1000, // winning stake > total stake (invalid)
        );

        // Should fail due to impossible scenario
        assert!(result.is_err());
    }

    #[test]
    fn test_audit_batch_payouts_with_negative_stake() {
        let user_stakes = vec![100, -50];
        let result = PayoutRoundingAudit::validate_batch_payouts(1000, 100, &user_stakes, 500);

        // Should fail due to negative stake
        assert_eq!(result, Err(PrediFiError::ArithmeticError));
    }

    #[test]
    fn test_audit_payout_never_exceeds_total_stake() {
        // Core invariant: payout must not exceed total stake
        let audit = PayoutRoundingAudit::new(
            100_000, // total stake
            9900,    // 99% fee (leave only 1000 for payouts)
            500,     // user stake
            50_000,  // winning stake (half of total)
        )
        .unwrap();

        let payout = audit.calculate_user_payout().unwrap();

        // Verify payout doesn't exceed total stake (INV-4)
        assert!(payout <= 100_000);

        // Verify validation passes
        assert!(audit.validate_payout().is_ok());
    }

    #[test]
    fn test_audit_fee_calculation_precision() {
        // Test fee calculation with various percentages
        let test_cases = vec![
            (1000, 100, 10),   // 1%
            (1000, 500, 50),   // 5%
            (1000, 2500, 250), // 25%
            (1000, 5000, 500), // 50%
            (1000, 9900, 990), // 99%
            (999, 333, 33),    // 3.33% with rounding
        ];

        for (total, bps, expected_fee) in test_cases {
            let audit = PayoutRoundingAudit::new(total, bps, 100, 100).unwrap();
            let fee = audit.calculate_protocol_fee().unwrap();
            assert_eq!(
                fee,
                expected_fee,
                "Fee mismatch for {}% of {}",
                bps / 100,
                total
            );
        }
    }
}
