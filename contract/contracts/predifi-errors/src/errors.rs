use soroban_sdk::contracterror;

/// Global error enum for PrediFi smart contracts.
/// The error type covers all cases across Predifi contracts.
/// Gap-based numbering allows future error codes to be added without
/// renumbering existing ones or breaking client-side mappings.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PrediFiError {
    // ── Initialization & Configuration (1–5) ─────────────────────────────────
    /// Contract has not been initialized yet.
    NotInitialized = 1,
    /// Contract has already been initialized.
    AlreadyInitialized = 2,
    /// Protocol treasury address is not set.
    TreasuryNotSet = 3,
    /// Access control contract is not set.
    AccessControlNotSet = 4,

    // ── Authorization & Access Control (10–15) ────────────────────────────────
    /// The caller is not authorized to perform this action.
    Unauthorized = 10,
    /// The specified role was not found.
    RoleNotFound = 11,
    /// The caller does not have the required permissions.
    InsufficientPermissions = 12,

    // ── Pool State (20–30) ────────────────────────────────────────────────────
    /// The specified pool was not found.
    PoolNotFound = 20,
    /// The pool has already been resolved.
    PoolAlreadyResolved = 21,
    /// The pool has not been resolved yet.
    PoolNotResolved = 22,
    /// The pool has already expired.
    PoolExpired = 23,
    /// The pool has not expired yet.
    PoolNotExpired = 24,
    /// The pool is not in a valid state for this operation.
    InvalidPoolState = 25,
    /// The outcome value is invalid.
    InvalidOutcome = 26,
    /// The resolution window has expired (too late to resolve).
    ResolutionWindowExpired = 27,
    /// The number of options provided is invalid.
    InvalidOptionsCount = 28,
    /// State inconsistency detected.
    InconsistentState = 29,

    // ── Prediction & Betting (40–50) ──────────────────────────────────────────
    /// The user has no prediction for this pool.
    PredictionNotFound = 40,
    /// The user has already placed a prediction on this pool.
    PredictionAlreadyExists = 41,
    /// The prediction amount is invalid (e.g., zero or negative).
    InvalidPredictionAmount = 42,
    /// Cannot place prediction after pool end time.
    PredictionTooLate = 43,
    /// The user has insufficient balance for this prediction.
    InsufficientBalance = 44,
    /// The prediction amount is below the minimum required stake.
    MinStakeNotMet = 45,

    // ── Claiming & Reward (60–70) ─────────────────────────────────────────────
    /// The user has already claimed winnings for this pool.
    AlreadyClaimed = 60,
    /// The user did not win this pool.
    NotAWinner = 61,
    /// Critical error: winning stake is zero but should not be.
    WinningStakeZero = 62,

    // ── Timestamp & Time Validation (80–85) ───────────────────────────────────
    /// The provided timestamp is invalid.
    InvalidTimestamp = 80,
    /// The end time must be in the future.
    EndTimeMustBeFuture = 81,
    /// The end time is too far in the future.
    EndTimeTooFar = 82,

    // ── Data & Validation (90–100) ────────────────────────────────────────────
    /// The provided data is invalid.
    InvalidData = 90,
    /// The provided address is invalid.
    InvalidAddress = 91,
    /// The provided token address is invalid.
    InvalidToken = 92,
    /// The pagination offset is out of bounds.
    InvalidOffset = 93,
    /// The pagination limit is invalid (e.g., zero or too large).
    InvalidLimit = 94,
    /// The fee basis points exceed the maximum allowed value.
    MaxFeeExceeded = 95,

    // ── Arithmetic & Calculation (110–115) ────────────────────────────────────
    /// An arithmetic overflow occurred.
    ArithmeticOverflow = 110,
    /// An arithmetic underflow occurred.
    ArithmeticUnderflow = 111,
    /// Division by zero attempted.
    DivisionByZero = 112,
    /// An overflow occurred during addition.
    AdditionOverflow = 113,
    /// An overflow occurred during multiplication.
    MultiplicationOverflow = 114,

    // ── Storage & State (120–125) ─────────────────────────────────────────────
    /// The storage key was not found.
    StorageKeyNotFound = 120,
    /// Storage is corrupted or in an invalid state.
    StorageCorrupted = 121,
}

impl PrediFiError {
    /// Returns a human-readable description of the error.
    pub fn as_str(&self) -> &'static str {
        match self {
            // Initialization & Configuration
            PrediFiError::NotInitialized => "Contract not initialized",
            PrediFiError::AlreadyInitialized => "Contract already initialized",
            PrediFiError::TreasuryNotSet => "Treasury address not set",
            PrediFiError::AccessControlNotSet => "Access control address not set",

            // Authorization & Access Control
            PrediFiError::Unauthorized => "Unauthorized access",
            PrediFiError::RoleNotFound => "Role not found",
            PrediFiError::InsufficientPermissions => "Insufficient permissions",

            // Pool State
            PrediFiError::PoolNotFound => "Pool not found",
            PrediFiError::PoolAlreadyResolved => "Pool already resolved",
            PrediFiError::PoolNotResolved => "Pool not resolved",
            PrediFiError::PoolExpired => "Pool has expired",
            PrediFiError::PoolNotExpired => "Pool has not expired",
            PrediFiError::InvalidPoolState => "Invalid pool state",
            PrediFiError::InvalidOutcome => "Invalid outcome",
            PrediFiError::ResolutionWindowExpired => "Resolution window has expired",
            PrediFiError::InvalidOptionsCount => "Invalid options count",
            PrediFiError::InconsistentState => "Inconsistent state detected",

            // Prediction & Betting
            PrediFiError::PredictionNotFound => "Prediction not found",
            PrediFiError::PredictionAlreadyExists => "Prediction already exists",
            PrediFiError::InvalidPredictionAmount => "Invalid prediction amount",
            PrediFiError::PredictionTooLate => "Cannot predict after pool end time",
            PrediFiError::InsufficientBalance => "Insufficient balance",
            PrediFiError::MinStakeNotMet => "Minimum stake not met",

            // Claiming & Rewards
            PrediFiError::AlreadyClaimed => "Already claimed",
            PrediFiError::NotAWinner => "User did not win",
            PrediFiError::WinningStakeZero => "Critical: winning stake is zero",

            // Timestamp & Time Validation
            PrediFiError::InvalidTimestamp => "Invalid timestamp",
            PrediFiError::EndTimeMustBeFuture => "End time must be in the future",
            PrediFiError::EndTimeTooFar => "End time too far in the future",

            // Data & Validation
            PrediFiError::InvalidData => "Invalid data",
            PrediFiError::InvalidAddress => "Invalid address",
            PrediFiError::InvalidToken => "Invalid token",
            PrediFiError::InvalidOffset => "Invalid offset",
            PrediFiError::InvalidLimit => "Invalid limit",
            PrediFiError::MaxFeeExceeded => "Maximum fee exceeded",

            // Arithmetic & Calculation
            PrediFiError::ArithmeticOverflow => "Arithmetic overflow",
            PrediFiError::ArithmeticUnderflow => "Arithmetic underflow",
            PrediFiError::DivisionByZero => "Division by zero",
            PrediFiError::AdditionOverflow => "Addition overflow",
            PrediFiError::MultiplicationOverflow => "Multiplication overflow",

            // Storage & State
            PrediFiError::StorageKeyNotFound => "Storage key not found",
            PrediFiError::StorageCorrupted => "Storage corrupted",
        }
    }
}

impl core::fmt::Display for PrediFiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
