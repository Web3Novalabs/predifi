#![no_std]
#![allow(clippy::too_many_arguments)]

#[cfg(test)]
#[macro_use]
extern crate std;

mod admin;
#[cfg(test)]
mod benchmark_test;
#[cfg(test)]
mod boundary_edge_case_tests;
#[cfg(test)]
mod oracle_edge_case_tests;
mod constants;
mod gas_opt;
mod oracle;
#[cfg(test)]
mod payout_proptests;
mod payouts;
mod pool;
mod prediction;
mod price_feed;
mod price_feed_simple;
mod referral;
mod safe_math;
#[cfg(test)]
mod safe_math_examples;
#[cfg(test)]
mod access_control_audit_tests;
#[cfg(test)]
mod storage_test;
#[cfg(test)]
mod stress_test;
#[cfg(test)]
mod stress_test_high_volume;
// #[cfg(test)]
// mod stress_test_max_pools;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod pause_unpause_boundary_tests;
// #[cfg(test)]
// mod max_pools_stress_tests;
#[cfg(test)]
mod update_pool_description_boundary_tests;
#[cfg(test)]
mod withdraw_treasury_boundary_tests;
mod treasury;
mod creator_incentive;

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, symbol_short, token,
    Address, BytesN, Env, IntoVal, String, Symbol, SymbolStr, TryFromVal, Vec,
};

pub use constants::*;
pub use payouts::{
    calculate_claim_payout, calculate_odds_bps, calculate_protocol_fee, calculate_referral_amount,
    calculate_winnings, PayoutBreakdown, PayoutInput,
};
pub use price_feed_simple::PriceFeedAdapter;
pub use safe_math::{RoundingMode, SafeMath};
pub use pool::{
    CreatePoolParams,
    InitialLiquidityProvidedEvent,
    MaxTotalStakeIncreasedEvent,
    Pool,
    PoolCanceledEvent,
    PoolConfig,
    PoolCreatedEvent,
    PoolDescriptionUpdatedEvent,
    PoolDisputedEvent,
    PoolReadyForResolutionEvent,
    PoolResolvedEvent,
    PoolStats,
    StakeLimitsUpdatedEvent,
    StakingClosedEvent,
};
pub use prediction::{
    MarketState,
    OutcomeStakesUpdatedEvent,
    Prediction,
    PredictionBlockedDelistedEvent,
    PredictionPlacedEvent,
    RefundClaimedEvent,
    UserPredictionDetail,
    WinningsClaimedEvent,
};
pub use admin::{
    AddedToWhitelistEvent,
    Config,
    ContractInfo,
    ContractMetadata,
    ContractUpgradedEvent,
    FeeChangeCancelEvent,
    FeeChangeProposeEvent,
    FeeConfig,
    FeeInfo,
    FeeTier,
    FeeTiersUpdateEvent,
    FeeUpdateEvent,
    InitEvent,
    MaxPredictionsUpdateEvent,
    MinPoolDurationUpdateEvent,
    MinStakeUpdateEvent,
    PauseEvent,
    PendingFeeChange,
    PredictionCooldownUpdateEvent,
    RemovedFromWhitelistEvent,
    ResolutionDelayUpdateEvent,
    StorageTtlRenewedEvent,
    TokenWhitelistAddedEvent,
    TokenWhitelistRemovedEvent,
    TreasuryUpdateEvent,
    UnpauseEvent,
    UpgradeEvent,
};
pub use oracle::{
    OracleInitEvent,
    OracleResolvedEvent,
    OracleWhitelistAddedEvent,
    OracleWhitelistRemovedEvent,
    PriceConditionSetEvent,
    PriceFeedUpdatedEvent,
    PriceFeedsCleanedEvent,
    PriceResolvedEvent,
    ResolutionConflictEvent,
    ResolutionVoteCastEvent,
};
pub use referral::{
    ReferralPaidEvent,
    ReferralThresholdUpdatedEvent,
    ReferrerUpdatedEvent,
    RewardClaimedEvent,
};
pub use treasury::{
    EmergencyWithdrawEvent,
    TreasuryWithdrawnEvent,
};

// ═══════════════════════════════════════════════════════════════════════════
// ACCESS CONTROL — ROLES & PERMISSIONS
// ═══════════════════════════════════════════════════════════════════════════
//
// Roles are managed by the companion `access-control` contract and are
// referenced here by their numeric discriminant.  The `require_role` helper
// cross-calls `access_control::has_role(user, role)` at runtime.
//
// ┌──────────┬───────┬──────────────────────────────────────────────────────┐
// │ Role     │ Value │ Permitted operations in predifi-contract              │
// ├──────────┼───────┼──────────────────────────────────────────────────────┤
// │ Admin    │   0   │ pause / unpause                                       │
// │          │       │ set_fee_bps                                           │
// │          │       │ set_treasury                                          │
// │          │       │ set_resolution_delay                                  │
// │          │       │ set_referral_cut_bps                                  │
// │          │       │ add_token_to_whitelist / remove_token_from_whitelist  │
// │          │       │ withdraw_treasury                                     │
// │          │       │ upgrade_contract                                      │
// │          │       │ migrate_state                                         │
// ├──────────┼───────┼──────────────────────────────────────────────────────┤
// │ Operator │   1   │ resolve_pool (multi-vote; finalises when threshold    │
// │          │       │   of required_resolutions is reached)                 │
// │          │       │ cancel_pool                                           │
// │          │       │ set_stake_limits                                      │
// ├──────────┼───────┼──────────────────────────────────────────────────────┤
// │ Oracle   │   3   │ oracle_resolve (OracleCallback trait; multi-vote;     │
// │          │       │   finalises when required_resolutions threshold met)  │
// └──────────┴───────┴──────────────────────────────────────────────────────┘
//
// Note: roles 2 (Moderator) and 4 (User) are defined in the access-control
// contract but are not currently enforced by predifi-contract.
// Role 2 (Moderator) is RESERVED FOR FUTURE USE — it is intended for dispute
// resolution functionality. See issue #595 for the implementation plan.
//
// HOW ROLES ARE ASSIGNED
// ──────────────────────
// 1. Deploy the `access-control` contract and call `access_control::init(admin)`
//    to set the initial administrator.
// 2. The admin calls `access_control::assign_role(admin, user, Role::Operator)`
//    (or `Role::Oracle`, etc.) to grant a role to any address.
// 3. Roles can be revoked with `access_control::revoke_role`, transferred with
//    `access_control::transfer_role`, or bulk-cleared with `revoke_all_roles`.
// 4. Admin authority itself can be transferred via `access_control::transfer_admin`.
// 5. Pass the deployed access-control contract address to
//    `predifi_contract::init(access_control, treasury, fee_bps, resolution_delay)`
//    so the predifi contract knows which access-control instance to query.
//
// ═══════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════
// MARKET CATEGORY CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════
//
// Canonical set of market category symbols. All categories use PascalCase
// convention and are ≤9 characters for compile-time symbol optimization.
//
// These constants define the allowed categories for prediction pools.
// Any pool creation must specify one of these categories.

/// Sports-related prediction markets (e.g., game outcomes, tournaments)
pub const CATEGORY_SPORTS: Symbol = symbol_short!("Sports");

/// Financial markets and economic predictions (e.g., stock prices, indices)
pub const CATEGORY_FINANCE: Symbol = symbol_short!("Finance");

/// Cryptocurrency and blockchain-related predictions (e.g., token prices, network events)
pub const CATEGORY_CRYPTO: Symbol = symbol_short!("Crypto");

/// Political events and elections
pub const CATEGORY_POLITICS: Symbol = symbol_short!("Politics");

/// Entertainment industry predictions (e.g., awards, box office)
pub const CATEGORY_ENTERTAIN: Symbol = symbol_short!("Entertain");

/// Technology and innovation predictions (e.g., product launches, tech trends)
pub const CATEGORY_TECH: Symbol = symbol_short!("Tech");

/// Maximum allowed resolution delay: 30 days in seconds
pub const MAX_RESOLUTION_DELAY: u64 = 2_592_000;

/// Minimum claim window: 1 day in seconds
pub const MIN_CLAIM_WINDOW: u64 = 86_400;

/// Maximum claim window: 365 days in seconds
pub const MAX_CLAIM_WINDOW: u64 = 31_536_000;

/// Miscellaneous predictions that don't fit other categories
pub const CATEGORY_OTHER: Symbol = symbol_short!("Other");

/// Minimum amount (in token base units / stroops) that may be withdrawn
/// via `withdraw_treasury`. Prevents dust withdrawals.
pub const MIN_WITHDRAWAL_AMOUNT: i128 = 1;

// ═══════════════════════════════════════════════════════════════════════════
// PROTOCOL INVARIANTS (for formal verification)
// ═══════════════════════════════════════════════════════════════════════════
//
// INV-1: Pool.total_stake = Σ(OutcomeStake(pool_id, outcome)) for all outcomes
// INV-2: Pool.state transitions: Active → {Resolved | Canceled}, never reversed
// INV-3: HasClaimed(user, pool) is write-once (prevents double-claim)
// INV-4: Winnings ≤ Pool.total_stake (no value creation)
// INV-5: For resolved pools: Σ(claimed_winnings) ≤ Pool.total_stake
// INV-6: Config.fee_bps ≤ 10_000 (max 100%)
// INV-7: Prediction.amount > 0 (no zero-stakes)
// INV-8: Pool.end_time > creation_time (pools must have future end)
//
// ═══════════════════════════════════════════════════════════════════════════

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PredifiError {
    AlreadyInitializedOrConfigNotSet = 2,
    Unauthorized = 10,
    PoolNotFound = 20,
    PoolNotResolved = 22,
    InvalidPoolState = 24,
    /// The outcome value is invalid or out of bounds.
    InvalidOutcome = 25,
    AlreadyClaimed = 60,
    PoolCanceled = 70,
    ResolutionDelayNotMet = 81,
    /// Token is not on the allowed betting whitelist.
    TokenNotWhitelisted = 91,
    /// Invalid amount provided (e.g., zero or negative).
    InvalidAmount = 42,
    /// Insufficient balance for the operation.
    InsufficientBalance = 44,
    /// Oracle not initialized.
    OracleNotInitialized = 100,
    /// Price feed not found.
    PriceFeedNotFound = 101,
    /// Price data expired or invalid.
    PriceDataInvalid = 102,
    /// Price data is older than the configured `max_price_age`.
    PriceStale = 113,
    /// Price condition not set for pool.
    PriceConditionNotSet = 103,
    /// Total pool stake cap reached or would be exceeded.
    MaxTotalStakeExceeded = 104,
    /// Oracles disagree on the outcome.
    ResolutionConflict = 105,
    /// This oracle has already cast a vote for this pool.
    OracleAlreadyVoted = 106,
    /// Stake amount is below the pool minimum.
    StakeBelowMinimum = 107,
    /// Stake amount exceeds the pool maximum.
    StakeAboveMaximum = 108,
    /// Stake amount is below the global protocol minimum.
    InsufficientStake = 45,
    /// User has exceeded the maximum number of predictions allowed per pool.
    MaxPredictionsExceeded = 111,
    /// The fee basis points exceed the maximum allowed value (10000).
    InvalidFeeBps = 93,
    /// Metadata URL exceeds maximum length (512 bytes).
    MetadataUrlInvalid = 109,
    /// An arithmetic overflow, underflow, or division by zero occurred.
    ArithmeticError = 110,
    /// required_resolutions exceeds the number of active operators; pool can never be resolved.
    RequiredResolutionsExceedOperators = 200,
    /// Rate limit exceeded, cooldown active, or suspicious activity detected.
    RateLimitOrSuspiciousActivity = 190,
    /// The pagination offset + limit combination overflows u32 or is otherwise invalid.
    InvalidPagination = 92,
    /// Generic invalid input data (e.g., a zero value where a positive value is required).
    InvalidData = 90,
    /// The provided timestamp is invalid (e.g., end_time too far in the future).
    InvalidTimestamp = 80,
    /// Pool has been flagged as disputed and cannot be modified.
    PoolDisputed = 27,
    /// target_price must be strictly positive.
    InvalidTargetPrice = 201,
    /// `close_staking` called before pool.end_time has passed.
    StakingStillOpen = 82,
    /// A time-window constraint (e.g. resolution window, claim window) is
    /// not met for the requested operation.
    TimeConstraintError = 84,
    /// One of the `outcome_descriptions` exceeds `MAX_OUTCOME_DESCRIPTION_LEN`
    /// (issue #1122).
    OutcomeDescriptionTooLong = 130,
    /// One of the `outcome_descriptions` is empty / shorter than
    /// `MIN_OUTCOME_DESCRIPTION_LEN` (issue #1122).
    OutcomeDescriptionEmpty = 131,
    /// A timestamp that must be in the future is in the past or equal to
    /// `env.ledger().timestamp()` (issue #1130).
    DeadlineInPast = 132,
    /// `initial_liquidity` is less than the required safety margin
    /// relative to `max_total_stake` (issue #1131).
    InitialLiquidityBelowSafetyMargin = 133,
    /// `emergency_cancel_pool` was called but the multisig threshold
    /// has not yet been reached (issue #1119).
    EmergencyCancelPending = 134,
    /// The caller has already approved this emergency-cancel proposal
    /// (issue #1119).
    EmergencyCancelAlreadyApproved = 135,
    /// The contract is currently paused; all state-mutating operations are blocked.
    ///
    /// Callers should check `is_contract_paused()` before submitting a transaction,
    /// or listen for `PauseEvent` / `UnpauseEvent` on-chain to stay in sync.
    ContractPaused = 83,
    InvalidAddressOrToken = 94,
    FeeChangePending = 95,
    NoFeeChangePending = 96,
    TimelockNotExpired = 97,
    /// Issue #1933: pool's `participants_count` is below the configured
    /// `get_creator_min_participants()` threshold, so the creator incentive
    /// cannot be claimed yet.
    MinParticipantsNotMet = 210,
    /// Issue #1933: `claim_creator_incentive` has already been paid out for
    /// this pool — it can only be claimed once per pool.
    CreatorIncentiveAlreadyPaid = 211,
}

/// Internal storage keys for contract data.
///
/// All variants use PascalCase. Abbreviated names are preserved for existing
/// on-chain keys to avoid storage migration (Soroban uses the variant name as
/// the XDR discriminant). New variants added here use full descriptive names.
///
/// # Naming conventions
/// - Existing abbreviated variants (e.g. `OutStake`, `UsrPrdCnt`) are kept
///   verbatim to preserve on-chain discriminant values.
/// - New variants added after the initial deployment use full PascalCase names
///   (e.g. `OracleConfig`, `PriceFeed`, `PriceCondition`).
/// - All variants are documented with their storage type mapping.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    // ── Pool data ────────────────────────────────────────────────────────────
    /// Pool data by pool ID: `Pool(pool_id)` -> `Pool`
    Pool(u64),
    /// Pool ID counter for generating unique pool IDs: `PoolIdCtr` -> `u64`
    PoolIdCtr,

    // ── Predictions & stakes ─────────────────────────────────────────────────
    /// User prediction by user address and pool ID: `Pred(user, pool_id)` -> `Prediction`
    Pred(Address, u64),
    /// Tracks whether a user has claimed winnings for a pool: `Claimed(user, pool_id)` -> `bool`
    Claimed(Address, u64),
    /// Stake amount for a specific outcome (backward-compat individual key):
    /// `OutStake(pool_id, outcome)` -> `i128`
    OutStake(u64, u32),
    /// Optimized batch storage for all outcome stakes in a pool:
    /// `OutStakes(pool_id)` -> `Vec<i128>`
    ///
    /// Preferred over `OutStake` for pools with many outcomes. Falls back to
    /// `OutStake` for backward compatibility when this key is absent.
    OutStakes(u64),
    /// User prediction count: `UsrPrdCnt(user)` -> `u32`
    UsrPrdCnt(Address),
    /// User prediction index: `UsrPrdIdx(user, index)` -> `UserPredictionDetail`
    UsrPrdIdx(Address, u32),
    /// Last successful prediction timestamp for a user: `LastPredictionTime(user)` -> `u64`
    LastPredictionTime(Address),

    // ── Protocol configuration ───────────────────────────────────────────────
    /// Global protocol configuration: `Config` -> `Config`
    Config,
    /// Contract pause state: `Paused` -> `bool`
    Paused,
    /// Contract version for safe upgrade migrations: `Version` -> `u32`
    Version,
    /// Referral cut in basis points: `ReferralCutBps` -> `u32`
    ReferralCutBps,
    /// Reentrancy guard (temporary storage): `RentGuard` -> `bool`
    RentGuard,

    // ── Token whitelist ──────────────────────────────────────────────────────
    /// Token whitelist entry: `TokenWl(token_address)` -> `bool`
    ///
    /// Present (with value `true`) when the token is allowed for betting.
    TokenWl(Address),
    /// Whitelisted tokens list: `TokenWhitelist` -> `Vec<Address>`
    ///
    /// Maintains an ordered list of all whitelisted token addresses for efficient enumeration.
    TokenWhitelist,

    // ── Categories ───────────────────────────────────────────────────────────
    /// Category pool count: `CatPoolCt(category)` -> `u32`
    CatPoolCt(Symbol),
    /// Category pool index: `CatPoolIx(category, index)` -> `u64` (pool_id)
    CatPoolIx(Symbol, u32),

    // ── Resolution voting (TEMPORARY STORAGE) ────────────────────────────────
    /// Tracks if an oracle/operator has already voted (temporary): `ResVote(pool_id, voter_address)` -> `()`
    /// Stored in temporary storage as it's only needed during resolution process.
    ResVote(u64, Address),
    /// Vote count for a specific outcome (temporary): `ResVoteCt(pool_id, outcome)` -> `u32`
    /// Stored in temporary storage as it's only needed during resolution process.
    ResVoteCt(u64, u32),
    /// Total number of votes cast for a pool (temporary): `ResTotal(pool_id)` -> `u32`
    /// Stored in temporary storage as it's only needed during resolution process.
    ResTotal(u64),

    // ── Referrals ────────────────────────────────────────────────────────────
    /// Referred volume for a referrer and pool: `ReferredVolume(referrer, pool_id)` -> `i128`
    ReferredVolume(Address, u64),
    /// Referrer address for a user and pool: `Referrer(user, pool_id)` -> `Address`
    ///
    /// FUTURE: Multiple referrers per user per pool
    /// Currently a user can only have one referrer per pool. If multiple referrers are needed
    /// (e.g. to split the referral share among several parties), this key should be changed to
    /// store a `Map<Address, u32>` (referrer -> share_bps) or a `Vec<Address>` with equal splits.
    /// The `ReferredVolume` key would similarly need to become per-(referrer, user, pool) or be
    /// aggregated differently. The payout loop in `claim_winnings` would iterate over all referrers
    /// and distribute proportional cuts. Until that requirement is confirmed, the single-referrer
    /// model is kept for simplicity and gas efficiency.
    Referrer(Address, u64),

    // ── Private pools ────────────────────────────────────────────────────────
    /// User whitelist for private pools: `Whitelist(pool_id, user_address)` -> `()`
    Whitelist(u64, Address),
    // Global active pool counter: ActivePoolCtr -> u32
    ActivePoolCtr,
    /// Global active pool index: ActivePool(index) -> u64 (pool_id)
    ActivePool(u32),
    /// Reverse lookup — position of a pool in the active index: ActivePoolIdx(pool_id) -> u32
    ActivePoolIdx(u64),
    /// Price condition for automated resolution: PriceCondition(pool_id) -> (feed_pair, target_price, operator, tolerance_bps)
    PriceCondition(u64),
    /// Latest price feed data: PriceFeed(feed_pair) -> (price, confidence, timestamp, expires_at)
    PriceFeed(Symbol),
    /// Tracked list of all registered feed pairs for cleanup: PriceFeedList -> Vec<Symbol>
    PriceFeedList,
    FeeTiers,
    /// Oracle configuration for price feed validation
    OracleConfig,
    /// Oracle whitelist entry: OracleWl(oracle_address) -> bool
    OracleWl(Address),
    /// Whitelisted oracle list: OracleWhitelist -> Vec<Address>
    OracleWhitelist,
    /// Disputed flag for a pool: Disputed(pool_id) -> ()
    Disputed(u64),
    /// Sentinel that records staking has been closed for a pool:
    /// `StakingClosed(pool_id)` -> `bool`
    ///
    /// Written (once) the first time `close_staking` is successfully called
    /// for a given pool so that subsequent calls are idempotent — they return
    /// `Ok(())` but do NOT re-emit the `StakingClosedEvent`.
    StakingClosed(u64),
    /// Pending protocol fee change awaiting timelock expiry:
    /// `PendingFeeBps` -> `PendingFeeChange`
    ///
    /// Present only while a fee proposal is queued (between a `set_fee_bps` call
    /// and the corresponding `apply_fee_bps` or `cancel_fee_proposal` call).
    PendingFeeBps,
    /// Minimum referred volume (in base token units) required before a referrer
    /// is eligible to receive a referral reward on claim.
    /// `ReferralMinVolumeBps` -> `i128`
    ///
    /// If the referrer's total referred volume for the pool is below this
    /// threshold the referral cut is silently skipped (not paid out).
    /// Default: 0 (no threshold — any volume qualifies).
    ReferralMinVolumeBps,

    // ── Issue #1119: Multi-sig emergency cancellation ───────────────────────
    /// Pending emergency-cancel approver set: `EmergencyCancelApprovers(pool_id)` -> `Vec<Address>`.
    /// Empty / absent when no emergency cancel is currently pending.
    EmergencyCancelApprovers(u64),
    /// Optional reason string captured when the first approval is recorded.
    /// `EmergencyCancelReason(pool_id)` -> `String`.
    EmergencyCancelReason(u64),
    /// Sentinel that records a pool has been marked ready for resolution.
    /// `PoolReady(pool_id)` -> `bool`.
    PoolReady(u64),

    // ── Issue #1933: Pool creator incentive system ──────────────────────────
    /// Creator incentive rate in basis points (share of a pool's protocol fee
    /// paid to its creator once the participation threshold is met):
    /// `CreatorIncentiveBps` -> `u32`. Default 0 (disabled) when unset.
    CreatorIncentiveBps,
    /// Minimum `Pool.participants_count` required before a pool's creator is
    /// eligible to claim the incentive: `CreatorMinParticipants` -> `u32`.
    /// Falls back to `DEFAULT_CREATOR_MIN_PARTICIPANTS` when unset.
    CreatorMinParticipants,
    /// Idempotency sentinel — set once `claim_creator_incentive` has paid out
    /// for a given pool: `CreatorIncentivePaid(pool_id)` -> `bool`.
    CreatorIncentivePaid(u64),
    /// Reputation counter: number of pools for which a creator has
    /// successfully claimed the incentive: `CreatorRewardedPools(creator)` -> `u32`.
    CreatorRewardedPools(Address),
    /// Reputation counter: cumulative incentive amount a creator has earned
    /// across all pools: `CreatorTotalIncentive(creator)` -> `i128`.
    CreatorTotalIncentive(Address),
}

// ── Events ───────────────────────────────────────────────────────────────────

// ── Monitoring & Alert Events ─────────────────────────────────────────────────
// These events are classified by severity and are intended for consumption by
// off-chain monitoring tools (Horizon event streaming, Grafana, SIEM, etc.).
// See MONITORING.md at the repo root for scraping patterns and alert rules.

/// 🔴 HIGH ALERT — emitted when `resolve_pool` is called by an address that
/// does not hold the Operator role.  Indicates a potential attack or
/// misconfigured access-control contract.
#[contractevent(topics = ["unauthorized_resolution"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnauthorizedResolveAttemptEvent {
    /// The address that attempted to resolve without authorization.
    pub caller: Address,
    /// The pool that was targeted.
    pub pool_id: u64,
    /// Ledger timestamp at the time of the attempt.
    pub timestamp: u64,
}

/// 🔴 HIGH ALERT — emitted when an admin-restricted operation (`set_fee_bps`,
/// `set_treasury`, `pause`, `unpause`) is called by an address that does not
/// hold the Admin role.
#[contractevent(topics = ["unauthorized_admin_op"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnauthorizedAdminAttemptEvent {
    /// The address that attempted the restricted operation.
    pub caller: Address,
    /// Short name of the operation that was attempted.
    pub operation: Symbol,
    /// Ledger timestamp at the time of the attempt.
    pub timestamp: u64,
}

/// 🔴 HIGH ALERT — emitted when `claim_winnings` is called after winnings have
/// already been claimed for the same (user, pool) pair.  Repeated attempts may
/// indicate a re-entrancy probe or a front-end bug worth investigating.
#[contractevent(topics = ["double_claim_attempt"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuspiciousDoubleClaimEvent {
    /// The address that attempted to double-claim.
    pub user: Address,
    /// The pool for which the claim was already made.
    pub pool_id: u64,
    /// Ledger timestamp at the time of the attempt.
    pub timestamp: u64,
}

/// 🔴 HIGH ALERT — emitted alongside `PauseEvent` whenever the contract is
/// successfully paused.  Having a dedicated alert topic makes it easy to set
/// a zero-tolerance PagerDuty rule that fires on any pause.
#[contractevent(topics = ["contract_paused_alert"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractPausedAlertEvent {
    /// The admin that triggered the pause.
    pub admin: Address,
    /// Ledger timestamp at pause time.
    pub timestamp: u64,
}

/// 🟡 MEDIUM ALERT — emitted in `place_prediction` when the staked amount
/// meets or exceeds `HIGH_VALUE_THRESHOLD`.  Useful for liquidity monitoring
/// and detecting unusual betting patterns.
#[contractevent(topics = ["high_value_prediction"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighValuePredictionEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
    pub outcome: u32,
    /// The threshold that was breached (aids display in dashboards).
    pub threshold: i128,
}

/// 🟢 INFO — emitted alongside `PoolResolvedEvent` with enriched numeric
/// context so monitors can calculate implied payouts and flag anomalies
/// (e.g., winning_stake == 0 meaning no winners).
#[contractevent(topics = ["pool_resolved_diag"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolResolvedDiagEvent {
    pub pool_id: u64,
    pub outcome: u32,
    /// Total stake across all outcomes at resolution time.
    pub total_stake: i128,
    /// Stake on the winning outcome (0 ⟹ no winners — notable anomaly).
    pub winning_stake: i128,
    /// Ledger timestamp at resolution time.
    pub timestamp: u64,
}

mod events;
use events::ClaimWindowUpdateEvent;
// pub use events::*; // Unused import

// ── Issue #1142: Event emission consistency ───────────────────────────────────

// ── Issue #1137: Contract metadata struct ────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────

pub trait OracleCallback {
    /// Resolve a pool based on external oracle data.
    /// Caller must have Oracle role (3).
    /// Cannot resolve a canceled pool.
    fn oracle_resolve(
        env: Env,
        oracle: Address,
        pool_id: u64,
        outcome: u32,
        proof: String,
    ) -> Result<(), PredifiError>;
}

#[contract]
pub struct PredifiContract;

#[contractimpl]
impl PredifiContract {
    // ====== Pure Helper Functions (side-effect free, verifiable) ======

    /// Validate that a category symbol is in the allowed list.
    /// Returns the category if valid, otherwise falls back to CATEGORY_OTHER.
    /// PRE: category is a valid Symbol
    /// POST: returns Ok(category) if category is in the allowed list, else Err(InvalidData)
    fn validate_category(env: &Env, category: &Symbol) -> Result<Symbol, PredifiError> {
        let mut allowed = Vec::new(env);
        allowed.push_back(CATEGORY_SPORTS);
        allowed.push_back(CATEGORY_FINANCE);
        allowed.push_back(CATEGORY_CRYPTO);
        allowed.push_back(CATEGORY_POLITICS);
        allowed.push_back(CATEGORY_ENTERTAIN);
        allowed.push_back(CATEGORY_TECH);
        allowed.push_back(CATEGORY_OTHER);

        for i in 0..allowed.len() {
            if let Some(allowed_cat) = allowed.get(i) {
                if &allowed_cat == category {
                    return Ok(category.clone());
                }
            }
        }
        Err(PredifiError::InvalidData)
    }

    /// Validate that a private-pool whitelist key meets the referral code format.
    ///
    /// A valid code must be 6–12 characters long and consist only of uppercase ASCII
    /// letters (`A`–`Z`) and digits (`0`–`9`). Lowercase letters, spaces, and special
    /// characters are all rejected so that codes remain URL-safe and easy to share.
    ///
    /// Called by `create_pool` when `config.whitelist_key` is `Some`.
    ///
    /// # Errors
    /// Returns `Err(PredifiError::InvalidData)` if the code is too short, too long,
    /// or contains any character outside `[A-Z0-9]`.
    fn validate_referral_code(env: &Env, code: &Symbol) -> Result<(), PredifiError> {
        let code_str = SymbolStr::try_from_val(env, &code.to_symbol_val())
            .map_err(|_| PredifiError::InvalidData)?;
        let code_bytes: &[u8] = code_str.as_ref();
        let len = code_bytes.len();

        if !(6..=12).contains(&len) {
            return Err(PredifiError::InvalidData);
        }

        for byte in code_bytes {
            if !matches!(*byte, b'A'..=b'Z' | b'0'..=b'9') {
                return Err(PredifiError::InvalidData);
            }
        }

        Ok(())
    }

    /// Validate core protocol invariants for a pool.
    /// Panics if any invariant is broken to prevent corrupted state from causing
    /// index-out-of-bounds or other logic errors in downstream processing.
    fn validate_pool_invariants(pool: &Pool) {
        assert_eq!(
            pool.outcome_descriptions.len(),
            pool.options_count,
            "outcome_descriptions length must equal options_count"
        );
        // Issue #1122 — bound each outcome description's length to prevent
        // unbounded persistent-storage growth and reject empty labels that
        // produce a useless UI.
        for desc in pool.outcome_descriptions.iter() {
            let len = desc.len();
            assert!(
                len >= MIN_OUTCOME_DESCRIPTION_LEN,
                "outcome description must be non-empty"
            );
            assert!(
                len <= MAX_OUTCOME_DESCRIPTION_LEN,
                "outcome description exceeds MAX_OUTCOME_DESCRIPTION_LEN bytes"
            );
        }
    }

    /// Pure: Check if pool state transition is valid
    /// PRE: current_state is valid MarketState
    /// POST: returns true only for valid transitions (INV-2)
    #[allow(dead_code)]
    fn is_valid_state_transition(current: MarketState, next: MarketState) -> bool {
        matches!(
            (current, next),
            (
                MarketState::Active,
                MarketState::Resolved | MarketState::Canceled
            )
        )
    }

    /// Pure: Validate fee basis points
    /// POST: returns true iff fee_bps ≤ 10_000 (INV-6)
    fn is_valid_fee_bps(fee_bps: u32) -> bool {
        fee_bps <= 10_000
    }

    /// Pure: Check if a pool is currently active.
    /// A pool is active iff it has not been resolved, not been canceled,
    /// and its state is explicitly `MarketState::Active`.
    ///
    /// PRE: pool is a valid Pool instance
    /// POST: returns true only when all three conditions hold simultaneously
    fn is_pool_active(pool: &Pool) -> bool {
        pool.state == MarketState::Active
    }

    /// Validate custom token transfer constraints before executing a transfer.
    /// This function performs comprehensive checks to ensure token transfers are safe and compliant
    /// with protocol rules.
    ///
    /// # Checks performed:
    /// 1. Token address validation (not null/default)
    /// 2. Amount validation (positive, not zero)
    /// 3. Sender/recipient validation (not same, not null)
    /// 4. Token contract callable check (via contract invocation)
    ///
    /// # Returns:
    /// - Ok(()) if all validation checks pass
    /// - Err(PredifiError::TokenError) if transfer is deemed unsafe
    /// - Err(PredifiError::InvalidAmount) if amount is invalid
    /// - Err(PredifiError::InvalidAddressOrToken) if addresses are invalid
    fn validate_token_transfer(
        env: &Env,
        token: &Address,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), PredifiError> {
        // Validate amount: must be positive and non-zero
        if amount <= 0 {
            return Err(PredifiError::InvalidAmount);
        }

        // Validate sender and recipient are distinct
        if from == to {
            return Err(PredifiError::InvalidAddressOrToken);
        }

        // Verify token contract is callable by attempting to get its balance
        // This ensures the token contract is valid and responsive
        let token_client = token::Client::new(env, token);
        let _ = token_client.balance(from);
        Ok(())
    }

    /// Validate stake limit modifications to ensure consistency and safety.
    /// This function performs comprehensive checks before applying new stake limits to a pool.
    ///
    /// # Validation Checks:
    /// 1. min_stake must be positive (> 0)
    /// 2. If max_stake is set (> 0), it must not be less than min_stake
    /// 3. New min_stake must not exceed the pool's total_stake (existing liability check)
    /// 4. New min_stake must not exceed the pool's max_total_stake limit (future capacity check)
    /// 5. If max_stake is set, verify it allows room for at least one prediction at max level
    /// 6. Prevent sudden reduction that would violate existing predictions (if applicable)
    ///
    /// # Returns:
    /// - Ok(()) if all validation checks pass
    /// - Err(PredifiError::StakeBelowMinimum) if min_stake <= 0
    /// - Err(PredifiError::StakeAboveMaximum) if constraints are violated
    /// - Err(PredifiError::InvalidAmount) if amounts are invalid
    fn validate_stake_limits(
        _env: &Env,
        pool: &Pool,
        new_min_stake: i128,
        new_max_stake: i128,
    ) -> Result<(), PredifiError> {
        // Check 1: min_stake must be positive
        if new_min_stake <= 0 {
            return Err(PredifiError::StakeBelowMinimum);
        }

        // Check 2: If max_stake is set, ensure min_stake <= max_stake
        if new_max_stake > 0 && new_min_stake > new_max_stake {
            return Err(PredifiError::InvalidAmount);
        }

        // Check 3: Ensure new min_stake doesn't exceed current total_stake if any
        // This prevents setting a minimum that would retroactively invalidate existing bets
        if pool.total_stake > 0 && new_min_stake > pool.total_stake {
            return Err(PredifiError::StakeAboveMaximum);
        }

        // Check 4: If pool has a max_total_stake limit, new min_stake should be reasonable
        if pool.max_total_stake > 0 && new_min_stake > pool.max_total_stake {
            return Err(PredifiError::StakeAboveMaximum);
        }

        // Check 5: If max_stake is set, ensure it's reasonable relative to pool capacity
        if new_max_stake > 0 && pool.max_total_stake > 0 && new_max_stake > pool.max_total_stake {
            return Err(PredifiError::StakeAboveMaximum);
        }

        // Check 6: Prevent extreme ratio between min and max (prevent usability issues)
        if new_max_stake > 0 && new_max_stake != new_min_stake {
            // Ensure max is at least 10x min to allow reasonable participation range
            let min_reasonable_ratio = new_min_stake
                .checked_mul(10)
                .ok_or(PredifiError::ArithmeticError)?;
            if new_max_stake < min_reasonable_ratio {
                return Err(PredifiError::InvalidAmount);
            }
        }

        Ok(())
    }

    /// Pure: Initialize outcome stakes vector with zeros.
    /// Used for markets with many outcomes (e.g., 32+ teams tournament).
    #[allow(dead_code)]
    fn init_outcome_stakes(env: &Env, options_count: u32) -> Vec<i128> {
        gas_opt::alloc_zero_stakes(env, options_count)
    }

    /// Get outcome stakes for a pool using optimized batch storage.
    /// Falls back to individual storage keys for backward compatibility.
    fn get_outcome_stakes(env: &Env, pool_id: u64, options_count: u32) -> Vec<i128> {
        let key = DataKey::OutStakes(pool_id);
        if let Some(stakes) = env.storage().persistent().get(&key) {
            Self::extend_persistent(env, &key);
            stakes
        } else {
            // Fallback: reconstruct from individual outcome stakes (backward compatibility)
            // Migrate into the batch key so subsequent reads are O(1) storage IO.
            let mut stakes = gas_opt::alloc_zero_stakes(env, options_count);
            for i in 0..options_count {
                let outcome_key = DataKey::OutStake(pool_id, i);
                let stake: i128 = env.storage().persistent().get(&outcome_key).unwrap_or(0);
                if stake != 0 {
                    stakes.set(i, stake);
                }
            }
            env.storage().persistent().set(&key, &stakes);
            Self::extend_persistent(env, &key);
            stakes
        }
    }

    /// Update outcome stake at a specific index and persist using batch storage only.
    ///
    /// Gas optimization: a single `OutStakes` write replaces the previous dual-write
    /// (`OutStakes` + per-outcome `OutStake`), cutting ~1 persistent write + TTL bump
    /// per `place_prediction` call.
    ///
    /// # Panics
    /// Panics if `outcome >= options_count` to prevent unbounded storage growth.
    fn update_outcome_stake(
        env: &Env,
        pool_id: u64,
        outcome: u32,
        amount: i128,
        options_count: u32,
    ) -> Vec<i128> {
        // Enforce outcome bounds to prevent unbounded storage growth
        if outcome >= options_count {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidOutcome);
        }

        let mut stakes = Self::get_outcome_stakes(env, pool_id, options_count);
        if gas_opt::apply_stake_delta(&mut stakes, outcome, amount).is_err() {
            soroban_sdk::panic_with_error!(&env, PredifiError::ArithmeticError);
        }

        // Single batch persist — no dual-write of individual OutStake keys
        let key = DataKey::OutStakes(pool_id);
        env.storage().persistent().set(&key, &stakes);
        Self::extend_persistent(env, &key);

        stakes
    }

    // ── Storage & Side-Effect Functions ───────────────────────────────────────

    /// Extend the TTL of the contract's instance storage entry.
    ///
    /// Instance storage holds global contract state (e.g., `Config`, `PoolIdCtr`).
    /// Extending its TTL every time the contract is called ensures the instance entry
    /// does not expire and become inaccessible between pool operations.
    ///
    /// Uses `BUMP_THRESHOLD` as the minimum remaining ledgers before extension and
    /// `BUMP_AMOUNT` as the target TTL after extension.
    fn extend_instance(env: &Env) {
        env.storage()
            .instance()
            .extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);
    }

    /// Extend the TTL of a single persistent storage entry identified by `key`.
    ///
    /// Persistent storage entries (e.g., pool state, category indexes, outcome stakes)
    /// can expire if they are not accessed frequently enough. This function must be
    /// called whenever a key is written or read so that the data survives until the
    /// next expected access. Failure to call this after a write would leave the entry
    /// at its default ledger TTL, which may be shorter than `BUMP_AMOUNT`.
    fn extend_persistent(env: &Env, key: &DataKey) {
        env.storage()
            .persistent()
            .extend_ttl(key, BUMP_THRESHOLD, BUMP_AMOUNT);
    }

    /// Extend the TTL of a temporary storage entry identified by `key`.
    ///
    /// Temporary storage is used for short-lived state such as the reentrancy guard
    /// (`DataKey::RentGuard`) and per-user prediction cooldown timestamps. The TTL
    /// is extended on each access to prevent the entry from expiring mid-transaction
    /// in edge cases where a ledger is unusually slow to close.
    fn extend_temporary(env: &Env, key: &DataKey) {
        env.storage()
            .temporary()
            .extend_ttl(key, BUMP_THRESHOLD, BUMP_AMOUNT);
    }

    /// Bumps both instance and persistent TTLs for the given key in one call.
    fn bump_ttl(env: &Env, key: &DataKey) {
        Self::extend_instance(env);
        Self::extend_persistent(env, key);
    }

    /// Call `has_role` on the access-control contract.
    ///
    /// On success returns the boolean result.
    /// On failure (e.g. the access-control contract panics or is not deployed)
    /// maps the error to [`PredifiError::OracleNotInitialized`] so callers get a
    /// typed error rather than an unstructured panic.
    fn has_role(
        env: &Env,
        contract: &Address,
        user: &Address,
        role: u32,
    ) -> Result<bool, PredifiError> {
        // try_invoke_contract returns Result<Result<T, ConversionError>, InvokeError>;
        // we flatten both layers into a single PredifiError.
        env.try_invoke_contract::<bool, PredifiError>(
            contract,
            &Symbol::new(env, "has_role"),
            soroban_sdk::vec![env, user.into_val(env), role.into_val(env)],
        )
        .map_err(|_| PredifiError::OracleNotInitialized) // outer: invocation failure
        .and_then(|inner| inner.map_err(|_| PredifiError::OracleNotInitialized))
        // inner: XDR error
    }

    /// Call `get_admin` on the access-control contract.
    ///
    /// Maps external call failures to [`PredifiError::InvalidData`] so callers
    /// receive a descriptive, typed error instead of a contract panic.
    fn get_access_control_admin(env: &Env, contract: &Address) -> Result<Address, PredifiError> {
        // try_invoke_contract returns Result<Result<T, ConversionError>, InvokeError>;
        // we flatten both layers into a single PredifiError.
        env.try_invoke_contract::<Address, PredifiError>(
            contract,
            &Symbol::new(env, "get_admin"),
            soroban_sdk::vec![env],
        )
        .map_err(|_| PredifiError::InvalidData) // outer: invocation failure
        .and_then(|inner| inner.map_err(|_| PredifiError::InvalidData)) // inner: XDR error
    }

    fn require_role(env: &Env, user: &Address, role: u32) -> Result<(), PredifiError> {
        let config = Self::get_config(env);
        let has = Self::has_role(env, &config.access_control, user, role)?;
        if !has {
            return Err(PredifiError::Unauthorized);
        }
        Ok(())
    }

    fn require_admin_role(
        env: &Env,
        admin: &Address,
        operation: &'static str,
    ) -> Result<(), PredifiError> {
        if let Err(e) = Self::require_role(env, admin, 0) {
            UnauthorizedAdminAttemptEvent {
                caller: admin.clone(),
                operation: Symbol::new(env, operation),
                timestamp: env.ledger().timestamp(),
            }
            .publish(env);
            return Err(e);
        }
        Ok(())
    }

    /// Verify that `operator` holds Operator role (`1`) and emit an audit event on failure.
    ///
    /// This wrapper adds observability around the raw `require_role` check: if the caller
    /// lacks the Operator role, a [`UnauthorizedResolveAttemptEvent`] is published before
    /// the error is returned so that indexers can alert on unauthorised resolution attempts
    /// without having to parse panic traces.
    ///
    /// Called at the start of `resolve_pool` before any state is read or mutated.
    ///
    /// # Errors
    /// Returns the same error as `require_role` — typically `PredifiError::Unauthorized`.
    fn require_operator_role_for_resolution(
        env: &Env,
        operator: &Address,
        pool_id: u64,
    ) -> Result<(), PredifiError> {
        if let Err(e) = Self::require_role(env, operator, 1) {
            UnauthorizedResolveAttemptEvent {
                caller: operator.clone(),
                pool_id,
                timestamp: env.ledger().timestamp(),
            }
            .publish(env);
            return Err(e);
        }
        Ok(())
    }

    /// Verify that `oracle` holds Oracle role (`3`) and emit an audit event on failure.
    ///
    /// Mirrors `require_operator_role_for_resolution` but uses Oracle role (`3`) instead
    /// of Operator role (`1`).  Used by price-feed-based resolution paths
    /// (`resolve_pool_from_price`) where a whitelisted oracle triggers settlement rather
    /// than a human operator.
    ///
    /// # Errors
    /// Returns the same error as `require_role` — typically `PredifiError::Unauthorized`.
    fn require_oracle_role_for_resolution(
        env: &Env,
        oracle: &Address,
        pool_id: u64,
    ) -> Result<(), PredifiError> {
        if let Err(e) = Self::require_role(env, oracle, 3) {
            UnauthorizedResolveAttemptEvent {
                caller: oracle.clone(),
                pool_id,
                timestamp: env.ledger().timestamp(),
            }
            .publish(env);
            return Err(e);
        }
        Ok(())
    }

    fn get_config(env: &Env) -> Config {
        let config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .expect("Config not set");
        Self::extend_instance(env);
        config
    }

    /// Referral cut in basis points (e.g. 5000 = 50% of referrer's fee share to referrer). Default 5000.
    ///
    /// Prefers `Config.referral_bps` (set via `set_referral_rate`) when non-zero,
    /// then falls back to the legacy `ReferralCutBps` storage key, then to 5000.
    fn read_referral_cut_bps(env: &Env) -> u32 {
        // Prefer the value stored in Config (set via set_referral_rate).
        let config_bps: Option<Config> = env.storage().instance().get(&DataKey::Config);
        if let Some(cfg) = config_bps {
            if cfg.referral_bps > 0 {
                Self::extend_instance(env);
                return cfg.referral_bps;
            }
        }
        // Fall back to legacy standalone key.
        let bps = env
            .storage()
            .instance()
            .get(&DataKey::ReferralCutBps)
            .unwrap_or(5000u32);
        Self::extend_instance(env);
        bps
    }

    fn is_paused(env: &Env) -> bool {
        let paused = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        Self::extend_instance(env);
        paused
    }

    /// Returns `Err(PredifiError::ContractPaused)` if the contract is currently
    /// paused, `Ok(())` otherwise. All state-mutating entry points call this at
    /// their top to block execution while the emergency pause is active.
    fn require_not_paused(env: &Env) -> Result<(), PredifiError> {
        if Self::is_paused(env) {
            return Err(PredifiError::ContractPaused);
        }
        Ok(())
    }

    /// Set the reentrancy guard for the current transaction.
    ///
    /// Writes `DataKey::RentGuard = true` to temporary storage. Because Soroban's
    /// temporary storage lives only for the duration of the current contract invocation
    /// tree, the flag is automatically cleared when the outermost call returns — but
    /// `exit_reentrancy_guard` removes it explicitly so that gas is not wasted on the
    /// automatic expiry path.
    ///
    /// # Panics
    /// Panics with `"Reentrancy detected"` if the guard is already set, which means
    /// the contract has been re-entered before the current top-level call has returned.
    fn enter_reentrancy_guard(env: &Env) {
        let key = DataKey::RentGuard;
        if env.storage().temporary().has(&key) {
            panic!("Reentrancy detected");
        }
        env.storage().temporary().set(&key, &true);
    }

    /// Clear the reentrancy guard after a protected operation completes.
    ///
    /// Must always be called after `enter_reentrancy_guard` — even when the protected
    /// operation returns an error — to avoid leaving the guard set and permanently
    /// blocking future calls within the same ledger snapshot.
    fn exit_reentrancy_guard(env: &Env) {
        env.storage().temporary().remove(&DataKey::RentGuard);
    }

    /// Returns true if the token is on the allowed betting whitelist.
    fn is_token_whitelisted(env: &Env, token: &Address) -> bool {
        let key = DataKey::TokenWl(token.clone());
        let whitelisted = env.storage().persistent().has(&key);
        if whitelisted {
            Self::extend_persistent(env, &key);
        }
        whitelisted
    }

    /// Returns true if the oracle address is explicitly whitelisted for price updates.
    fn is_oracle_whitelisted(env: &Env, oracle: &Address) -> bool {
        let key = DataKey::OracleWl(oracle.clone());
        let whitelisted: bool = env.storage().persistent().get(&key).unwrap_or(false);
        if whitelisted {
            Self::extend_persistent(env, &key);
        }
        whitelisted
    }

    /// Register a newly created pool in the global active pool index.
    ///
    /// The counter (`ActivePoolCtr`) is stored in **persistent** storage rather than
    /// instance storage so that pool-index bookkeeping does not inflate the instance
    /// storage entry that is loaded on every contract call.
    fn add_to_active_index(env: &Env, pool_id: u64) {
        let ctr_key = DataKey::ActivePoolCtr;
        let count: u32 = env.storage().persistent().get(&ctr_key).unwrap_or(0u32);
        // Only extend TTL if the key already exists (extend_ttl panics on missing keys).
        if count > 0 {
            Self::extend_persistent(env, &ctr_key);
        }

        let slot_key = DataKey::ActivePool(count);
        env.storage().persistent().set(&slot_key, &pool_id);
        Self::extend_persistent(env, &slot_key);

        let idx_key = DataKey::ActivePoolIdx(pool_id);
        env.storage().persistent().set(&idx_key, &count);
        Self::extend_persistent(env, &idx_key);

        // Write the incremented counter and extend its TTL.
        env.storage().persistent().set(&ctr_key, &(count + 1));
        Self::extend_persistent(env, &ctr_key);
    }

    /// Remove a pool from the global active pool index using swap-and-pop.
    /// The last entry is moved into the vacated slot so the index stays dense.
    ///
    /// The counter is persisted in persistent storage (see `add_to_active_index`).
    fn remove_from_active_index(env: &Env, pool_id: u64) {
        let ctr_key = DataKey::ActivePoolCtr;
        let count: u32 = env.storage().persistent().get(&ctr_key).unwrap_or(0u32);
        if count == 0 {
            return;
        }
        // Key exists (count > 0), safe to extend TTL.
        Self::extend_persistent(env, &ctr_key);

        let idx_key = DataKey::ActivePoolIdx(pool_id);
        let pos: u32 = match env.storage().persistent().get(&idx_key) {
            Some(p) => p,
            None => return, // not in index — already removed or never added
        };

        let last = count - 1;

        if pos != last {
            // Move the last entry into the vacated slot.
            let last_slot_key = DataKey::ActivePool(last);
            let last_pool_id: u64 = env
                .storage()
                .persistent()
                .get(&last_slot_key)
                .expect("active pool index inconsistency");

            let target_slot_key = DataKey::ActivePool(pos);
            env.storage()
                .persistent()
                .set(&target_slot_key, &last_pool_id);
            Self::extend_persistent(env, &target_slot_key);

            // Update the moved pool's reverse-lookup entry.
            let moved_idx_key = DataKey::ActivePoolIdx(last_pool_id);
            env.storage().persistent().set(&moved_idx_key, &pos);
            Self::extend_persistent(env, &moved_idx_key);

            // Clean up the old last slot.
            env.storage().persistent().remove(&last_slot_key);
        } else {
            // The pool being removed IS the last entry — just delete its slot.
            let slot_key = DataKey::ActivePool(pos);
            env.storage().persistent().remove(&slot_key);
        }

        // Remove the reverse-lookup entry for the removed pool.
        env.storage().persistent().remove(&idx_key);

        // Decrement the counter.
        env.storage().persistent().set(&ctr_key, &last);
        Self::extend_persistent(env, &ctr_key);
    }

    /// Returns true if the pool has a properly resolved outcome (not the sentinel value).
    /// Returns `true` if the pool has been assigned a definitive winning outcome.
    ///
    /// A pool is considered resolved when `pool.outcome` differs from
    /// `UNRESOLVED_OUTCOME` (`u32::MAX`), the sentinel used while the pool is
    /// still accepting predictions or awaiting operator votes.
    fn is_pool_resolved(pool: &Pool) -> bool {
        pool.outcome != UNRESOLVED_OUTCOME
    }

    /// Determine the protocol fee rate (in basis points) to apply at resolution time.
    ///
    /// The fee is chosen by walking the configured fee tiers in ascending order and
    /// selecting the tier whose `stake_threshold` is the highest value that is still
    /// `<= pool.total_stake`.  If no tier threshold is met, the base fee from
    /// `Config.fee_bps` is used as a fallback.
    ///
    /// Tiers allow the protocol to offer reduced fees for high-volume pools as a
    /// liquidity incentive — the more total stake a pool attracts, the lower the cut
    /// taken from winners.
    ///
    /// Called by `resolve_pool` and `resolve_pool_from_price` immediately before the
    /// pool's state is written as `Resolved`, so the fee is fixed at the moment of
    /// resolution rather than at prediction time.
    ///
    /// # Returns
    /// A fee rate in basis points (0–10 000).  The caller is responsible for ensuring
    /// the returned value satisfies `is_valid_fee_bps`.
    fn calculate_dynamic_fee(env: &Env, pool: &Pool) -> u32 {
        let config = Self::get_config(env);
        let tiers = Self::get_fee_tiers(env.clone());
        let mut applied_fee = config.fee_bps;

        let mut max_threshold = -1i128;
        for i in 0..tiers.len() {
            if let Some(tier) = tiers.get(i) {
                if pool.total_stake >= tier.stake_threshold && tier.stake_threshold > max_threshold
                {
                    max_threshold = tier.stake_threshold;
                    applied_fee = tier.fee_bps;
                }
            }
        }
        applied_fee
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1125 — Storage TTL Renewal Helper
// Issue #1128 — Referral Volume Threshold Logic
// Issue #1137 — Contract Metadata Getter
// Issue #1142 — Event Emission Consistency (new events wired in below)
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod issues_1446_1447_1448_1449_boundary_tests;
#[cfg(test)]
mod referral_integration_tests;
#[cfg(test)]
mod test;

