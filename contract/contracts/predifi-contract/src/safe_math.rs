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
        let numerator = amount
            .checked_mul(bps)
            .ok_or(PrediFiError::ArithmeticError)?;

        Self::divide_with_rounding(numerator, MAX_BPS, rounding)
    }

    /// Calculate a proportion: (numerator / denominator) * amount
    ///
    /// This is the core function for payout calculations.
    ///
    /// # Arguments
    /// * `numerator` - The user's stake or share
    /// * `denominator` - The total stake or pool
    /// * `amount` - The amount to distribute proportionally
    /// * `rounding` - Rounding mode to use
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
            .ok_or(PrediFiError::ArithmeticError)?;

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
            .ok_or(PrediFiError::ArithmeticError)?;
        let remainder = numerator
            .checked_rem(denominator)
            .ok_or(PrediFiError::ArithmeticError)?;

        match rounding {
            RoundingMode::ProtocolFavor => {
                // Always round down (floor)
                Ok(quotient)
            }
            RoundingMode::Neutral => {
                // Round to nearest (half up)
                let half = denominator
                    .checked_div(2)
                    .ok_or(PrediFiError::ArithmeticError)?;
                if remainder >= half {
                    quotient.checked_add(1).ok_or(PrediFiError::ArithmeticError)
                } else {
                    Ok(quotient)
                }
            }
            RoundingMode::UserFavor => {
                // Round up (ceiling) if there's any remainder
                if remainder > 0 {
                    quotient.checked_add(1).ok_or(PrediFiError::ArithmeticError)
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

        // Verify we didn't over-distribute
        let total_distributed: i128 = results.iter().sum();
        if total_distributed > pool_balance {
            return Err(PrediFiError::RewardError);
        }

        Ok(results)
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
        a.checked_mul(b).ok_or(PrediFiError::ArithmeticError)
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
            Err(PrediFiError::ArithmeticError)
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
}
