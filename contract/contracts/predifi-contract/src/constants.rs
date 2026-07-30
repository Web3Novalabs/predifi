//! Contract constants and configuration values.
//!
//! This module contains all constant values used throughout the PrediFi contract,
//! including storage parameters, pool limits, and default values.

// ═══════════════════════════════════════════════════════════════════════════
// STORAGE & LEDGER CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/// Number of ledgers in a day (assuming ~5 second ledger close time).
///
/// **Units:** Ledgers (dimensionless)
///
/// **Rationale:** Stellar ledgers close approximately every 5 seconds. This constant
/// represents the number of ledgers that elapse in a 24-hour period (86400 seconds / 5).
///
/// **Impact of changes:**
/// - Increasing this value would cause storage TTL calculations to be too conservative,
///   potentially extending storage more frequently than necessary and increasing costs.
/// - Decreasing this value would cause storage to expire sooner than intended,
///   risking data loss if TTL extensions are not triggered in time.
/// - This value should only be changed if Stellar's ledger close time changes significantly.
///
/// **Used for:** Calculating storage TTL extensions (`BUMP_THRESHOLD`, `BUMP_AMOUNT`).
pub const DAY_IN_LEDGERS: u32 = 17280;

/// Threshold for extending storage TTL (14 days in ledgers).
///
/// **Units:** Ledgers (dimensionless)
/// **Value:** 241,920 ledgers (14 days)
///
/// **Rationale:** When persistent storage entries approach expiration, they must be
/// extended to prevent data loss. This threshold provides a 14-day buffer, ensuring
/// that even if the contract is not called frequently, storage entries remain valid.
/// The 14-day window balances storage costs with safety margins.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 30 days) reduces the frequency of TTL extensions,
///   lowering storage costs but increasing the risk of data loss if the contract is inactive.
/// - Decreasing this value (e.g., to 7 days) increases extension frequency, raising costs
///   but providing a larger safety margin against data loss.
/// - Must be less than `BUMP_AMOUNT` to ensure storage is extended before it expires.
///
/// **Used for:** Determining when to call `extend_persistent` and `bump_ttl`.
pub const BUMP_THRESHOLD: u32 = 14 * DAY_IN_LEDGERS;

/// Amount to extend storage TTL by (30 days in ledgers).
///
/// **Units:** Ledgers (dimensionless)
/// **Value:** 518,400 ledgers (30 days)
///
/// **Rationale:** When storage TTL is extended, it's extended by a significant duration
/// to minimize the frequency of future extensions. 30 days provides a reasonable balance
/// between storage efficiency and cost. This is more than double the `BUMP_THRESHOLD`
/// to ensure that after extension, storage remains valid for a substantial period.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 60 days) reduces extension frequency but increases
///   the maximum storage cost per entry, as Soroban charges for the maximum TTL.
/// - Decreasing this value (e.g., to 15 days) increases extension frequency, potentially
///   raising transaction costs due to more frequent bump operations.
/// - Must be greater than `BUMP_THRESHOLD` to ensure storage doesn't expire between checks.
///
/// **Used for:** Calculating the new TTL when calling `extend_persistent` and `bump_ttl`.
pub const BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

// ═══════════════════════════════════════════════════════════════════════════
// POOL CONFIGURATION CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/// Default minimum pool duration in seconds (1 hour).
///
/// **Units:** Seconds
/// **Value:** 3,600 seconds (1 hour)
///
/// **Rationale:** Pools need sufficient time to attract participants and generate meaningful
/// betting activity. A 1-hour minimum prevents creation of ultra-short-lived pools that
/// would provide poor user experience and negligible engagement. This also gives oracle
/// systems time to update before resolution.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 24 hours) ensures longer engagement windows but
///   may limit the types of markets that can be created (e.g., rapid-response events).
/// - Decreasing this value (e.g., to 10 minutes) allows more granular, short-term markets
///   but may increase spam and reduce participation per pool.
/// - Can be overridden per-pool via configuration; this is only the default.
///
/// **Used for:** Validating `end_time - start_time` during pool creation.
pub const DEFAULT_MIN_POOL_DURATION: u64 = 3600;

/// Cancellation delay in seconds for overdue pools (7 days).
///
/// **Units:** Seconds
/// **Value:** 604,800 seconds (7 days)
///
/// **Rationale:** After a pool's `end_time` passes, it may remain unresolved due to oracle
/// delays, disputes, or operator inaction. This delay provides a grace period for normal
/// resolution before allowing anyone to cancel the pool. 7 days balances giving operators
/// sufficient time with preventing indefinite limbo states.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 30 days) gives operators more time to resolve pools
///   but leaves user funds locked longer if resolution is delayed.
/// - Decreasing this value (e.g., to 3 days) allows faster cancellation of stuck pools
///   but may trigger premature cancellations before operators can act.
/// - Affects the `cancel_pool` function's time-based authorization logic.
///
/// **Used for:** Determining when any user (not just operators) can cancel an overdue pool.
pub const CANCELATION_DELAY: u64 = 604_800;

/// Default global minimum stake amount (1 unit in token base units).
///
/// **Units:** Token base units, which are the smallest divisible units accepted
/// by the Stellar token contract. For native XLM this is stroops, where
/// 1 XLM = 10,000,000 stroops.
/// **Value:** 1 base unit/stroop-equivalent
///
/// **Rationale:** To prevent spam and dust transactions, there must be a minimum stake
/// requirement. Setting this to 1 (the smallest possible unit) allows maximum flexibility
/// while still rejecting zero or negative amounts. The actual effective minimum is
/// typically set higher via per-pool `min_stake` or admin configuration.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 1000) would reject very small stakes globally,
///   preventing participation from users with small balances but reducing spam.
/// - Decreasing this value is not possible (cannot go below 1).
/// - This is a hard floor; individual pools can have higher minimums via `pool.min_stake`.
/// - Can be overridden via `Config::min_stake` by admin.
///
/// **Used for:** Validating that `amount > 0` in `place_prediction` (via `InsufficientStake` error).
pub const DEFAULT_GLOBAL_MIN_STAKE: i128 = 1;

/// Default cooldown in seconds between consecutive place_prediction calls by the same user.
///
/// **Units:** Seconds
/// **Value:** 30 seconds (enabled by default)
///
/// **Rationale:** Rate limiting prevents spam and front-running attacks by limiting how
/// quickly a single user can place predictions. A 30-second cooldown slows down
/// rapid-fire prediction strategies that could be used to exploit temporary price
/// discrepancies or to front-run other users' predictions.
///
/// **Impact of changes:**
/// - Increasing this value enforces a stricter cooldown by default, preventing rapid
///   consecutive predictions but potentially frustrating legitimate users.
/// - Decreasing this value is not possible (cannot go below 0).
/// - Setting to 0 via admin disables the cooldown mechanism entirely.
/// - When enabled, the cooldown is enforced via `LastPredictionTime(user)` storage.
/// - Can be overridden via `Config::prediction_cooldown_seconds` by admin.
///
/// **Used for:** Initializing `Config::prediction_cooldown_seconds` during contract initialization.
pub const DEFAULT_PREDICTION_COOLDOWN_SECONDS: u64 = 30;

/// Maximum number of options/outcomes allowed in a single pool.
///
/// **Units:** Count (dimensionless)
/// **Value:** 100 options
///
/// **Rationale:** Each outcome requires storage entries and computational overhead during
/// resolution. Limiting to 100 prevents excessive gas costs, storage bloat, and UX complexity.
/// This supports large tournaments (e.g., 64-team brackets) while preventing unbounded complexity.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 200) would allow larger tournaments but increase
///   gas costs for pool creation and resolution, and may hit Soroban's storage limits.
/// - Decreasing this value (e.g., to 32) would reduce complexity but prevent certain market types.
/// - Affects storage layout: pools with >= 16 outcomes use batch storage (`OutStakes`) instead
///   of individual keys for efficiency.
///
/// **Used for:** Validating `options_count` during pool creation (via `InvalidData` error).
pub const MAX_OPTIONS_COUNT: u32 = 100;

/// Maximum initial liquidity that can be provided (100M tokens at 7 decimals).
///
/// **Units:** Token base units. For native XLM, these are stroops; for issued
/// Stellar assets, this is the asset's smallest contract unit.
/// **Value:** 100,000,000,000,000 base units
/// **Equivalent:** 100,000,000 tokens at 7 decimal places (e.g., 100M USDC)
///
/// **Rationale:** Initial liquidity represents "house money" provided by the pool creator
/// to bootstrap the market. This cap prevents excessive risk exposure for creators and
/// potential protocol-level liquidity imbalances. At 7 decimals (common for USDC on Stellar),
/// this equals 100 million tokens, a substantial but not unlimited amount.
///
/// **Impact of changes:**
/// - Increasing this value would allow larger initial liquidity, enabling bigger markets
///   but increasing creator risk and potential protocol exposure.
/// - Decreasing this value would limit market size, potentially preventing legitimate large-scale markets.
/// - The actual value in token terms depends on the token's decimal places:
///   - 7 decimals (USDC): 100,000,000 tokens
///   - 6 decimals (USDT): 1,000,000,000 tokens
///   - 7 decimals (XLM): 100,000,000 XLM
///
/// **Used for:** Validating `initial_liquidity` during pool creation.
pub const MAX_INITIAL_LIQUIDITY: i128 = 100_000_000_000_000;

/// Sentinel value used to indicate that a pool outcome has not been resolved yet.
///
/// **Units:** Outcome index (dimensionless)
/// **Value:** 4,294,967,295 (u32::MAX)
///
/// **Rationale:** When a pool is created, its outcome is unknown. We need a sentinel value
/// that cannot be confused with a valid outcome index (0, 1, 2, ...). Using `u32::MAX`
/// ensures this since valid outcomes are bounded by `MAX_OPTIONS_COUNT` (100). This allows
/// us to distinguish between "outcome 0 won" (a valid result) and "pool not resolved".
///
/// **Impact of changes:**
/// - This value should never be changed as it's used throughout the codebase as a
///   sentinel for unresolved pools.
/// - Changing this would require a storage migration to update all existing pools.
/// - Must be a value that cannot occur as a valid outcome index (given `MAX_OPTIONS_COUNT`).
///
/// **Used for:** Initializing `pool.outcome` and checking if a pool is resolved.
pub const UNRESOLVED_OUTCOME: u32 = u32::MAX;

/// Maximum allowed pool duration in seconds (365 days).
///
/// **Units:** Seconds
/// **Value:** 31,536,000 seconds (365 days)
///
/// **Rationale:** Long-running pools tie up user funds for extended periods and may become
/// irrelevant due to changing market conditions. A 1-year maximum ensures pools remain
/// timely and relevant while still allowing long-term predictions (e.g., annual events).
/// This also limits the duration over which oracle data must remain valid.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 2 years) would allow longer-term markets but increase
///   the risk of funds being locked for extended periods and oracle data becoming stale.
/// - Decreasing this value (e.g., to 180 days) would limit market types to shorter-term events.
/// - Affects the validation of `end_time - start_time` during pool creation.
///
/// **Used for:** Validating pool duration during creation (via `InvalidTimestamp` error).
pub const MAX_POOL_DURATION: u64 = 31_536_000;

/// Maximum length (in bytes/chars) of a single outcome description.
///
/// **Units:** Bytes/characters
/// **Value:** 128 bytes/characters
///
/// **Rationale:** Outcome descriptions (e.g., "Yes", "No", "Team A wins") are stored in
/// persistent storage. Without bounds, a malicious or careless creator could create
/// pools with extremely long descriptions, causing storage bloat and excessive gas costs.
/// 128 characters is sufficient for most outcome labels while preventing abuse.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 256) would allow more descriptive labels but increase
///   storage costs per outcome and potential for abuse.
/// - Decreasing this value (e.g., to 64) would reduce storage costs but may limit expressiveness
///   for certain market types (e.g., long team names).
/// - Affects storage size: `pool.outcome_descriptions` is a Vec<String>, so total storage
///   scales with `options_count * MAX_OUTCOME_DESCRIPTION_LEN`.
///
/// **Used for:** Validating outcome description lengths during pool creation (issue #1122).
pub const MAX_OUTCOME_DESCRIPTION_LEN: u32 = 128;

/// Minimum length (in bytes/chars) of a single outcome description.
///
/// **Units:** Bytes/characters
/// **Value:** 1 byte/character
///
/// **Rationale:** Empty or whitespace-only outcome descriptions provide no useful information
/// to users and may indicate errors or malicious intent. Requiring at least 1 character
/// ensures all outcomes have meaningful labels.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 3) would prevent single-character labels like "A" or "B",
///   which may be too restrictive for some use cases.
/// - Decreasing this value is not possible (cannot go below 1).
/// - Works in conjunction with `MAX_OUTCOME_DESCRIPTION_LEN` to bound description length.
///
/// **Used for:** Validating outcome description lengths during pool creation (issue #1122).
pub const MIN_OUTCOME_DESCRIPTION_LEN: u32 = 1;

/// Initial-liquidity safety margin in basis points relative to `max_total_stake`.
///
/// **Units:** Basis points (bps, where 100 bps = 1%)
/// **Value:** 100 bps (1%)
///
/// **Rationale:** When a pool creator sets a `max_total_stake` cap and provides initial
/// liquidity (house money), there's a risk that early large bets could drain the pool
/// before the creator's liquidity provides meaningful coverage. This margin requires that
/// initial liquidity be at least 1% of `max_total_stake`, ensuring the creator has skin
/// in the game and the pool has sufficient buffer against early manipulation.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 500 bps = 5%) would require more initial liquidity,
///   reducing pool creation flexibility but increasing safety against early draining.
/// - Decreasing this value (e.g., to 50 bps = 0.5%) would allow pools with less initial
///   liquidity but increase the risk of early pool draining attacks.
/// - Only applies when `max_total_stake > 0`; pools without a cap are not subject to this check.
///
/// **Used for:** Validating `initial_liquidity` relative to `max_total_stake` during pool creation (issue #1131).
pub const INITIAL_LIQUIDITY_SAFETY_MARGIN_BPS: u32 = 100;

/// Multisig threshold for the emergency-cancel flow.
///
/// **Units:** Count (dimensionless)
/// **Value:** 2 approvals
///
/// **Rationale:** Emergency cancellation is a powerful action that should require consensus
/// to prevent abuse. Requiring at least 2 distinct operator/admin approvals ensures that
/// no single actor can unilaterally cancel a pool, providing a check against malicious
/// or mistaken emergency cancellations.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 3) would require more consensus, making emergency
///   cancellation harder but safer against collusion.
/// - Decreasing this value to 1 would allow single-actor emergency cancellation, increasing
///   the risk of abuse.
/// - Must be <= the number of active operators/admins, or pools can never be emergency-cancelled.
///
/// **Used for:** Validating that sufficient approvals have been collected before executing
/// `emergency_cancel_pool` (issue #1119).
pub const EMERGENCY_CANCEL_MULTISIG_THRESHOLD: u32 = 2;

// ═══════════════════════════════════════════════════════════════════════════
// MONITORING & ALERT THRESHOLDS
// ═══════════════════════════════════════════════════════════════════════════

/// Stake amount above which a `HighValuePredictionEvent` is emitted.
///
/// **Units:** Token base units. For XLM-denominated pools, this value is in
/// stroops; for other Stellar tokens it uses that token contract's base units.
/// **Value:** 1,000,000,000 base units
/// **Equivalent:** 100 tokens at 7 decimal places (e.g., 100 USDC)
///
/// **Rationale:** Large stakes represent significant user exposure and potential market impact.
/// Emitting a special event when stakes exceed this threshold allows off-chain monitoring
/// systems to apply extra scrutiny, detect potential manipulation, or trigger alerts.
/// At 7 decimals (common for USDC), this equals 100 USDC, a reasonable threshold for "large" bets.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 10,000,000,000 = 1000 USDC) would reduce the frequency
///   of high-value events, potentially missing significant bets.
/// - Decreasing this value (e.g., to 100,000,000 = 10 USDC) would increase event frequency,
///   potentially creating noise in monitoring systems.
/// - The actual token value depends on the token's decimal places.
///
/// **Used for:** Triggering `HighValuePredictionEvent` in `place_prediction`.
pub const HIGH_VALUE_THRESHOLD: i128 = 1_000_000_000;

/// Maximum tolerance in basis points (1 bp = 0.01%).
///
/// **Units:** Basis points (bps)
/// **Value:** 10,000 bps (100%)
///
/// **Rationale:** Price conditions (e.g., "price within 5% of target") use basis points
/// to express tolerance ranges. This constant represents 100% tolerance, serving as the
/// denominator for tolerance calculations. Using basis points allows precise percentage-based
/// conditions without floating-point arithmetic.
///
/// **Impact of changes:**
/// - This value should never be changed as it represents the mathematical definition of
///   100% in basis points.
/// - Changing this would break all existing price condition calculations.
/// - Used as a scaling factor: `tolerance_amount = (base_amount * tolerance_bps) / MAX_TOLERANCE`.
///
/// **Used for:** Calculating tolerance ranges in price-based oracle conditions.
pub const MAX_TOLERANCE: u32 = 10_000;

/// Maximum number of primitive checks allowed while matching a price condition.
///
/// **Units:** Count (dimensionless)
/// **Value:** 4 checks
///
/// **Rationale:** Price condition matching (e.g., "price > X AND price < Y") involves
/// evaluating primitive checks against oracle data. To ensure resolution remains O(1)
/// and gas-predictable, we bound the number of checks. This prevents future condition
/// logic from becoming unbounded and causing resolution failures due to gas limits.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 8) would allow more complex conditions but increase
///   gas costs and potential for resolution failures.
/// - Decreasing this value (e.g., to 2) would limit condition expressiveness, potentially
///   preventing useful market types.
/// - This is a hard limit on condition complexity; exceeding it causes resolution to fail.
///
/// **Used for:** Validating price condition complexity during oracle-based resolution.
pub const MAX_PRICE_CONDITION_MATCH_STEPS: u32 = 4;

// ═══════════════════════════════════════════════════════════════════════════
// VERSION CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/// Current contract version. Bump on each release to support safe migrations.
///
/// **Units:** Version number (dimensionless)
/// **Value:** 1
///
/// **Rationale:** Contract upgrades require version tracking to enable safe state migrations.
/// This version is stored in instance storage and checked during upgrades to ensure
/// compatibility. Each release should increment this value to trigger migration logic
/// if needed.
///
/// **Impact of changes:**
/// - Must be incremented on each contract release that requires state migration.
/// - Should not be decremented (versions are monotonic).
/// - Used by upgrade logic to determine which migration steps to execute.
/// - Changing this without proper migration logic can cause upgrade failures or data corruption.
///
/// **Used for:** Version tracking in instance storage and upgrade migration logic.
pub const CONTRACT_VERSION: u32 = 1;

/// Minimum timelock delay in seconds for protocol fee changes.
///
/// **Units:** Seconds
/// **Value:** 86,400 seconds (1 day)
///
/// **Rationale:** Protocol fee changes are sensitive parameters that affect user economics.
/// A timelock delay ensures users and integrators have time to observe pending fee changes
/// and react (e.g., withdraw positions, adjust strategies) before the new fee takes effect.
/// This prevents surprise fee hikes and provides governance transparency.
///
/// **Impact of changes:**
/// - Increasing this value (e.g., to 604,800 = 7 days) would provide more reaction time
///   but slow down legitimate fee adjustments.
/// - Decreasing this value (e.g., to 43,200 = 12 hours) would allow faster fee changes
///   but reduce the window for users to react.
/// - Affects the `set_fee_bps` → `apply_fee_bps` workflow via `PendingFeeChange.effective_at`.
/// - Too short a timelock could enable governance attacks or user harm.
///
/// **Used for:** Calculating `PendingFeeChange.effective_at` when queuing fee changes.
pub const FEE_CHANGE_TIMELOCK_SECONDS: u64 = 86_400;

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
    fn test_high_value_threshold_equals_100_usdc() {
        // At 7 decimals, 1 USDC = 10_000_000 base units.
        // Therefore 1_000_000_000 base units equals 100 USDC.
        assert_eq!(HIGH_VALUE_THRESHOLD, 1_000_000_000);
    }
}
