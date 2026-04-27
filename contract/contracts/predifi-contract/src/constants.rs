//! Contract constants and configuration values.
//!
//! This module contains all constant values used throughout the PrediFi contract,
//! including storage parameters, pool limits, and default values.

// ═══════════════════════════════════════════════════════════════════════════
// STORAGE & LEDGER CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/// Number of ledgers in a day (assuming ~5 second ledger close time).
/// Used for calculating storage TTL extensions.
pub const DAY_IN_LEDGERS: u32 = 17280;

/// Threshold for extending storage TTL (14 days in ledgers).
/// When storage TTL falls below this, it should be extended.
pub const BUMP_THRESHOLD: u32 = 14 * DAY_IN_LEDGERS;

/// Amount to extend storage TTL by (30 days in ledgers).
/// Storage is extended by this amount when bumped.
pub const BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

// ═══════════════════════════════════════════════════════════════════════════
// POOL CONFIGURATION CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/// Default minimum pool duration in seconds (1 hour).
/// Pools must be active for at least this duration before they can end.
pub const DEFAULT_MIN_POOL_DURATION: u64 = 3600;

/// Cancellation delay in seconds for overdue pools (7 days).
/// After this period past the pool's end_time, any user can cancel the pool.
pub const CANCELATION_DELAY: u64 = 604800;

/// Default global minimum stake amount (1 unit in base token units).
/// Predictions below this threshold are rejected to prevent spam.
pub const DEFAULT_GLOBAL_MIN_STAKE: i128 = 1;

/// Default cooldown in seconds between consecutive place_prediction calls by the same user.
/// Defaults to disabled so existing deployments can opt in explicitly via admin config.
pub const DEFAULT_PREDICTION_COOLDOWN_SECONDS: u64 = 0;

/// Maximum number of options/outcomes allowed in a single pool.
/// This limit prevents excessive gas costs and ensures reasonable pool complexity.
pub const MAX_OPTIONS_COUNT: u32 = 100;

/// Maximum initial liquidity that can be provided (100M tokens at 7 decimals).
/// This is the maximum amount of "house money" a pool creator can provide.
/// At 7 decimal places (e.g., USDC on Stellar), this equals 100,000,000 USDC.
pub const MAX_INITIAL_LIQUIDITY: i128 = 100_000_000_000_000;

// ═══════════════════════════════════════════════════════════════════════════
// MONITORING & ALERT THRESHOLDS
// ═══════════════════════════════════════════════════════════════════════════

/// Stake amount (in base token units) above which a `HighValuePredictionEvent`
/// is emitted so off-chain monitors can apply extra scrutiny.
/// At 7 decimal places (e.g., USDC on Stellar), this equals 0.1 USDC.
pub const HIGH_VALUE_THRESHOLD: i128 = 1_000_000;

// ═══════════════════════════════════════════════════════════════════════════
// VERSION CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/// Current contract version. Bump on each release to support safe migrations.
/// This is stored in contract instance storage during initialization.
pub const CONTRACT_VERSION: u32 = 1;

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_constants_are_positive() {
        assert!(DAY_IN_LEDGERS > 0);
        assert!(BUMP_THRESHOLD > 0);
        assert!(BUMP_AMOUNT > 0);
    }

    #[test]
    fn test_bump_threshold_less_than_bump_amount() {
        // Bump threshold should be less than bump amount to ensure
        // storage is extended before it expires
        assert!(BUMP_THRESHOLD < BUMP_AMOUNT);
    }

    #[test]
    fn test_pool_duration_is_reasonable() {
        // Default minimum pool duration should be at least 1 hour
        assert!(DEFAULT_MIN_POOL_DURATION >= 3600);
    }

    #[test]
    fn test_max_options_is_reasonable() {
        // Max options should be between 2 and 1000
        assert!(MAX_OPTIONS_COUNT >= 2);
        assert!(MAX_OPTIONS_COUNT <= 1000);
    }

    #[test]
    fn test_max_initial_liquidity_is_positive() {
        assert!(MAX_INITIAL_LIQUIDITY > 0);
    }

    #[test]
    fn test_high_value_threshold_is_positive() {
        assert!(HIGH_VALUE_THRESHOLD > 0);
    }

    #[test]
    fn test_prediction_cooldown_is_non_negative() {
        assert_eq!(DEFAULT_PREDICTION_COOLDOWN_SECONDS, 0);
    }

    #[test]
    fn test_contract_version_is_positive() {
        assert!(CONTRACT_VERSION > 0);
    }

    #[test]
    fn test_ledger_calculations() {
        // Verify that BUMP_THRESHOLD and BUMP_AMOUNT are correctly calculated
        assert_eq!(BUMP_THRESHOLD, 14 * DAY_IN_LEDGERS);
        assert_eq!(BUMP_AMOUNT, 30 * DAY_IN_LEDGERS);
    }

    #[test]
    fn test_high_value_threshold_equals_0_1_usdc() {
        // At 7 decimals, 1 USDC = 10_000_000 base units.
        // Therefore 1_000_000 base units equals 0.1 USDC.
        assert_eq!(HIGH_VALUE_THRESHOLD, 1_000_000);
    }
}
