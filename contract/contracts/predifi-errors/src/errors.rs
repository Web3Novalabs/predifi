//! # PrediFi Error Code Reference
//!
//! This module defines all error codes emitted by PrediFi smart contracts.
//! Error codes are designed to be machine-readable so that off-chain monitoring
//! tools (Grafana, SIEM, PagerDuty) can automatically route alerts.
//!
//! ## Alert Severity Tiers
//!
//! ### 🔴 HIGH — Page immediately; potential attack or critical bug
//! | Code | Variant | Meaning |
//! |------|---------|---------|
//! | 10 | `Unauthorized` | Caller lacks required role; pair with `unauthorized_resolution` / `unauthorized_admin_op` on-chain events |
//! | 11 | `InsufficientPermissions` | Role not found in access-control contract |
//! | 120 | `StorageError` | Storage key missing or corrupted |
//! | 121 | `ConsistencyError` | Stake or index inconsistency — state may be corrupt |
//! | 122 | `BalanceMismatch` | Contract holds unexpected token balance |
//! | 160 | `OracleError` | Oracle not set, invalid, or stale |
//! | 161 | `ResolutionError` | Unauthorized or duplicate resolution attempt |
//! | 180 | `AdminError` | Pause / upgrade / version error |
//! | 190 | `RateLimitOrSuspiciousActivity` | Possible spam or abuse detected |
//!
//! ### 🟡 MEDIUM — Alert within 15 minutes; user-impacting but not critical
//! | Code | Variant | Meaning |
//! |------|---------|---------|
//! | 60 | `AlreadyClaimed` | Double-claim attempt; pair with `double_claim_attempt` on-chain event |
//! | 62 | `RewardError` | Reward calc failed or winning stake is zero |
//! | 110 | `ArithmeticError` | Overflow / underflow / division by zero |
//! | 111 | `FeeExceedsAmount` | Fee configuration issue |
//! | 150 | `TokenError` | Token transfer or contract call failed |
//! | 151 | `WithdrawalOrTreasuryError` | Treasury transfer failed |
//!
//! ### 🟢 LOW — Log and review during business hours
//! All remaining codes (1, 2, 20–26, 40–44, 61, 80–81, 90–94) represent
//! expected user-facing validation errors (pool not found, prediction too late,
//! not a winner, etc.) and require no immediate action.
//!
//! ## Log Pattern for External Scrapers
//!
//! Horizon returns contract errors as `Error(Contract, #<code>)` in the
//! transaction result XDR.  Match with:
//! ```text
//! Error\(Contract, #(10|11|120|121|122|160|161|180|190)\)
//! ```
//! to catch all HIGH-severity errors in a single regex rule.

use soroban_sdk::contracterror;

/// Global error enum for PrediFi smart contracts.
/// The error type covers all cases across Predifi contracts.
/// Gap-based numbering allows future error codes to be added without
/// renumbering existing ones or breaking client-side mappings.
///
/// Note: Soroban limits the number of error variants to 32.
/// This enum is optimized to stay within that limit while providing
/// comprehensive error coverage through consolidated error variants.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PrediFiError {
    // -- Initialization & Configuration (1-2) ----------------------------------
    /// Contract has not been initialized yet.
    NotInitialized = 1,
    /// Contract has already been initialized or config not set.
    AlreadyInitializedOrConfigNotSet = 2,

    // -- Authorization & Access Control (10-11) -------------------------------
    /// The caller is not authorized to perform this action.
    Unauthorized = 10,
    /// The specified role was not found or insufficient permissions.
    InsufficientPermissions = 11,

    // -- Pool State (20-26) ---------------------------------------------------
    /// The specified pool was not found.
    PoolNotFound = 20,
    /// The pool has already been resolved.
    PoolAlreadyResolved = 21,
    /// The pool has not been resolved yet.
    PoolNotResolved = 22,
    /// The pool expiry state is invalid for this operation.
    PoolExpiryError = 23,
    /// The pool is not in a valid state for this operation.
    InvalidPoolState = 24,
    /// The outcome value is invalid or out of bounds.
    InvalidOutcome = 25,
    /// State inconsistency or invalid options count detected.
    StateError = 26,

    // -- Prediction & Betting (40-44) -----------------------------------------
    /// The user has no prediction for this pool.
    PredictionNotFound = 40,
    /// The user has already placed a prediction on this pool.
    PredictionAlreadyExists = 41,
    /// The prediction amount is invalid (e.g., zero or negative).
    InvalidPredictionAmount = 42,
    /// Cannot place prediction after pool end time.
    PredictionTooLate = 43,
    /// The user has insufficient balance or stake limit violation.
    InsufficientBalanceOrStakeLimit = 44,

    // -- Claiming & Reward (60-62) --------------------------------------------
    /// The user has already claimed winnings for this pool.
    AlreadyClaimed = 60,
    /// The user did not win this pool.
    NotAWinner = 61,
    /// Reward calculation failed or payout exceeds pool balance.
    RewardError = 62,

    // -- Timestamp & Time Validation (80-81) ----------------------------------
    /// The provided timestamp is invalid or time constraints not met.
    InvalidTimestamp = 80,
    /// The end time or resolution time constraints are not met.
    TimeConstraintError = 81,

    // -- Data & Validation (90-94) -------------------------------------------
    /// The provided data is invalid.
    InvalidData = 90,
    /// The provided address or token is invalid.
    InvalidAddressOrToken = 91,
    /// The pagination offset or limit is invalid.
    InvalidPagination = 92,
    /// The fee basis points exceed the maximum allowed value (10000).
    InvalidFeeBps = 93,
    /// Metadata, label, or duplicate labels error.
    MetadataError = 94,

    // -- Arithmetic & Calculation (110-112) ------------------------------------
    /// An arithmetic overflow, underflow, or division by zero occurred.
    ArithmeticError = 110,
    /// The calculated fee exceeds the total amount.
    FeeExceedsAmount = 111,
    /// An input amount is invalid (e.g., would cause overflow in arithmetic).
    InvalidAmount = 112,

    // -- Storage & State (120-122) ---------------------------------------------
    /// The storage key was not found or storage is corrupted.
    StorageError = 120,
    /// The pool's total stake or index is inconsistent.
    ConsistencyError = 121,
    /// A balance mismatch was detected in the contract account.
    BalanceMismatch = 122,

    // -- Token & Transfer (150-151) --------------------------------------------
    /// Token transfer, approval, or contract call failed.
    TokenError = 150,
    /// Withdrawal or treasury transfer failed.
    WithdrawalOrTreasuryError = 151,

    // -- Oracle & Resolution (160-161) -----------------------------------------
    /// Oracle error or stale data detected.
    OracleError = 160,
    /// Resolution error or unauthorized resolver.
    ResolutionError = 161,

    // -- Emergency & Admin (180) -----------------------------------------------
    /// Contract pause, emergency, version, or upgrade error.
    AdminError = 180,

    // -- Rate Limiting & Spam Prevention (190) ---------------------------------
    /// Rate limit exceeded, cooldown not elapsed, or suspicious activity.
    RateLimitOrSuspiciousActivity = 190,

    // -- Pool Configuration (200) ----------------------------------------------
    /// required_resolutions exceeds the number of active operators; pool can never be resolved.
    RequiredResolutionsExceedOperators = 200,
}

impl PrediFiError {
    /// Returns the numeric error code for this error.
    /// Useful for frontend error handling and logging.
    pub const fn code(&self) -> u32 {
        *self as u32
    }

    /// Returns the error category as a string.
    /// Useful for grouping errors in logs and analytics.
    pub const fn category(&self) -> &'static str {
        match self {
            Self::NotInitialized | Self::AlreadyInitializedOrConfigNotSet => "initialization",
            Self::Unauthorized | Self::InsufficientPermissions => "authorization",
            Self::PoolNotFound
            | Self::PoolAlreadyResolved
            | Self::PoolNotResolved
            | Self::PoolExpiryError
            | Self::InvalidPoolState
            | Self::InvalidOutcome
            | Self::StateError => "pool_state",
            Self::PredictionNotFound
            | Self::PredictionAlreadyExists
            | Self::InvalidPredictionAmount
            | Self::PredictionTooLate
            | Self::InsufficientBalanceOrStakeLimit => "prediction",
            Self::AlreadyClaimed | Self::NotAWinner | Self::RewardError => "claiming",
            Self::InvalidTimestamp | Self::TimeConstraintError => "timestamp",
            Self::InvalidData
            | Self::InvalidAddressOrToken
            | Self::InvalidPagination
            | Self::InvalidFeeBps
            | Self::MetadataError => "validation",
            Self::ArithmeticError | Self::FeeExceedsAmount | Self::InvalidAmount => "arithmetic",
            Self::StorageError | Self::ConsistencyError | Self::BalanceMismatch => "storage",
            Self::TokenError | Self::WithdrawalOrTreasuryError => "token",
            Self::OracleError | Self::ResolutionError => "oracle",
            Self::AdminError => "admin",
            Self::RateLimitOrSuspiciousActivity => "rate_limiting",
            Self::RequiredResolutionsExceedOperators => "pool_configuration",
        }
    }

    /// Returns a stable, machine-friendly label for this error variant.
    ///
    /// These labels are useful when explorer output only shows numeric
    /// contract error codes and off-chain tools need a deterministic mapping.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::NotInitialized => "INIT_NOT_INITIALIZED",
            Self::AlreadyInitializedOrConfigNotSet => "INIT_ALREADY_INITIALIZED_OR_CONFIG_NOT_SET",
            Self::Unauthorized => "AUTH_UNAUTHORIZED",
            Self::InsufficientPermissions => "AUTH_INSUFFICIENT_PERMISSIONS",
            Self::PoolNotFound => "POOL_NOT_FOUND",
            Self::PoolAlreadyResolved => "POOL_ALREADY_RESOLVED",
            Self::PoolNotResolved => "POOL_NOT_RESOLVED",
            Self::PoolExpiryError => "POOL_EXPIRY_ERROR",
            Self::InvalidPoolState => "POOL_INVALID_STATE",
            Self::InvalidOutcome => "POOL_INVALID_OUTCOME",
            Self::StateError => "POOL_STATE_ERROR",
            Self::PredictionNotFound => "PREDICTION_NOT_FOUND",
            Self::PredictionAlreadyExists => "PREDICTION_ALREADY_EXISTS",
            Self::InvalidPredictionAmount => "PREDICTION_INVALID_AMOUNT",
            Self::PredictionTooLate => "PREDICTION_TOO_LATE",
            Self::InsufficientBalanceOrStakeLimit => {
                "PREDICTION_INSUFFICIENT_BALANCE_OR_STAKE_LIMIT"
            }
            Self::AlreadyClaimed => "CLAIM_ALREADY_CLAIMED",
            Self::NotAWinner => "CLAIM_NOT_A_WINNER",
            Self::RewardError => "CLAIM_REWARD_ERROR",
            Self::InvalidTimestamp => "TIME_INVALID_TIMESTAMP",
            Self::TimeConstraintError => "TIME_CONSTRAINT_ERROR",
            Self::InvalidData => "VALIDATION_INVALID_DATA",
            Self::InvalidAddressOrToken => "VALIDATION_INVALID_ADDRESS_OR_TOKEN",
            Self::InvalidPagination => "VALIDATION_INVALID_PAGINATION",
            Self::InvalidFeeBps => "VALIDATION_INVALID_FEE_BPS",
            Self::MetadataError => "VALIDATION_METADATA_ERROR",
            Self::ArithmeticError => "MATH_ARITHMETIC_ERROR",
            Self::FeeExceedsAmount => "MATH_FEE_EXCEEDS_AMOUNT",
            Self::InvalidAmount => "MATH_INVALID_AMOUNT",
            Self::StorageError => "STORAGE_ERROR",
            Self::ConsistencyError => "STORAGE_CONSISTENCY_ERROR",
            Self::BalanceMismatch => "STORAGE_BALANCE_MISMATCH",
            Self::TokenError => "TOKEN_ERROR",
            Self::WithdrawalOrTreasuryError => "TOKEN_WITHDRAWAL_OR_TREASURY_ERROR",
            Self::OracleError => "ORACLE_ERROR",
            Self::ResolutionError => "ORACLE_RESOLUTION_ERROR",
            Self::AdminError => "ADMIN_ERROR",
            Self::RateLimitOrSuspiciousActivity => "RATE_LIMIT_OR_SUSPICIOUS_ACTIVITY",
            Self::RequiredResolutionsExceedOperators => {
                "POOL_REQUIRED_RESOLUTIONS_EXCEED_OPERATORS"
            }
        }
    }

    /// Returns whether this error is recoverable by the user.
    /// Non-recoverable errors typically indicate system issues or bugs.
    pub const fn is_recoverable(&self) -> bool {
        match self {
            // Non-recoverable: system/contract issues
            Self::NotInitialized
            | Self::AlreadyInitializedOrConfigNotSet
            | Self::StorageError
            | Self::ConsistencyError
            | Self::BalanceMismatch
            | Self::RewardError
            | Self::StateError
            | Self::AdminError => false,
            // Recoverable: user can fix by changing input or waiting
            _ => true,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            // Initialization & Configuration
            Self::NotInitialized => "Contract is not initialized. Call init before this operation.",
            Self::AlreadyInitializedOrConfigNotSet => {
                "Contract already initialized or required config (treasury/access control) is missing"
            }

            // Authorization & Access Control
            Self::Unauthorized => "Caller is not authorized to perform this action",
            Self::InsufficientPermissions => "Caller role is missing or does not grant required permission",

            // Pool State
            Self::PoolNotFound => "Pool ID does not exist",
            Self::PoolAlreadyResolved => "Pool is already resolved",
            Self::PoolNotResolved => "Pool is not resolved yet",
            Self::PoolExpiryError => "Pool expiry state is invalid for this operation",
            Self::InvalidPoolState => "Invalid pool state",
            Self::InvalidOutcome => "Invalid outcome or outcome index out of bounds",
            Self::StateError => "State inconsistency or invalid options count detected",

            // Prediction & Betting
            Self::PredictionNotFound => "No prediction found for this user and pool",
            Self::PredictionAlreadyExists => "User already placed a prediction in this pool",
            Self::InvalidPredictionAmount => {
                "Invalid prediction amount (zero, negative, or invalid)"
            }
            Self::PredictionTooLate => "Prediction window has closed for this pool",
            Self::InsufficientBalanceOrStakeLimit => {
                "Insufficient balance, below min stake, or above max stake limit"
            }

            // Claiming & Rewards
            Self::AlreadyClaimed => "Winnings already claimed for this pool",
            Self::NotAWinner => "User is not in a winning outcome for this pool",
            Self::RewardError => {
                "Reward calculation failed, winning stake is zero, or payout exceeds pool"
            }

            // Timestamp & Time Validation
            Self::InvalidTimestamp => "Invalid timestamp or time constraints not met",
            Self::TimeConstraintError => "End time or resolution time constraints are not met",

            // Data & Validation
            Self::InvalidData => "Input data failed validation",
            Self::InvalidAddressOrToken => "Provided address or token contract is invalid",
            Self::InvalidPagination => "Invalid pagination offset or limit",
            Self::InvalidFeeBps => "Invalid fee basis points (max 10000)",
            Self::MetadataError => "Metadata, label invalid/too long, or duplicate labels detected",

            // Arithmetic & Calculation
            Self::ArithmeticError => "Arithmetic overflow, underflow, or division-by-zero occurred",
            Self::FeeExceedsAmount => "Calculated fee exceeds total amount",
            Self::InvalidAmount => "Input amount is invalid or would cause arithmetic overflow",

            // Storage & State
            Self::StorageError => "Required storage key missing or storage is corrupted",
            Self::ConsistencyError => "Pool stake or index inconsistency detected",
            Self::BalanceMismatch => "Contract token balance does not match internal accounting",

            // Token & Transfer
            Self::TokenError => "Token transfer/approval or token contract call failed",
            Self::WithdrawalOrTreasuryError => "Withdrawal or treasury transfer failed",

            // Oracle & Resolution
            Self::OracleError => "Oracle is not configured, returned invalid data, or data is stale",
            Self::ResolutionError => {
                "Pool resolution failed due to duplicate attempt, mismatch, or unauthorized resolver"
            }

            // Emergency & Admin
            Self::AdminError => "Administrative operation failed (pause/emergency/version/upgrade)",

            // Rate Limiting & Spam Prevention
            Self::RateLimitOrSuspiciousActivity => "Rate limit exceeded, cooldown active, or suspicious activity detected",

            // Pool Configuration
            Self::RequiredResolutionsExceedOperators => "Required resolutions exceeds the number of active operators; pool can never be resolved",
        }
    }
}

impl core::fmt::Display for PrediFiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::PrediFiError;

    #[test]
    fn error_helpers_return_expected_metadata() {
        let error = PrediFiError::Unauthorized;

        assert_eq!(error.code(), 10);
        assert_eq!(error.category(), "authorization");
        assert!(error.is_recoverable());
        assert_eq!(
            error.as_str(),
            "Caller is not authorized to perform this action"
        );
    }

    #[test]
    fn system_errors_are_marked_non_recoverable() {
        let error = PrediFiError::StorageError;

        assert_eq!(error.category(), "storage");
        assert!(!error.is_recoverable());
    }

    #[test]
    fn labels_and_messages_are_descriptive() {
        let error = PrediFiError::Unauthorized;

        assert_eq!(error.label(), "AUTH_UNAUTHORIZED");
        assert!(
            error.as_str().contains("authorized"),
            "message should help explain access failure"
        );
    }

    // --- Exhaustive per-variant tests ---

    #[test]
    fn not_initialized() {
        let e = PrediFiError::NotInitialized;
        assert_eq!(e.code(), 1);
        assert_eq!(e.category(), "initialization");
        assert_eq!(e.label(), "INIT_NOT_INITIALIZED");
        assert!(!e.is_recoverable());
        assert!(!e.as_str().is_empty());
    }

    #[test]
    fn already_initialized_or_config_not_set() {
        let e = PrediFiError::AlreadyInitializedOrConfigNotSet;
        assert_eq!(e.code(), 2);
        assert_eq!(e.category(), "initialization");
        assert_eq!(e.label(), "INIT_ALREADY_INITIALIZED_OR_CONFIG_NOT_SET");
        assert!(!e.is_recoverable());
    }

    #[test]
    fn unauthorized() {
        let e = PrediFiError::Unauthorized;
        assert_eq!(e.code(), 10);
        assert_eq!(e.category(), "authorization");
        assert_eq!(e.label(), "AUTH_UNAUTHORIZED");
        assert!(e.is_recoverable());
    }

    #[test]
    fn insufficient_permissions() {
        let e = PrediFiError::InsufficientPermissions;
        assert_eq!(e.code(), 11);
        assert_eq!(e.category(), "authorization");
        assert_eq!(e.label(), "AUTH_INSUFFICIENT_PERMISSIONS");
        assert!(e.is_recoverable());
    }

    #[test]
    fn pool_not_found() {
        let e = PrediFiError::PoolNotFound;
        assert_eq!(e.code(), 20);
        assert_eq!(e.category(), "pool_state");
        assert_eq!(e.label(), "POOL_NOT_FOUND");
        assert!(e.is_recoverable());
    }

    #[test]
    fn pool_already_resolved() {
        let e = PrediFiError::PoolAlreadyResolved;
        assert_eq!(e.code(), 21);
        assert_eq!(e.category(), "pool_state");
        assert_eq!(e.label(), "POOL_ALREADY_RESOLVED");
        assert!(e.is_recoverable());
    }

    #[test]
    fn pool_not_resolved() {
        let e = PrediFiError::PoolNotResolved;
        assert_eq!(e.code(), 22);
        assert_eq!(e.category(), "pool_state");
        assert_eq!(e.label(), "POOL_NOT_RESOLVED");
        assert!(e.is_recoverable());
    }

    #[test]
    fn pool_expiry_error() {
        let e = PrediFiError::PoolExpiryError;
        assert_eq!(e.code(), 23);
        assert_eq!(e.category(), "pool_state");
        assert_eq!(e.label(), "POOL_EXPIRY_ERROR");
        assert!(e.is_recoverable());
    }

    #[test]
    fn invalid_pool_state() {
        let e = PrediFiError::InvalidPoolState;
        assert_eq!(e.code(), 24);
        assert_eq!(e.category(), "pool_state");
        assert_eq!(e.label(), "POOL_INVALID_STATE");
        assert!(e.is_recoverable());
    }

    #[test]
    fn invalid_outcome() {
        let e = PrediFiError::InvalidOutcome;
        assert_eq!(e.code(), 25);
        assert_eq!(e.category(), "pool_state");
        assert_eq!(e.label(), "POOL_INVALID_OUTCOME");
        assert!(e.is_recoverable());
    }

    #[test]
    fn state_error() {
        let e = PrediFiError::StateError;
        assert_eq!(e.code(), 26);
        assert_eq!(e.category(), "pool_state");
        assert_eq!(e.label(), "POOL_STATE_ERROR");
        assert!(!e.is_recoverable());
    }

    #[test]
    fn prediction_not_found() {
        let e = PrediFiError::PredictionNotFound;
        assert_eq!(e.code(), 40);
        assert_eq!(e.category(), "prediction");
        assert_eq!(e.label(), "PREDICTION_NOT_FOUND");
        assert!(e.is_recoverable());
    }

    #[test]
    fn prediction_already_exists() {
        let e = PrediFiError::PredictionAlreadyExists;
        assert_eq!(e.code(), 41);
        assert_eq!(e.category(), "prediction");
        assert_eq!(e.label(), "PREDICTION_ALREADY_EXISTS");
        assert!(e.is_recoverable());
    }

    #[test]
    fn invalid_prediction_amount() {
        let e = PrediFiError::InvalidPredictionAmount;
        assert_eq!(e.code(), 42);
        assert_eq!(e.category(), "prediction");
        assert_eq!(e.label(), "PREDICTION_INVALID_AMOUNT");
        assert!(e.is_recoverable());
    }

    #[test]
    fn prediction_too_late() {
        let e = PrediFiError::PredictionTooLate;
        assert_eq!(e.code(), 43);
        assert_eq!(e.category(), "prediction");
        assert_eq!(e.label(), "PREDICTION_TOO_LATE");
        assert!(e.is_recoverable());
    }

    #[test]
    fn insufficient_balance_or_stake_limit() {
        let e = PrediFiError::InsufficientBalanceOrStakeLimit;
        assert_eq!(e.code(), 44);
        assert_eq!(e.category(), "prediction");
        assert_eq!(e.label(), "PREDICTION_INSUFFICIENT_BALANCE_OR_STAKE_LIMIT");
        assert!(e.is_recoverable());
    }

    #[test]
    fn already_claimed() {
        let e = PrediFiError::AlreadyClaimed;
        assert_eq!(e.code(), 60);
        assert_eq!(e.category(), "claiming");
        assert_eq!(e.label(), "CLAIM_ALREADY_CLAIMED");
        assert!(e.is_recoverable());
    }

    #[test]
    fn not_a_winner() {
        let e = PrediFiError::NotAWinner;
        assert_eq!(e.code(), 61);
        assert_eq!(e.category(), "claiming");
        assert_eq!(e.label(), "CLAIM_NOT_A_WINNER");
        assert!(e.is_recoverable());
    }

    #[test]
    fn reward_error() {
        let e = PrediFiError::RewardError;
        assert_eq!(e.code(), 62);
        assert_eq!(e.category(), "claiming");
        assert_eq!(e.label(), "CLAIM_REWARD_ERROR");
        assert!(!e.is_recoverable());
    }

    #[test]
    fn invalid_timestamp() {
        let e = PrediFiError::InvalidTimestamp;
        assert_eq!(e.code(), 80);
        assert_eq!(e.category(), "timestamp");
        assert_eq!(e.label(), "TIME_INVALID_TIMESTAMP");
        assert!(e.is_recoverable());
    }

    #[test]
    fn time_constraint_error() {
        let e = PrediFiError::TimeConstraintError;
        assert_eq!(e.code(), 81);
        assert_eq!(e.category(), "timestamp");
        assert_eq!(e.label(), "TIME_CONSTRAINT_ERROR");
        assert!(e.is_recoverable());
    }

    #[test]
    fn invalid_data() {
        let e = PrediFiError::InvalidData;
        assert_eq!(e.code(), 90);
        assert_eq!(e.category(), "validation");
        assert_eq!(e.label(), "VALIDATION_INVALID_DATA");
        assert!(e.is_recoverable());
    }

    #[test]
    fn invalid_address_or_token() {
        let e = PrediFiError::InvalidAddressOrToken;
        assert_eq!(e.code(), 91);
        assert_eq!(e.category(), "validation");
        assert_eq!(e.label(), "VALIDATION_INVALID_ADDRESS_OR_TOKEN");
        assert!(e.is_recoverable());
    }

    #[test]
    fn invalid_pagination() {
        let e = PrediFiError::InvalidPagination;
        assert_eq!(e.code(), 92);
        assert_eq!(e.category(), "validation");
        assert_eq!(e.label(), "VALIDATION_INVALID_PAGINATION");
        assert!(e.is_recoverable());
    }

    #[test]
    fn invalid_fee_bps() {
        let e = PrediFiError::InvalidFeeBps;
        assert_eq!(e.code(), 93);
        assert_eq!(e.category(), "validation");
        assert_eq!(e.label(), "VALIDATION_INVALID_FEE_BPS");
        assert!(e.is_recoverable());
    }

    #[test]
    fn metadata_error() {
        let e = PrediFiError::MetadataError;
        assert_eq!(e.code(), 94);
        assert_eq!(e.category(), "validation");
        assert_eq!(e.label(), "VALIDATION_METADATA_ERROR");
        assert!(e.is_recoverable());
    }

    #[test]
    fn arithmetic_error() {
        let e = PrediFiError::ArithmeticError;
        assert_eq!(e.code(), 110);
        assert_eq!(e.category(), "arithmetic");
        assert_eq!(e.label(), "MATH_ARITHMETIC_ERROR");
        assert!(e.is_recoverable());
    }

    #[test]
    fn fee_exceeds_amount() {
        let e = PrediFiError::FeeExceedsAmount;
        assert_eq!(e.code(), 111);
        assert_eq!(e.category(), "arithmetic");
        assert_eq!(e.label(), "MATH_FEE_EXCEEDS_AMOUNT");
        assert!(e.is_recoverable());
    }

    #[test]
    fn invalid_amount() {
        let e = PrediFiError::InvalidAmount;
        assert_eq!(e.code(), 112);
        assert_eq!(e.category(), "arithmetic");
        assert_eq!(e.label(), "MATH_INVALID_AMOUNT");
        assert!(e.is_recoverable());
    }

    #[test]
    fn storage_error() {
        let e = PrediFiError::StorageError;
        assert_eq!(e.code(), 120);
        assert_eq!(e.category(), "storage");
        assert_eq!(e.label(), "STORAGE_ERROR");
        assert!(!e.is_recoverable());
    }

    #[test]
    fn consistency_error() {
        let e = PrediFiError::ConsistencyError;
        assert_eq!(e.code(), 121);
        assert_eq!(e.category(), "storage");
        assert_eq!(e.label(), "STORAGE_CONSISTENCY_ERROR");
        assert!(!e.is_recoverable());
    }

    #[test]
    fn balance_mismatch() {
        let e = PrediFiError::BalanceMismatch;
        assert_eq!(e.code(), 122);
        assert_eq!(e.category(), "storage");
        assert_eq!(e.label(), "STORAGE_BALANCE_MISMATCH");
        assert!(!e.is_recoverable());
    }

    #[test]
    fn token_error() {
        let e = PrediFiError::TokenError;
        assert_eq!(e.code(), 150);
        assert_eq!(e.category(), "token");
        assert_eq!(e.label(), "TOKEN_ERROR");
        assert!(e.is_recoverable());
    }

    #[test]
    fn withdrawal_or_treasury_error() {
        let e = PrediFiError::WithdrawalOrTreasuryError;
        assert_eq!(e.code(), 151);
        assert_eq!(e.category(), "token");
        assert_eq!(e.label(), "TOKEN_WITHDRAWAL_OR_TREASURY_ERROR");
        assert!(e.is_recoverable());
    }

    #[test]
    fn oracle_error() {
        let e = PrediFiError::OracleError;
        assert_eq!(e.code(), 160);
        assert_eq!(e.category(), "oracle");
        assert_eq!(e.label(), "ORACLE_ERROR");
        assert!(e.is_recoverable());
    }

    #[test]
    fn resolution_error() {
        let e = PrediFiError::ResolutionError;
        assert_eq!(e.code(), 161);
        assert_eq!(e.category(), "oracle");
        assert_eq!(e.label(), "ORACLE_RESOLUTION_ERROR");
        assert!(e.is_recoverable());
    }

    #[test]
    fn admin_error() {
        let e = PrediFiError::AdminError;
        assert_eq!(e.code(), 180);
        assert_eq!(e.category(), "admin");
        assert_eq!(e.label(), "ADMIN_ERROR");
        assert!(!e.is_recoverable());
    }

    #[test]
    fn rate_limit_or_suspicious_activity() {
        let e = PrediFiError::RateLimitOrSuspiciousActivity;
        assert_eq!(e.code(), 190);
        assert_eq!(e.category(), "rate_limiting");
        assert_eq!(e.label(), "RATE_LIMIT_OR_SUSPICIOUS_ACTIVITY");
        assert!(e.is_recoverable());
    }

    #[test]
    fn required_resolutions_exceed_operators() {
        let e = PrediFiError::RequiredResolutionsExceedOperators;
        assert_eq!(e.code(), 200);
        assert_eq!(e.category(), "pool_configuration");
        assert_eq!(e.label(), "POOL_REQUIRED_RESOLUTIONS_EXCEED_OPERATORS");
        assert!(e.is_recoverable());
    }

    #[test]
    fn display_impl_non_empty() {
        // Verify Display impl delegates to as_str (non-empty output)
        let e = PrediFiError::PoolNotFound;
        assert!(!e.as_str().is_empty());
    }

    #[test]
    fn all_variants_have_non_empty_label_and_message() {
        let variants = [
            PrediFiError::NotInitialized,
            PrediFiError::AlreadyInitializedOrConfigNotSet,
            PrediFiError::Unauthorized,
            PrediFiError::InsufficientPermissions,
            PrediFiError::PoolNotFound,
            PrediFiError::PoolAlreadyResolved,
            PrediFiError::PoolNotResolved,
            PrediFiError::PoolExpiryError,
            PrediFiError::InvalidPoolState,
            PrediFiError::InvalidOutcome,
            PrediFiError::StateError,
            PrediFiError::PredictionNotFound,
            PrediFiError::PredictionAlreadyExists,
            PrediFiError::InvalidPredictionAmount,
            PrediFiError::PredictionTooLate,
            PrediFiError::InsufficientBalanceOrStakeLimit,
            PrediFiError::AlreadyClaimed,
            PrediFiError::NotAWinner,
            PrediFiError::RewardError,
            PrediFiError::InvalidTimestamp,
            PrediFiError::TimeConstraintError,
            PrediFiError::InvalidData,
            PrediFiError::InvalidAddressOrToken,
            PrediFiError::InvalidPagination,
            PrediFiError::InvalidFeeBps,
            PrediFiError::MetadataError,
            PrediFiError::ArithmeticError,
            PrediFiError::FeeExceedsAmount,
            PrediFiError::InvalidAmount,
            PrediFiError::StorageError,
            PrediFiError::ConsistencyError,
            PrediFiError::BalanceMismatch,
            PrediFiError::TokenError,
            PrediFiError::WithdrawalOrTreasuryError,
            PrediFiError::OracleError,
            PrediFiError::ResolutionError,
            PrediFiError::AdminError,
            PrediFiError::RateLimitOrSuspiciousActivity,
            PrediFiError::RequiredResolutionsExceedOperators,
        ];
        for v in variants {
            assert!(!v.label().is_empty(), "label empty for code {}", v.code());
            assert!(
                !v.as_str().is_empty(),
                "message empty for code {}",
                v.code()
            );
            assert!(
                !v.category().is_empty(),
                "category empty for code {}",
                v.code()
            );
        }
    }
}
