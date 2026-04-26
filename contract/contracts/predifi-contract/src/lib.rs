#![no_std]
#![allow(clippy::too_many_arguments)]

mod benchmark_test;
mod constants;
#[cfg(test)]
mod payout_proptests;
mod price_feed;
mod price_feed_simple;
mod safe_math;
#[cfg(test)]
mod safe_math_examples;
#[cfg(test)]
mod storage_test;
#[cfg(test)]
mod stress_test;
#[cfg(test)]
mod test_utils;
// #[cfg(test)]
// mod storage_test;

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, log, symbol_short, token,
    Address, BytesN, Env, IntoVal, String, Symbol, Vec,
};

pub use constants::*;
pub use price_feed_simple::PriceFeedAdapter;
pub use safe_math::{RoundingMode, SafeMath};

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

/// Miscellaneous predictions that don't fit other categories
pub const CATEGORY_OTHER: Symbol = symbol_short!("Other");

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
}

/// Represents the current state of a prediction market.
///
/// State transitions are one-way: `Active` can only transition to `Resolved` or `Canceled`.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketState {
    /// Market is active and accepting predictions.
    Active = 0,
    /// Market has been resolved and winnings can be claimed.
    Resolved = 1,
    /// Market has been canceled and stakes can be refunded.
    Canceled = 2,
}

/// Parameters for creating a new prediction pool.
///
/// This struct is used internally to validate and organize pool creation data.
/// All fields must pass validation before a pool can be created.
#[contracttype]
#[derive(Clone)]
pub struct CreatePoolParams {
    /// Unix timestamp after which no more predictions are accepted.
    pub end_time: u64,
    /// The Stellar token contract address used for staking.
    pub token: Address,
    /// Number of possible outcomes (must be >= 2 and <= MAX_OPTIONS_COUNT).
    pub options_count: u32,
    /// Short human-readable description of the event (max 256 bytes).
    pub description: String,
    /// URL pointing to extended metadata, e.g. an IPFS link (max 512 bytes).
    pub metadata_url: String,
    /// Minimum stake amount per prediction (must be > 0).
    pub min_stake: i128,
    /// Maximum stake amount per prediction (0 = no limit).
    pub max_stake: i128,
    /// Optional initial liquidity to provide from creator (house money).
    pub initial_liquidity: i128,
    /// Market category for classification (e.g., Sports, Finance, Crypto).
    pub category: Symbol,
    /// Whether the pool is private (invite-only).
    pub private: bool,
    /// Optional symbol used as an invite key for private pools.
    pub whitelist_key: Option<Symbol>,
    /// Human-readable labels for each outcome (length must equal options_count).
    pub outcome_descriptions: Vec<String>,
}

/// Represents a prediction pool with all its configuration and state.
///
/// A pool is the core data structure that represents a prediction market.
/// It contains all information about the market, including its lifecycle,
/// financial configuration, participant constraints, and resolution status.
///
/// # Invariants
/// - `end_time` must be in the future when the pool is created (INV-8).
/// - `state` can only transition from `Active` to either `Resolved` or `Canceled` (INV-2).
/// - `total_stake` must always equal the sum of all individual outcome stakes (INV-1).
/// - For resolved pools: total winnings ≤ `total_stake` (INV-5)
#[contracttype]
#[derive(Clone)]
pub struct Pool {
    /// Unix timestamp after which no more predictions (stakes) are accepted.
    /// This defines the end of the "betting window".
    pub end_time: u64,
    /// Current operational state of the market.
    /// Possible values: `Active` (betting open), `Resolved` (result final), `Canceled` (refunds available).
    pub state: MarketState,
    /// The winning outcome index (0-based) after resolution.
    /// Only meaningful if `state` is `Resolved`. Default is 0.
    pub outcome: u32,
    /// The contract address of the Stellar token (e.g., USDC) used for all stakes and payouts.
    pub token: Address,
    /// Total amount of tokens currently staked in the pool.
    /// Includes user stakes, initial house liquidity, and any subsequent liquidity injections.
    pub total_stake: i128,
    /// Market category for organizational purposes (e.g., Sports, Finance, Crypto).
    pub category: Symbol,
    /// A short, human-readable title or question for the prediction market (max 256 bytes).
    pub description: String,
    /// A URL (e.g., IPFS URI) pointing to extended metadata, rules, or rich media for the pool.
    pub metadata_url: String,
    /// Number of distinct outcomes participants can bet on (must be >= 2).
    pub options_count: u32,
    /// Minimum amount a user must stake in a single prediction (must be > 0).
    pub min_stake: i128,
    /// Maximum amount a user can stake in a single prediction (0 indicates no limit).
    pub max_stake: i128,
    /// Minimum `total_stake` required for the pool to be considered valid for resolution.
    /// If this is not met by `end_time`, the pool may be eligible for cancellation.
    pub min_total_stake: i128,
    /// Hard cap on the `total_stake` the pool can accept (0 indicates no limit).
    pub max_total_stake: i128,
    /// Seed liquidity provided by the pool creator at initialization ("house money").
    /// This amount is part of `total_stake` but is typically excluded from protocol fee calculations.
    pub initial_liquidity: i128,
    /// Address of the account that created the pool and provided initial liquidity.
    pub creator: Address,
    /// Number of independent oracle/operator resolutions required before the pool is finalized.
    /// This provides a decentralized consensus mechanism for result verification.
    pub required_resolutions: u32,
    /// If true, only whitelisted addresses can participate in this pool.
    pub private: bool,
    /// A unique symbol or secret used as an invite key for accessing private pools.
    pub whitelist_key: Option<Symbol>,
    /// Human-readable labels for each possible outcome (e.g., ["Yes", "No"]).
    /// The length of this vector must exactly match `options_count`.
    pub outcome_descriptions: Vec<String>,
    /// The specific protocol fee in basis points (1 bp = 0.01%) applied to this pool at resolution.
    /// This value is typically determined by the dynamic fee tier system.
    pub fee_bps: u32,
    /// Number of unique addresses that have placed at least one prediction in this pool.
    pub participants_count: u32,
}

/// Configuration parameters for creating a prediction pool.
///
/// This struct is passed to `create_pool` to define the pool's immutable (or near-immutable)
/// blueprint. It separates creation-time parameters from the runtime state managed in `Pool`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolConfig {
    /// A short, human-readable title or question for the prediction market (max 256 bytes).
    pub description: String,
    /// A URL (e.g., IPFS URI) pointing to extended metadata, rules, or rich media (max 512 bytes).
    pub metadata_url: String,
    /// Minimum amount a user must stake in a single prediction (must be > 0).
    pub min_stake: i128,
    /// Maximum amount a user can stake in a single prediction (0 indicates no limit).
    /// If non-zero, it must be greater than or equal to `min_stake`.
    pub max_stake: i128,
    /// Minimum `total_stake` required for the pool to be considered valid for resolution.
    /// This ensures the pool has meaningful participation before a result is finalized.
    pub min_total_stake: i128,
    /// Hard cap on the `total_stake` the pool can accept (0 indicates no limit).
    pub max_total_stake: i128,
    /// Seed liquidity provided by the pool creator at initialization ("house money").
    /// This amount participates in the pool but is typically excluded from fee calculations.
    pub initial_liquidity: i128,
    /// Number of independent oracle/operator resolutions required before the pool is finalized.
    /// Multi-resolution provides a safety layer against single-oracle failure or manipulation.
    pub required_resolutions: u32,
    /// If true, only whitelisted addresses can participate in this pool.
    pub private: bool,
    /// A unique symbol or secret used as an invite key for accessing private pools.
    pub whitelist_key: Option<Symbol>,
    /// Human-readable labels for each outcome (length must equal options_count).
    pub outcome_descriptions: Vec<String>,
}

/// Statistics for a prediction pool.
///
/// Provides a snapshot of pool activity including stakes, participants, and odds.
/// Useful for frontends and analytics.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PoolStats {
    /// Unique identifier of the pool.
    pub pool_id: u64,
    /// Total amount of tokens staked across all outcomes.
    pub total_stake: i128,
    /// Vector of stake amounts for each outcome (indexed by outcome number).
    pub stakes_per_outcome: Vec<i128>,
    /// Number of unique participants in this pool.
    pub participants_count: u32,
    /// Current odds for each outcome in fixed-point format with 4 decimals.
    /// For example, 10000 represents 1.00x, 5000 represents 0.50x, 20000 represents 2.00x.
    pub current_odds: Vec<u64>,
}

/// Global protocol configuration.
///
/// Contains system-wide settings that control protocol behavior.
/// These settings can be updated by admin with appropriate governance.
///
/// # Invariants
/// - `fee_bps` must be <= 10,000 (100%) (INV-6)
/// - `max_predictions_per_user` must be >= 0 (0 = no limit)
#[contracttype]
#[derive(Clone)]
pub struct Config {
    /// Protocol fee in basis points (1 bp = 0.01%). Valid range: 0-10,000.
    /// A value of 5000 represents 50% fee on winnings.
    pub fee_bps: u32,
    /// Address that receives protocol fees.
    pub treasury: Address,
    /// Address of the access control contract for role-based permissions.
    pub access_control: Address,
    /// Minimum delay in seconds after pool end time before resolution is allowed.
    /// This provides a grace period for oracle data to settle.
    pub resolution_delay: u64,
    /// Minimum pool duration in seconds.
    pub min_pool_duration: u64,
    /// Global minimum stake amount. Predictions below this are rejected.
    pub min_stake: i128,
    /// Maximum number of predictions a user can place per pool.
    /// A value of 0 means no limit.
    pub max_predictions_per_user: u32,
}

/// Fee percentages returned by [`PredifiContract::get_fees`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeInfo {
    /// Protocol (treasury) fee in basis points (1 bp = 0.01%). Range: 0-10,000.
    pub treasury_fee_bps: u32,
    /// Referral cut in basis points — the share of the protocol fee paid to referrers.
    /// Range: 0-10,000. Default: 5,000 (50%).
    pub referral_fee_bps: u32,
}

/// Aggregated contract metadata for frontend consumption.
///
/// This read model allows clients to fetch protocol configuration and core stats
/// in one call instead of performing multiple separate getters.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInfo {
    /// Contract version tracked in instance storage.
    pub version: u32,
    /// Admin address from the access-control contract.
    pub current_admin: Address,
    /// Whether the contract is currently paused.
    pub is_paused: bool,
    /// Total number of pools created so far.
    pub total_pools: u64,
    /// Protocol fee in basis points (1 bp = 0.01%).
    pub fee_bps: u32,
    /// Referral fee cut in basis points.
    pub referral_cut_bps: u32,
    /// Treasury address that receives protocol fees.
    pub treasury: Address,
    /// Access-control contract address.
    pub access_control: Address,
    /// Global resolution delay in seconds.
    pub resolution_delay: u64,
    /// Minimum pool duration in seconds.
    pub min_pool_duration: u64,
    /// Global minimum stake.
    pub min_stake: i128,
    /// Maximum predictions allowed per user per pool.
    pub max_predictions_per_user: u32,
}

/// Represents a fee tier within the protocol's dynamic fee system.
///
/// Fee tiers allow the protocol to adjust fees based on the pool's total volume (stake).
/// Tiers are applied based on the total stake (volume) of the pool at resolution time.
/// Higher volumes typically result in lower fee percentages to encourage participation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeTier {
    /// The `total_stake` threshold at or above which this tier's `fee_bps` becomes applicable.
    pub stake_threshold: i128,
    /// The protocol fee in basis points (1 bp = 0.01%) for this tier.
    /// Must be between 0 and 10,000 (inclusive).
    pub fee_bps: u32,
}

/// Detailed information about a user's prediction in a specific pool.
///
/// This struct is a convenient "read-only" view that combines user-specific prediction
/// data with current pool state. It is primarily used for frontend displays and
/// calculating potential or final winnings.
#[contracttype]
#[derive(Clone)]
pub struct UserPredictionDetail {
    /// Unique identifier (ID) of the prediction pool.
    pub pool_id: u64,
    /// Total amount of tokens the user has staked on their chosen outcome.
    pub amount: i128,
    /// The outcome index (0-based) that the user predicted would win.
    pub user_outcome: u32,
    /// Unix timestamp when the pool's betting window ends.
    pub pool_end_time: u64,
    /// Current operational state of the pool (Active, Resolved, or Canceled).
    pub pool_state: MarketState,
    /// The winning outcome index (0-based) if the pool is `Resolved`.
    /// Only meaningful when `pool_state` is `MarketState::Resolved`.
    pub pool_outcome: u32,
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
    /// Participant count for a pool: `PartCnt(pool_id)` -> `u32`
    PartCnt(u64),

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

    // ── Resolution voting ────────────────────────────────────────────────────
    /// Tracks if an oracle/operator has already voted: `ResVote(pool_id, voter_address)` -> `()`
    ResVote(u64, Address),
    /// Vote count for a specific outcome: `ResVoteCt(pool_id, outcome)` -> `u32`
    ResVoteCt(u64, u32),
    /// Total number of votes cast for a pool: `ResTotal(pool_id)` -> `u32`
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
}

/// Represents a user's individual stake in a prediction market.
///
/// This is the core structure for tracking participation. It is stored as part of the
/// ledger state for each user-pool pair, mapping a specific outcome to a staked amount.
#[contracttype]
#[derive(Clone)]
pub struct Prediction {
    /// Total amount of tokens staked by the user on this outcome.
    pub amount: i128,
    /// The chosen outcome index (0-based). This corresponds to the index in `Pool.outcome_descriptions`.
    pub outcome: u32,
}

// ── Events ───────────────────────────────────────────────────────────────────

#[contractevent(topics = ["init"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitEvent {
    pub access_control: Address,
    pub treasury: Address,
    pub fee_bps: u32,
    pub resolution_delay: u64,
    pub min_pool_duration: u64,
    pub max_predictions_per_user: u32,
}

#[contractevent(topics = ["pause"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseEvent {
    pub admin: Address,
}

#[contractevent(topics = ["unpause"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnpauseEvent {
    pub admin: Address,
}

#[contractevent(topics = ["fee_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeUpdateEvent {
    pub admin: Address,
    pub fee_bps: u32,
}

#[contractevent(topics = ["max_predictions_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxPredictionsUpdateEvent {
    pub admin: Address,
    pub limit: u32,
}

#[contractevent(topics = ["fee_tiers_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeTiersUpdateEvent {
    pub admin: Address,
    pub tiers_count: u32,
}

#[contractevent(topics = ["treasury_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryUpdateEvent {
    pub admin: Address,
    pub treasury: Address,
}

#[contractevent(topics = ["resolution_delay_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionDelayUpdateEvent {
    pub admin: Address,
    pub delay: u64,
}
#[contractevent(topics = ["min_pool_duration_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinPoolDurationUpdateEvent {
    pub admin: Address,
    pub duration: u64,
}

#[contractevent(topics = ["min_stake_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinStakeUpdateEvent {
    pub admin: Address,
    pub min_stake: i128,
}

#[contractevent(topics = ["pool_ready"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolReadyForResolutionEvent {
    pub pool_id: u64,
    pub timestamp: u64,
}

#[contractevent(topics = ["pool_created"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolCreatedEvent {
    pub pool_id: u64,
    pub creator: Address,
    pub end_time: u64,
    pub token: Address,
    pub options_count: u32,
    pub metadata_url: String,
    pub initial_liquidity: i128,
    pub category: Symbol,
    pub required_resolutions: u32,
    pub max_total_stake: i128,
    pub outcome_descriptions: Vec<String>,
}

#[contractevent(topics = ["initial_liquidity_provided"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialLiquidityProvidedEvent {
    pub pool_id: u64,
    pub creator: Address,
    pub amount: i128,
}

#[contractevent(topics = ["pool_resolved"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolResolvedEvent {
    pub pool_id: u64,
    pub operator: Address,
    pub outcome: u32,
}

#[contractevent(topics = ["oracle_resolved"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleResolvedEvent {
    pub pool_id: u64,
    pub oracle: Address,
    pub outcome: u32,
    pub proof: String,
}

#[contractevent(topics = ["pool_canceled"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolCanceledEvent {
    pub pool_id: u64,
    pub caller: Address,
    pub reason: String,
    pub operator: Address,
}

#[contractevent(topics = ["stake_limits_updated"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeLimitsUpdatedEvent {
    pub pool_id: u64,
    pub operator: Address,
    pub min_stake: i128,
    pub max_stake: i128,
}

#[contractevent(topics = ["pool_description_updated"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolDescriptionUpdatedEvent {
    pub pool_id: u64,
    pub caller: Address,
    pub new_description: String,
}

#[contractevent(topics = ["prediction_placed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionPlacedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
    pub outcome: u32,
}

#[contractevent(topics = ["winnings_claimed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WinningsClaimedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
}

#[contractevent(topics = ["referral_paid"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferralPaidEvent {
    pub pool_id: u64,
    pub referrer: Address,
    pub referred_user: Address,
    pub amount: i128,
}

// ── Monitoring & Alert Events ─────────────────────────────────────────────────
// These events are classified by severity and are intended for consumption by
// off-chain monitoring tools (Horizon event streaming, Grafana, SIEM, etc.).
// See MONITORING.md at the repo root for scraping patterns and alert rules.

/// 🔴 HIGH ALERT — emitted when `resolve_pool` is called by an address that
/// does not hold the Operator role.  Indicates a potential attack or
/// misconfigured access-control contract.
#[contractevent(topics = ["unauthorized_resolution"])]
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

/// 🟢 INFO — emitted when all outcome stakes are updated in a single operation.
/// Useful for markets with many outcomes (e.g., 32+ teams tournament) where
/// emitting individual events per outcome would be impractical.
#[contractevent(topics = ["outcome_stakes_updated"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeStakesUpdatedEvent {
    pub pool_id: u64,
    /// Number of outcomes in this pool.
    pub options_count: u32,
    /// Total stake across all outcomes after the update.
    pub total_stake: i128,
}

#[contractevent(topics = ["token_whitelist_added"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenWhitelistAddedEvent {
    pub admin: Address,
    pub token: Address,
}

#[contractevent(topics = ["token_whitelist_removed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenWhitelistRemovedEvent {
    pub admin: Address,
    pub token: Address,
}

#[contractevent(topics = ["added_to_whitelist"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddedToWhitelistEvent {
    pub pool_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

#[contractevent(topics = ["removed_from_whitelist"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemovedFromWhitelistEvent {
    pub pool_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

#[contractevent(topics = ["treasury_withdrawn"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryWithdrawnEvent {
    pub admin: Address,
    pub token: Address,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}
#[contractevent(topics = ["refund_claimed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundClaimedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
}

#[contractevent(topics = ["upgrade"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeEvent {
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
}

#[contractevent(topics = ["oracle_init"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleInitEvent {
    pub admin: Address,
    pub pyth_contract: Address,
    pub max_price_age: u64,
    pub min_confidence_ratio: u32,
}

#[contractevent(topics = ["price_feed_updated"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceFeedUpdatedEvent {
    pub oracle: Address,
    pub feed_pair: Symbol,
    pub price: i128,
    pub confidence: i128,
    pub timestamp: u64,
    pub expires_at: u64,
}

#[contractevent(topics = ["price_condition_set"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceConditionSetEvent {
    pub pool_id: u64,
    pub feed_pair: Symbol,
    pub target_price: i128,
    pub operator: u32,
    pub tolerance_bps: u32,
}

#[contractevent(topics = ["price_resolved"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceResolvedEvent {
    pub pool_id: u64,
    pub feed_pair: Symbol,
    pub current_price: i128,
    pub target_price: i128,
    pub outcome: u32,
}

#[contractevent(topics = ["resolution_conflict"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionConflictEvent {
    pub pool_id: u64,
    pub oracle: Address,
    pub outcome: u32,
    pub existing_outcome: u32,
}
mod events;
// pub use events::*; // Unused import

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

    /// Validate that a category symbol is in the allowed list, falling back to CATEGORY_OTHER if not.
    /// Canonical categories are defined as constants at the top of the file.
    /// Any non-matching category is normalized to CATEGORY_OTHER to ensure compatibility
    /// with off-chain analytics while allowing specialized categories in metadata if needed.
    fn validate_category(env: &Env, category: &Symbol) -> Symbol {
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
                    return category.clone();
                }
            }
        }
        CATEGORY_OTHER
    }

    /// Pure: Check if pool state transition is valid
    /// PRE: current_state is valid MarketState
    /// POST: returns true only for valid transitions (INV-2)
    #[allow(dead_code)]
    fn is_valid_state_transition(current: MarketState, next: MarketState) -> bool {
        matches!(
            (current, next),
            (MarketState::Active, MarketState::Resolved)
                | (MarketState::Active, MarketState::Canceled)
        )
    }

    /// Pure: Validate fee basis points
    /// POST: returns true iff fee_bps ≤ 10_000 (INV-6)
    #[allow(dead_code)]
    fn is_valid_fee_bps(fee_bps: u32) -> bool {
        fee_bps <= 10_000
    }

    /// Pure: Check if a pool is currently active.
    /// A pool is active iff it has not been resolved, not been canceled,
    /// and its state is explicitly `MarketState::Active`.
    ///
    /// PRE: pool is a valid Pool instance
    /// POST: returns true only when all three conditions hold simultaneously
    #[allow(dead_code)]
    fn is_pool_active(pool: &Pool) -> bool {
        pool.state == MarketState::Active
    }

    /// Pure: Initialize outcome stakes vector with zeros
    /// Used for markets with many outcomes (e.g., 32+ teams tournament)
    #[allow(dead_code)]
    fn init_outcome_stakes(env: &Env, options_count: u32) -> Vec<i128> {
        let mut stakes = Vec::new(env);
        for _ in 0..options_count {
            stakes.push_back(0);
        }
        stakes
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
            let mut stakes = Vec::new(env);
            for i in 0..options_count {
                let outcome_key = DataKey::OutStake(pool_id, i);
                let stake: i128 = env.storage().persistent().get(&outcome_key).unwrap_or(0);
                stakes.push_back(stake);
            }
            stakes
        }
    }

    /// Update outcome stake at a specific index and persist using optimized batch storage.
    /// Also maintains backward compatibility with individual outcome stake keys.
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
        let current = stakes.get(outcome).unwrap_or(0);
        stakes.set(outcome, current + amount);

        // Store using optimized batch key
        let key = DataKey::OutStakes(pool_id);
        env.storage().persistent().set(&key, &stakes);
        Self::extend_persistent(env, &key);

        // Also update individual key for backward compatibility
        let outcome_key = DataKey::OutStake(pool_id, outcome);
        env.storage()
            .persistent()
            .set(&outcome_key, &(current + amount));
        Self::extend_persistent(env, &outcome_key);

        stakes
    }

    // ── Storage & Side-Effect Functions ───────────────────────────────────────

    fn extend_instance(env: &Env) {
        env.storage()
            .instance()
            .extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);
    }

    fn extend_persistent(env: &Env, key: &DataKey) {
        env.storage()
            .persistent()
            .extend_ttl(key, BUMP_THRESHOLD, BUMP_AMOUNT);
    }

    /// Bumps both instance and persistent TTLs for the given key in one call.
    fn bump_ttl(env: &Env, key: &DataKey) {
        Self::extend_instance(env);
        Self::extend_persistent(env, key);
    }

    fn has_role(env: &Env, contract: &Address, user: &Address, role: u32) -> bool {
        env.invoke_contract(
            contract,
            &Symbol::new(env, "has_role"),
            soroban_sdk::vec![env, user.into_val(env), role.into_val(env)],
        )
    }

    fn get_access_control_admin(env: &Env, contract: &Address) -> Address {
        env.invoke_contract(
            contract,
            &Symbol::new(env, "get_admin"),
            soroban_sdk::vec![env],
        )
    }

    fn require_role(env: &Env, user: &Address, role: u32) -> Result<(), PredifiError> {
        let config = Self::get_config(env);
        if !Self::has_role(env, &config.access_control, user, role) {
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
    fn read_referral_cut_bps(env: &Env) -> u32 {
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

    fn require_not_paused(env: &Env) {
        if Self::is_paused(env) {
            panic!("Contract is paused");
        }
    }

    fn enter_reentrancy_guard(env: &Env) {
        let key = DataKey::RentGuard;
        if env.storage().temporary().has(&key) {
            panic!("Reentrancy detected");
        }
        env.storage().temporary().set(&key, &true);
    }

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

    /// Register a newly created pool in the global active pool index.
    fn add_to_active_index(env: &Env, pool_id: u64) {
        let ctr_key = DataKey::ActivePoolCtr;
        let count: u32 = env.storage().instance().get(&ctr_key).unwrap_or(0);

        let slot_key = DataKey::ActivePool(count);
        env.storage().persistent().set(&slot_key, &pool_id);
        Self::extend_persistent(env, &slot_key);

        let idx_key = DataKey::ActivePoolIdx(pool_id);
        env.storage().persistent().set(&idx_key, &count);
        Self::extend_persistent(env, &idx_key);

        env.storage().instance().set(&ctr_key, &(count + 1));
        Self::extend_instance(env);
    }

    /// Remove a pool from the global active pool index using swap-and-pop.
    /// The last entry is moved into the vacated slot so the index stays dense.
    fn remove_from_active_index(env: &Env, pool_id: u64) {
        let ctr_key = DataKey::ActivePoolCtr;
        let count: u32 = env.storage().instance().get(&ctr_key).unwrap_or(0);
        if count == 0 {
            return;
        }

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
        env.storage().instance().set(&ctr_key, &last);
        Self::extend_instance(env);
    }

    // ── Public interface ──────────────────────────────────────────────────────

    /// Initialize the contract. Idempotent — safe to call multiple times.
    pub fn init(
        env: Env,
        access_control: Address,
        treasury: Address,
        fee_bps: u32,
        resolution_delay: u64,
        min_pool_duration: u64,
        max_predictions_per_user: u32,
    ) {
        if !env.storage().instance().has(&DataKey::Config) {
            let config = Config {
                fee_bps,
                treasury: treasury.clone(),
                access_control: access_control.clone(),
                resolution_delay,
                min_pool_duration,
                min_stake: DEFAULT_GLOBAL_MIN_STAKE,
                max_predictions_per_user,
            };
            env.storage().instance().set(&DataKey::Config, &config);
            env.storage().instance().set(&DataKey::PoolIdCtr, &0u64);
            env.storage()
                .instance()
                .set(&DataKey::Version, &CONTRACT_VERSION);
            Self::extend_instance(&env);

            InitEvent {
                access_control,
                treasury,
                fee_bps,
                resolution_delay,
                min_pool_duration,
                max_predictions_per_user,
            }
            .publish(&env);
        }
    }

    /// Pause the contract. Only callable by Admin (role 0).
    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        if Self::require_admin_role(&env, &admin, "pause").is_err() {
            panic!("Unauthorized: missing required role");
        }
        env.storage().instance().set(&DataKey::Paused, &true);
        Self::extend_instance(&env);

        // Emit dedicated pause-alert event so monitors can apply zero-tolerance
        // rules independently of the generic PauseEvent.
        ContractPausedAlertEvent {
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        PauseEvent { admin }.publish(&env);
    }

    /// Unpause the contract. Only callable by Admin (role 0).
    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        if Self::require_admin_role(&env, &admin, "unpause").is_err() {
            panic!("Unauthorized: missing required role");
        }
        env.storage().instance().set(&DataKey::Paused, &false);
        Self::extend_instance(&env);

        UnpauseEvent { admin }.publish(&env);
    }

    /// Check if the contract is paused.
    ///
    /// This is a public query function that allows third-party integrations
    /// to check the pause state without sending a transaction.
    ///
    /// # Returns
    /// `true` if the contract is paused, `false` otherwise.
    pub fn is_contract_paused(env: Env) -> bool {
        Self::is_paused(&env)
    }

    /// Return the contract version stored during initialization.
    /// Returns 0 if the contract was deployed before version tracking was added.
    pub fn get_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or(0u32)
    }

    /// Return the contract version as a semantic version string.
    ///
    /// This getter provides the human-readable version number in the format "X_Y_Z"
    /// (e.g., "0_0_0"). The version string matches the version specified in Cargo.toml
    /// but uses underscores instead of dots since Symbols don't allow dots.
    ///
    /// # Returns
    /// A `Symbol` containing the current contract version string.
    ///
    /// # Example
    /// ```ignore
    /// let version = contract.get_version_string(&env);
    /// assert_eq!(version, Symbol::new(&env, "0_0_0"));
    /// ```
    pub fn get_version_string(env: Env) -> Symbol {
        Symbol::new(&env, "0_0_0")
    }

    /// Set fee in basis points. Caller must have Admin role (0).
    /// PRE: admin has role 0
    /// POST: Config.fee_bps ≤ 10_000 (INV-6)
    pub fn set_fee_bps(env: Env, admin: Address, fee_bps: u32) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_fee_bps")?;
        assert!(Self::is_valid_fee_bps(fee_bps), "fee_bps exceeds 10000");
        let mut config = Self::get_config(&env);
        config.fee_bps = fee_bps;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        FeeUpdateEvent { admin, fee_bps }.publish(&env);
        Ok(())
    }

    /// Set treasury address. Caller must have Admin role (0).
    pub fn set_treasury(env: Env, admin: Address, treasury: Address) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_treasury")?;
        let mut config = Self::get_config(&env);
        config.treasury = treasury.clone();
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        TreasuryUpdateEvent { admin, treasury }.publish(&env);
        Ok(())
    }

    /// Set maximum predictions per user. Caller must have Admin role (0).
    /// PRE: admin has role 0
    /// POST: Config.max_predictions_per_user >= 0 (0 = no limit)
    pub fn set_max_predictions_per_user(
        env: Env,
        admin: Address,
        limit: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_max_predictions_per_user")?;
        let mut config = Self::get_config(&env);
        config.max_predictions_per_user = limit;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        MaxPredictionsUpdateEvent { admin, limit }.publish(&env);
        Ok(())
    }

    /// Set resolution delay in seconds. Caller must have Admin role (0).
    pub fn set_resolution_delay(env: Env, admin: Address, delay: u64) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_resolution_delay")?;
        let mut config = Self::get_config(&env);
        config.resolution_delay = delay;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        ResolutionDelayUpdateEvent { admin, delay }.publish(&env);
        Ok(())
    }

    /// Set minimum pool duration in seconds. Caller must have Admin role (0).
    pub fn set_min_pool_duration(
        env: Env,
        admin: Address,
        duration: u64,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_min_pool_duration")?;

        let mut config = Self::get_config(&env);
        config.min_pool_duration = duration;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        MinPoolDurationUpdateEvent { admin, duration }.publish(&env);
        Ok(())
    }

    /// Set the global minimum stake amount. Caller must have Admin role (0).
    ///
    /// Predictions with an amount below this threshold will be rejected with
    /// `PredifiError::InsufficientStake`. This prevents spam from micro-predictions.
    ///
    /// # Arguments
    /// * `admin`  - Address with Admin role (0).
    /// * `amount` - New minimum stake in base token units. Must be > 0.
    pub fn set_min_stake(env: Env, admin: Address, amount: i128) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_min_stake")?;
        assert!(amount > 0, "min_stake must be greater than zero");

        let mut config = Self::get_config(&env);
        config.min_stake = amount;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        MinStakeUpdateEvent {
            admin,
            min_stake: amount,
        }
        .publish(&env);
        Ok(())
    }

    /// Set referral cut in basis points (e.g. 5000 = 50% of referrer's fee share). Caller must have Admin role (0).
    /// Must be ≤ 10_000.
    pub fn set_referral_cut_bps(
        env: Env,
        admin: Address,
        referral_cut_bps: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_referral_cut_bps")?;
        assert!(
            referral_cut_bps <= 10_000,
            "referral_cut_bps must be at most 10000"
        );
        env.storage()
            .instance()
            .set(&DataKey::ReferralCutBps, &referral_cut_bps);
        Self::extend_instance(&env);
        Ok(())
    }

    /// Add a token to the allowed betting whitelist. Caller must have Admin role (0).
    pub fn add_token_to_whitelist(
        env: Env,
        admin: Address,
        token: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "add_token_to_whitelist")?;
        let key = DataKey::TokenWl(token.clone());
        env.storage().persistent().set(&key, &true);
        Self::extend_persistent(&env, &key);

        // Add to the whitelist list if not already present
        let whitelist_key = DataKey::TokenWhitelist;
        let mut whitelist: Vec<Address> = env
            .storage()
            .persistent()
            .get(&whitelist_key)
            .unwrap_or_else(|| Vec::new(&env));

        // Only add if not already in the list
        if !whitelist.contains(&token) {
            whitelist.push_back(token.clone());
            env.storage().persistent().set(&whitelist_key, &whitelist);
            Self::extend_persistent(&env, &whitelist_key);
        }

        TokenWhitelistAddedEvent {
            admin: admin.clone(),
            token: token.clone(),
        }
        .publish(&env);
        Ok(())
    }

    /// Remove a token from the allowed betting whitelist. Caller must have Admin role (0).
    pub fn remove_token_from_whitelist(
        env: Env,
        admin: Address,
        token: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "remove_token_from_whitelist")?;
        let key = DataKey::TokenWl(token.clone());
        env.storage().persistent().remove(&key);

        // Remove from the whitelist list
        let whitelist_key = DataKey::TokenWhitelist;
        let mut whitelist: Vec<Address> = env
            .storage()
            .persistent()
            .get(&whitelist_key)
            .unwrap_or_else(|| Vec::new(&env));

        // Remove the token from the list if present
        let new_whitelist = Vec::new(&env);
        let mut new_whitelist = new_whitelist;
        for t in whitelist.iter() {
            if t.clone() != token {
                new_whitelist.push_back(t);
            }
        }
        whitelist = new_whitelist;

        env.storage().persistent().set(&whitelist_key, &whitelist);
        Self::extend_persistent(&env, &whitelist_key);

        TokenWhitelistRemovedEvent {
            admin: admin.clone(),
            token: token.clone(),
        }
        .publish(&env);
        Ok(())
    }

    /// Get the list of all supported (whitelisted) tokens.
    /// Returns a Vec of token addresses that are allowed for betting.
    pub fn get_supported_tokens(env: Env) -> Vec<Address> {
        let whitelist_key = DataKey::TokenWhitelist;
        let whitelist: Vec<Address> = env
            .storage()
            .persistent()
            .get(&whitelist_key)
            .unwrap_or_else(|| Vec::new(&env));

        if env.storage().persistent().has(&whitelist_key) {
            Self::extend_persistent(&env, &whitelist_key);
        }

        whitelist
    }

    /// Upgrade the contract Wasm code. Only callable by Admin (role 0).
    pub fn upgrade_contract(
        env: Env,
        admin: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<(), PredifiError> {
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "upgrade_contract")?;

        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        UpgradeEvent {
            admin: admin.clone(),
            new_wasm_hash,
        }
        .publish(&env);

        Ok(())
    }

    /// Post-upgrade migration logic.
    ///
    /// v2 migration: the deprecated `resolved` and `canceled` boolean fields have been
    /// removed from the `Pool` struct. All state is now represented exclusively by the
    /// `state: MarketState` field. Existing pools stored with the old schema are
    /// automatically handled by Soroban's XDR codec — the removed fields are simply
    /// ignored on read, so no explicit data rewrite is required.
    pub fn migrate_state(env: Env, admin: Address) -> Result<(), PredifiError> {
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "migrate_state")?;
        Ok(())
    }

    /// Returns true if the given token is on the allowed betting whitelist.
    pub fn is_token_allowed(env: Env, token: Address) -> bool {
        Self::is_token_whitelisted(&env, &token)
    }

    /// Get referral cut in basis points (e.g. 5000 = 50% of referrer's fee share).
    pub fn get_referral_cut_bps(env: Env) -> u32 {
        Self::read_referral_cut_bps(&env)
    }

    /// Returns the current treasury and referral fee percentages as a [`FeeInfo`].
    ///
    /// - `treasury_fee_bps`: protocol fee charged on winnings (set via `set_fee_bps`).
    /// - `referral_fee_bps`: share of the protocol fee paid to referrers (set via `set_referral_cut_bps`).
    pub fn get_fees(env: Env) -> FeeInfo {
        FeeInfo {
            treasury_fee_bps: Self::get_config(&env).fee_bps,
            referral_fee_bps: Self::read_referral_cut_bps(&env),
        }
    }

    /// Return an aggregated metadata view of contract config and protocol state.
    pub fn get_contract_info(env: Env) -> ContractInfo {
        let config = Self::get_config(&env);
        let current_admin = Self::get_access_control_admin(&env, &config.access_control);

        ContractInfo {
            version: env
                .storage()
                .instance()
                .get(&DataKey::Version)
                .unwrap_or(0u32),
            current_admin,
            is_paused: Self::is_paused(&env),
            total_pools: env
                .storage()
                .instance()
                .get(&DataKey::PoolIdCtr)
                .unwrap_or(0u64),
            fee_bps: config.fee_bps,
            referral_cut_bps: Self::read_referral_cut_bps(&env),
            treasury: config.treasury,
            access_control: config.access_control,
            resolution_delay: config.resolution_delay,
            min_pool_duration: config.min_pool_duration,
            min_stake: config.min_stake,
            max_predictions_per_user: config.max_predictions_per_user,
        }
    }

    /// Get total referred volume for a (referrer, pool_id) in base token units.
    pub fn get_referred_volume(env: Env, referrer: Address, pool_id: u64) -> i128 {
        let key = DataKey::ReferredVolume(referrer, pool_id);
        let vol = env.storage().persistent().get(&key).unwrap_or(0);
        if env.storage().persistent().has(&key) {
            Self::extend_persistent(&env, &key);
        }
        vol
    }

    /// Withdraw accumulated protocol fees or unused liquidity from the contract.
    /// Only callable by Admin (role 0).
    ///
    /// # Arguments
    /// * `admin` - Address with Admin role (must provide auth)
    /// * `token` - The token contract address to withdraw
    /// * `amount` - Amount to withdraw (must be > 0)
    /// * `recipient` - Address to receive the withdrawn funds (typically treasury)
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Security
    /// - Requires Admin role (0)
    /// - Emits TreasuryWithdrawnEvent for audit trail
    /// - Validates amount > 0
    /// - Checks contract has sufficient balance
    pub fn withdraw_treasury(
        env: Env,
        admin: Address,
        token: Address,
        amount: i128,
        recipient: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();

        // Verify admin role
        Self::require_admin_role(&env, &admin, "withdraw_treasury")?;

        // Validate amount
        if amount <= 0 {
            return Err(PredifiError::InvalidAmount);
        }

        // Get token client and check balance
        let token_client = token::Client::new(&env, &token);
        let contract_balance = token_client.balance(&env.current_contract_address());

        if contract_balance < amount {
            return Err(PredifiError::InsufficientBalance);
        }

        // Transfer tokens to recipient
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);

        // Emit audit event
        TreasuryWithdrawnEvent {
            admin: admin.clone(),
            token: token.clone(),
            amount,
            recipient: recipient.clone(),
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    /// Create a new prediction pool. Returns the new pool ID.
    ///
    /// PRE: end_time > current_time (INV-8)
    /// POST: Pool.state = Active, Pool.total_stake = initial_liquidity (if provided)
    ///
    /// # Arguments
    /// * `creator`           - Address of the pool creator (must provide auth).
    /// * `end_time`          - Unix timestamp after which no more predictions are accepted.
    /// * `token`             - The Stellar token contract address used for staking.
    /// * `options_count`     - Number of possible outcomes (must be >= 2 and <= MAX_OPTIONS_COUNT).
    /// * `description`       - Short human-readable description of the event (max 256 bytes).
    /// * `metadata_url`      - URL pointing to extended metadata, e.g. an IPFS link (max 512 bytes).
    /// * `min_stake`         - Minimum stake amount per prediction (must be > 0).
    /// * `max_stake`         - Maximum stake amount per prediction (0 = no limit, else must be >= min_stake).
    /// * `initial_liquidity` - Optional initial liquidity to provide (house money). Must be > 0 if provided.
    #[allow(clippy::too_many_arguments)]
    pub fn create_pool(
        env: Env,
        creator: Address,
        end_time: u64,
        token: Address,
        options_count: u32,
        category: Symbol,
        config: PoolConfig,
    ) -> u64 {
        Self::require_not_paused(&env);
        creator.require_auth();

        // Validate: category must be in the allowed list, else fallback to CATEGORY_OTHER
        let normalized_category = Self::validate_category(&env, &category);

        // Validate: token must be on the allowed betting whitelist
        if !Self::is_token_whitelisted(&env, &token) {
            soroban_sdk::panic_with_error!(&env, PredifiError::TokenNotWhitelisted);
        }

        let current_time = env.ledger().timestamp();

        // Validate: end_time must be in the future
        assert!(end_time > current_time, "end_time must be in the future");

        let min_pool_duration = env
            .storage()
            .instance()
            .get::<DataKey, Config>(&DataKey::Config)
            .map(|c| c.min_pool_duration)
            .unwrap_or(DEFAULT_MIN_POOL_DURATION);

        // Validate: minimum pool duration
        assert!(
            end_time >= current_time + min_pool_duration,
            "end_time must be at least min_pool_duration in the future"
        );

        // Validate: options_count must be at least 2 (binary or more outcomes)
        assert!(options_count >= 2, "options_count must be at least 2");

        // Validate: options_count must not exceed maximum limit
        assert!(
            options_count <= MAX_OPTIONS_COUNT,
            "options_count exceeds maximum allowed value"
        );

        // Validate: initial_liquidity must be non-negative if provided
        assert!(
            config.initial_liquidity >= 0,
            "initial_liquidity must be non-negative"
        );

        // Validate: initial_liquidity must not exceed maximum limit
        assert!(
            config.initial_liquidity <= MAX_INITIAL_LIQUIDITY,
            "initial_liquidity exceeds maximum allowed value"
        );

        // Validate: required_resolutions must be at least 1
        assert!(
            config.required_resolutions >= 1,
            "required_resolutions must be at least 1"
        );

        // Validate: required_resolutions must not exceed the number of active operators.
        // If required_resolutions > operator_count, the pool can never reach the resolution
        // threshold and will be permanently stuck in the Active state.
        // WARNING: This is a hard check — pool creation will fail if there are not enough
        // operators registered in the access_control contract to satisfy required_resolutions.
        // Note: If operator_count is 0, the pool can still be resolved by oracles.
        {
            let cfg = Self::get_config(&env);
            let operator_count: u32 = env.invoke_contract(
                &cfg.access_control,
                &Symbol::new(&env, "get_operator_count"),
                soroban_sdk::vec![&env],
            );
            if operator_count > 0 && config.required_resolutions > operator_count {
                soroban_sdk::panic_with_error!(
                    &env,
                    PredifiError::RequiredResolutionsExceedOperators
                );
            }
        }

        // Note: Token address validation is deferred to when the token is actually used.
        // This is the standard pattern in Soroban - invalid tokens will fail when
        // transfers are attempted during place_prediction.

        assert!(
            config.description.len() <= 256,
            "description exceeds 256 bytes"
        );
        if config.metadata_url.len() > 512 {
            soroban_sdk::panic_with_error!(&env, PredifiError::MetadataUrlInvalid);
        }

        // Validate stake limits
        assert!(config.min_stake > 0, "min_stake must be greater than zero");
        assert!(
            config.max_stake == 0 || config.max_stake >= config.min_stake,
            "max_stake must be zero (unlimited) or >= min_stake"
        );
        // Validate: min_total_stake must be strictly positive (> 0)
        assert!(
            config.min_total_stake > 0,
            "min_total_stake must be greater than zero"
        );
        assert!(config.max_total_stake >= 0, "max_total_stake must be >= 0");

        if !config.outcome_descriptions.is_empty() {
            assert!(
                config.outcome_descriptions.len() == options_count,
                "outcome_descriptions length must equal options_count"
            );
        }

        let pool_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PoolIdCtr)
            .unwrap_or(0);
        // Initialize pool data structure
        let pool = Pool {
            end_time,
            state: MarketState::Active,
            outcome: 0,
            token: token.clone(),
            total_stake: config.initial_liquidity,
            category: normalized_category,
            description: config.description.clone(),
            metadata_url: config.metadata_url.clone(),
            options_count,
            min_stake: config.min_stake,
            max_stake: config.max_stake,
            min_total_stake: config.min_total_stake,
            max_total_stake: config.max_total_stake,
            initial_liquidity: config.initial_liquidity,
            creator: creator.clone(),
            required_resolutions: config.required_resolutions,
            private: config.private,
            whitelist_key: config.whitelist_key.clone(),
            outcome_descriptions: config.outcome_descriptions.clone(),
            fee_bps: 0, // Will be set at resolution
            participants_count: 0,
        };

        let pool_key = DataKey::Pool(pool_id);
        env.storage().persistent().set(&pool_key, &pool);
        Self::bump_ttl(&env, &pool_key);

        let pc_key = DataKey::PartCnt(pool_id);
        env.storage().persistent().set(&pc_key, &0u32);
        Self::bump_ttl(&env, &pc_key);

        // Initialize optimized batch storage with zeros to avoid expensive fallback reads
        let mut initial_stakes = Vec::new(&env);
        for _ in 0..options_count {
            initial_stakes.push_back(0i128);
        }
        let stakes_key = DataKey::OutStakes(pool_id);
        env.storage().persistent().set(&stakes_key, &initial_stakes);
        Self::extend_persistent(&env, &stakes_key);

        // Transfer initial liquidity from creator to contract if provided
        if config.initial_liquidity > 0 {
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(
                &creator,
                env.current_contract_address(),
                &config.initial_liquidity,
            );
        }

        // Update category index
        let category_count_key = DataKey::CatPoolCt(category.clone());
        let category_count: u32 = env
            .storage()
            .persistent()
            .get(&category_count_key)
            .unwrap_or(0);

        let category_index_key = DataKey::CatPoolIx(category.clone(), category_count);
        env.storage()
            .persistent()
            .set(&category_index_key, &pool_id);
        Self::bump_ttl(&env, &category_index_key);

        env.storage()
            .persistent()
            .set(&category_count_key, &(category_count + 1));
        Self::bump_ttl(&env, &category_count_key);

        env.storage()
            .instance()
            .set(&DataKey::PoolIdCtr, &(pool_id + 1));
        Self::extend_instance(&env);

        PoolCreatedEvent {
            pool_id,
            creator: creator.clone(),
            end_time,
            token,
            options_count,
            metadata_url: config.metadata_url,
            initial_liquidity: config.initial_liquidity,
            category,
            required_resolutions: config.required_resolutions,
            max_total_stake: config.max_total_stake,
            outcome_descriptions: config.outcome_descriptions,
        }
        .publish(&env);

        // Emit initial liquidity event if liquidity was provided
        if config.initial_liquidity > 0 {
            InitialLiquidityProvidedEvent {
                pool_id,
                creator,
                amount: config.initial_liquidity,
            }
            .publish(&env);
        }

        // Register pool in the global active pool index.
        Self::add_to_active_index(&env, pool_id);

        pool_id
    }

    /// Increase the maximum total stake cap for a pool.
    /// Only the pool creator can increase it, and only before the market ends.
    ///
    /// - `new_max_total_stake` must be >= current `pool.total_stake`.
    /// - Setting to 0 means "no cap" (only allowed if current cap is 0 or increasing from a non-zero).
    pub fn increase_max_total_stake(
        env: Env,
        creator: Address,
        pool_id: u64,
        new_max_total_stake: i128,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        creator.require_auth();

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        if pool.creator != creator {
            return Err(PredifiError::Unauthorized);
        }

        // Pool must still be active and not ended
        // if pool.state != MarketState::Active {
        //     return Err(PredifiError::InvalidPoolState);
        // }
        if !Self::is_pool_active(&pool) {
            return Err(PredifiError::InvalidPoolState);
        }

        assert!(env.ledger().timestamp() < pool.end_time, "Pool has ended");

        // Must not set a cap below what is already staked
        assert!(
            new_max_total_stake == 0 || new_max_total_stake >= pool.total_stake,
            "new_max_total_stake must be zero (unlimited) or >= total_stake"
        );

        // Only allow increasing the cap (or setting unlimited)
        if pool.max_total_stake > 0 && new_max_total_stake > 0 {
            assert!(
                new_max_total_stake >= pool.max_total_stake,
                "new_max_total_stake must be >= current max_total_stake"
            );
        }

        pool.max_total_stake = new_max_total_stake;
        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        Ok(())
    }

    /// Update the description of a pool before any participant has joined.
    ///
    /// Allows the pool creator or a protocol admin to correct a typo or clarify
    /// ambiguous wording. Once the first prediction is placed the description is
    /// locked to prevent fraud.
    ///
    /// # Arguments
    /// * `caller`   - Pool creator **or** an address with Admin role (0).
    /// * `pool_id`  - The pool to update.
    /// * `new_desc` - Replacement description (max 256 bytes, must be non-empty).
    ///
    /// # Errors
    /// * `Unauthorized`     – caller is neither the creator nor an admin.
    /// * `InvalidPoolState` – pool is not `Active`, has ended, or already has participants.
    /// * `InvalidAmount`    – description is empty or exceeds 256 bytes.
    pub fn update_pool_description(
        env: Env,
        caller: Address,
        pool_id: u64,
        new_desc: String,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        caller.require_auth();

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        // Only the creator or a protocol admin may update the description.
        let is_creator = pool.creator == caller;
        let is_admin = Self::require_admin_role(&env, &caller, "update_pool_description").is_ok();
        if !is_creator && !is_admin {
            return Err(PredifiError::Unauthorized);
        }

        // Pool must still be active (not resolved or canceled).
        if !Self::is_pool_active(&pool) {
            return Err(PredifiError::InvalidPoolState);
        }

        // Pool must not have ended.
        if env.ledger().timestamp() >= pool.end_time {
            return Err(PredifiError::InvalidPoolState);
        }

        // Lock the description once any participant has joined — equivalent to
        // "pool has started" in this contract's model (no separate start_time).
        // We read the PartCnt key which is the authoritative participant counter.
        let pc_key = DataKey::PartCnt(pool_id);
        let participants: u32 = env.storage().persistent().get(&pc_key).unwrap_or(0);
        if participants > 0 {
            return Err(PredifiError::InvalidPoolState);
        }

        // Validate the new description: non-empty and within the 256-byte limit.
        if new_desc.is_empty() || new_desc.len() > 256 {
            return Err(PredifiError::InvalidAmount);
        }

        pool.description = new_desc.clone();
        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        PoolDescriptionUpdatedEvent {
            pool_id,
            caller,
            new_description: new_desc,
        }
        .publish(&env);

        Ok(())
    }

    /// Resolve a pool with a winning outcome. Caller must have Operator role (1).
    /// Cannot resolve a canceled pool.
    /// PRE: pool.state = Active, operator has role 1
    /// POST: pool.state = Resolved, state transition valid (INV-2)
    pub fn resolve_pool(
        env: Env,
        operator: Address,
        pool_id: u64,
        outcome: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        operator.require_auth();
        Self::require_operator_role_for_resolution(&env, &operator, pool_id)?;

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");


        // if pool.state != MarketState::Active {
        //     return Err(PredifiError::InvalidPoolState);
        // }
        if !Self::is_pool_active(&pool) {
            log!(
                &env,
                "resolve_pool rejected: pool is not active",
                pool_id,
                operator.clone(),
                outcome,
                pool.end_time
            );
            return Err(PredifiError::InvalidPoolState);
        }

        let current_time = env.ledger().timestamp();
        let config = Self::get_config(&env);
        let eligible_at = pool.end_time.saturating_add(config.resolution_delay);

        if current_time < eligible_at {
            log!(
                &env,
                "resolve_pool rejected: resolution delay not met",
                pool_id,
                operator.clone(),
                outcome,
                current_time,
                eligible_at
            );
            return Err(PredifiError::ResolutionDelayNotMet);
        }

        // Validate: outcome must be within the valid options range
        if outcome >= pool.options_count {
            log!(
                &env,
                "resolve_pool rejected: outcome is out of bounds",
                pool_id,
                operator.clone(),
                outcome,
                pool.options_count
            );
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidOutcome);
        }

        // --- Multi-resolution Voting Logic ---

        // Check if this operator has already voted for this pool
        let vote_key = DataKey::ResVote(pool_id, operator.clone());
        if env.storage().persistent().has(&vote_key) {
            log!(
                &env,
                "resolve_pool rejected: operator already voted",
                pool_id,
                operator.clone(),
                outcome
            );
            return Err(PredifiError::OracleAlreadyVoted); // Reusing error code for operators
        }

        // Record the operator's vote
        env.storage().persistent().set(&vote_key, &outcome);
        Self::extend_persistent(&env, &vote_key);

        // Increment total number of votes cast for this pool
        let total_votes_key = DataKey::ResTotal(pool_id);
        let total_votes: u32 = env
            .storage()
            .persistent()
            .get(&total_votes_key)
            .unwrap_or(0);
        let new_total_votes = total_votes + 1;
        env.storage()
            .persistent()
            .set(&total_votes_key, &new_total_votes);
        Self::extend_persistent(&env, &total_votes_key);

        // Increment specific outcome vote count
        let outcome_votes_key = DataKey::ResVoteCt(pool_id, outcome);
        let outcome_votes: u32 = env
            .storage()
            .persistent()
            .get(&outcome_votes_key)
            .unwrap_or(0);
        let new_outcome_votes = outcome_votes + 1;
        env.storage()
            .persistent()
            .set(&outcome_votes_key, &new_outcome_votes);
        Self::extend_persistent(&env, &outcome_votes_key);

        // Detect conflicts
        if new_total_votes > new_outcome_votes {
            for i in 0..pool.options_count {
                if i == outcome {
                    continue;
                }
                let other_key = DataKey::ResVoteCt(pool_id, i);
                if env.storage().persistent().has(&other_key) {
                    ResolutionConflictEvent {
                        pool_id,
                        oracle: operator.clone(),
                        outcome,
                        existing_outcome: i,
                    }
                    .publish(&env);
                    break;
                }
            }
        }

        // Check if the required threshold has been met
        if new_outcome_votes >= pool.required_resolutions {
            pool.state = MarketState::Resolved;
            pool.outcome = outcome;
            pool.fee_bps = Self::calculate_dynamic_fee(&env, &pool);

            env.storage().persistent().set(&pool_key, &pool);

            // Remove from global active index now that the pool is resolved.
            Self::remove_from_active_index(&env, pool_id);
            Self::bump_ttl(&env, &pool_key);

            // Retrieve winning-outcome stake for the diagnostic event efficiently
            let winning_stake = Self::get_outcome_stake(env.clone(), pool_id, outcome);

            PoolResolvedEvent {
                pool_id,
                operator,
                outcome,
            }
            .publish(&env);

            PoolResolvedDiagEvent {
                pool_id,
                outcome,
                total_stake: pool.total_stake,
                winning_stake,
                timestamp: env.ledger().timestamp(),
            }
            .publish(&env);
        }

        Ok(())
    }

    /// Mark a pool as ready for resolution and emit an event.
    /// Can be called by anyone once the resolution delay has passed.
    pub fn mark_pool_ready(env: Env, pool_id: u64) -> Result<(), PredifiError> {
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        if pool.state != MarketState::Active {
            return Err(PredifiError::InvalidPoolState);
        }

        let config = Self::get_config(&env);
        let current_time = env.ledger().timestamp();

        if current_time >= pool.end_time.saturating_add(config.resolution_delay) {
            PoolReadyForResolutionEvent {
                pool_id,
                timestamp: current_time,
            }
            .publish(&env);
            Ok(())
        } else {
            Err(PredifiError::ResolutionDelayNotMet)
        }
    }

    /// Cancel an active pool. Caller must have Operator role (1).
    /// Cancel a pool, freezing all betting and enabling refund process.
    /// Only callable by Admin (role 0) - can cancel any pool for any reason.
    ///
    /// # Arguments
    /// * `caller`  - The address requesting the cancellation (must be admin).
    /// * `pool_id` - The ID of the pool to cancel.
    /// * `reason`  - A short description of why the pool is being canceled.
    ///
    /// # Errors
    /// - `Unauthorized` if caller is not admin/operator and not the pool creator, or if creator
    ///   attempts to cancel a pool that already has bets beyond initial liquidity.
    /// - `PoolNotResolved` error (code 22) is returned if trying to cancel an already resolved pool.
    /// PRE: pool.state = Active, caller has role 0/1 OR (caller == pool.creator AND total_stake <= initial_liquidity)
    /// POST: pool.state = Canceled, state transition valid (INV-2)
    pub fn cancel_pool(
        env: Env,
        operator: Address,
        pool_id: u64,
        reason: String,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        operator.require_auth();

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        // Determine if caller is admin/operator (role 0 or 1)
        let is_privileged = Self::require_role(&env, &operator, 0).is_ok()
            || Self::require_role(&env, &operator, 1).is_ok();

        if !is_privileged {
            // Check if pool is overdue (past end_time + CANCELATION_DELAY)
            let current_time = env.ledger().timestamp();
            let overdue_threshold = pool.end_time + CANCELATION_DELAY;

            if current_time > overdue_threshold {
                // Allow any user to cancel overdue pools
                // This is a failsafe to unlock funds when resolution is delayed
            } else {
                // Allow creator to cancel only if no bets have been placed beyond initial liquidity
                if operator != pool.creator {
                    return Err(PredifiError::Unauthorized);
                }
                if pool.total_stake > pool.initial_liquidity {
                    return Err(PredifiError::Unauthorized);
                }
            }
        }

        // Ensure resolved pools cannot be canceled
        if pool.state == MarketState::Resolved {
            return Err(PredifiError::PoolNotResolved);
        }

        if !Self::is_pool_active(&pool) {
            return Err(PredifiError::InvalidPoolState);
        }

        pool.state = MarketState::Canceled;
        env.storage().persistent().set(&pool_key, &pool);
        Self::bump_ttl(&env, &pool_key);
        Self::remove_from_active_index(&env, pool_id);

        PoolCanceledEvent {
            pool_id,
            caller: operator.clone(),
            reason,
            operator,
        }
        .publish(&env);

        Ok(())
    }

    /// Place a prediction on a pool. Cannot predict on canceled or resolved pools.
    /// Optional `referrer`: if set, that address will receive a referral cut of the protocol fee
    /// when this user claims winnings. Stored only on first prediction for (user, pool_id).
    /// PRE: amount > 0 (INV-7), pool.state = Active, current_time < pool.end_time
    /// PRE: pool.min_stake <= amount <= pool.max_stake (unless max_stake == 0)
    /// POST: pool.total_stake increases by amount, OutcomeStake increases by amount (INV-1)
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn place_prediction(
        env: Env,
        user: Address,
        pool_id: u64,
        amount: i128,
        outcome: u32,
        referrer: Option<Address>,
        invite_key: Option<Symbol>,
    ) {
        Self::require_not_paused(&env);
        user.require_auth();
        assert!(amount > 0, "amount must be positive");

        // Validate: amount must meet the global protocol minimum stake
        let global_min_stake = Self::get_config(&env).min_stake;
        if amount < global_min_stake {
            soroban_sdk::panic_with_error!(&env, PredifiError::InsufficientStake);
        }

        // Validate referrer if provided: cannot be self or contract
        if let Some(ref r) = referrer {
            assert!(r != &user, "referrer cannot be self");
            assert!(
                r != &env.current_contract_address(),
                "referrer cannot be contract"
            );
        }

        Self::enter_reentrancy_guard(&env);

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        // assert!(pool.state == MarketState::Active, "Pool is not active");
        if !Self::is_pool_active(&pool) {
            panic!("Pool is not active");
        }
        assert!(env.ledger().timestamp() < pool.end_time, "Pool has ended");

        // Check private pool authorization
        // Check private pool authorization
        if pool.private {
            let whitelist_key_data = DataKey::Whitelist(pool_id, user.clone());
            let is_whitelisted = env
                .storage()
                .persistent()
                .get(&whitelist_key_data)
                .unwrap_or(false);

            let has_valid_invite = if let Some(ref pool_key) = pool.whitelist_key {
                if let Some(ref prov_key) = invite_key {
                    pool_key == prov_key
                } else {
                    false
                }
            } else {
                false
            };

            assert!(
                is_whitelisted || user == pool.creator || has_valid_invite,
                "User not authorized for private pool"
            );
        }

        // Validate: outcome must be within the valid options range
        if outcome >= pool.options_count {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidOutcome);
        }

        // --- INTERNAL CHECKS & EFFECTS ---
        // Validate: per-pool stake limits
        if amount < pool.min_stake {
            Self::exit_reentrancy_guard(&env);
            soroban_sdk::panic_with_error!(&env, PredifiError::StakeBelowMinimum);
        }
        if pool.max_stake > 0 && amount > pool.max_stake {
            Self::exit_reentrancy_guard(&env);
            soroban_sdk::panic_with_error!(&env, PredifiError::StakeAboveMaximum);
        }

        // Enforce global pool cap (max total stake)
        if pool.max_total_stake > 0 {
            let new_total = pool.total_stake.checked_add(amount).expect("overflow");
            if new_total > pool.max_total_stake {
                Self::exit_reentrancy_guard(&env);
                soroban_sdk::panic_with_error!(&env, PredifiError::MaxTotalStakeExceeded);
            }
        }

        // Enforce maximum predictions per user limit (across all pools)
        let config = Self::get_config(&env);
        if config.max_predictions_per_user > 0 {
            let pred_key = DataKey::Pred(user.clone(), pool_id);
            let existing_pred = env.storage().persistent().get::<_, Prediction>(&pred_key);

            // If user already has a prediction on this pool, allow increasing stake (same prediction)
            // If this is a new prediction for this pool, check if user has reached the limit
            if existing_pred.is_none() {
                // Count current number of pools this user has predictions in
                let user_prediction_count_key = DataKey::UsrPrdCnt(user.clone());
                let current_count: u32 = env
                    .storage()
                    .persistent()
                    .get(&user_prediction_count_key)
                    .unwrap_or(0);

                if current_count >= config.max_predictions_per_user {
                    Self::exit_reentrancy_guard(&env);
                    soroban_sdk::panic_with_error!(&env, PredifiError::MaxPredictionsExceeded);
                }
            }
            // Note: If user already has a prediction on this pool, we allow increasing the stake
            // as it's the same prediction, not a new pool participation
        }

        let pred_key = DataKey::Pred(user.clone(), pool_id);
        let existing_pred = env.storage().persistent().get::<_, Prediction>(&pred_key);
        if let Some(mut existing_pred) = existing_pred {
            assert!(
                existing_pred.outcome == outcome,
                "Cannot change prediction outcome"
            );
            existing_pred.amount = existing_pred.amount.checked_add(amount).expect("overflow");
            env.storage().persistent().set(&pred_key, &existing_pred);
            Self::extend_persistent(&env, &pred_key);

            // Track referred volume: if this user already has a referrer, add to their volume
            let referrer_key = DataKey::Referrer(user.clone(), pool_id);
            if let Some(referrer_addr) = env.storage().persistent().get::<_, Address>(&referrer_key)
            {
                Self::extend_persistent(&env, &referrer_key);
                let vol_key = DataKey::ReferredVolume(referrer_addr.clone(), pool_id);
                let vol: i128 = env.storage().persistent().get(&vol_key).unwrap_or(0);
                env.storage().persistent().set(&vol_key, &(vol + amount));
                Self::extend_persistent(&env, &vol_key);
            }
        } else {
            env.storage()
                .persistent()
                .set(&pred_key, &Prediction { amount, outcome });
            Self::extend_persistent(&env, &pred_key);

            // Store referrer on first prediction and track referred volume.
            // NOTE: Only one referrer per (user, pool) is supported today.
            // See DataKey::Referrer for a note on extending this to multiple referrers.
            if let Some(ref referrer_addr) = referrer {
                let referrer_key = DataKey::Referrer(user.clone(), pool_id);
                env.storage().persistent().set(&referrer_key, referrer_addr);
                Self::extend_persistent(&env, &referrer_key);
                let vol_key = DataKey::ReferredVolume(referrer_addr.clone(), pool_id);
                let vol: i128 = env.storage().persistent().get(&vol_key).unwrap_or(0);
                env.storage().persistent().set(&vol_key, &(vol + amount));
                Self::extend_persistent(&env, &vol_key);
            }

            let pc_key = DataKey::PartCnt(pool_id);
            let pc: u32 = env.storage().persistent().get(&pc_key).unwrap_or(0);
            env.storage().persistent().set(&pc_key, &(pc + 1));
            Self::extend_persistent(&env, &pc_key);

            // Mirror the count on the Pool struct so get_pool_participants_count
            // can read it with a single storage fetch.
            pool.participants_count = pool.participants_count.saturating_add(1);

            let count_key = DataKey::UsrPrdCnt(user.clone());
            let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);

            let index_key = DataKey::UsrPrdIdx(user.clone(), count);
            env.storage().persistent().set(&index_key, &pool_id);
            Self::extend_persistent(&env, &index_key);

            env.storage().persistent().set(&count_key, &(count + 1));
            Self::extend_persistent(&env, &count_key);
        }

        // Update total stake (INV-1)
        pool.total_stake = pool.total_stake.checked_add(amount).expect("overflow");
        env.storage().persistent().set(&pool_key, &pool);
        Self::bump_ttl(&env, &pool_key);

        // Update outcome stake (INV-1) - using optimized batch storage
        let _stakes =
            Self::update_outcome_stake(&env, pool_id, outcome, amount, pool.options_count);

        // --- INTERACTIONS ---

        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        Self::exit_reentrancy_guard(&env);

        PredictionPlacedEvent {
            pool_id,
            user: user.clone(),
            amount,
            outcome,
        }
        .publish(&env);

        // 🟡 MEDIUM ALERT: large stake detected — emit supplementary event.
        if amount >= HIGH_VALUE_THRESHOLD {
            HighValuePredictionEvent {
                pool_id,
                user,
                amount,
                outcome,
                threshold: HIGH_VALUE_THRESHOLD,
            }
            .publish(&env);
        }

        // 🟢 INFO: For markets with many outcomes (16+), emit batch stake update event
        // to avoid emitting individual events per outcome which would be impractical
        // for large tournaments (e.g., 32-team bracket).
        if pool.options_count >= 16 {
            OutcomeStakesUpdatedEvent {
                pool_id,
                options_count: pool.options_count,
                total_stake: pool.total_stake,
            }
            .publish(&env);
        }
    }

    /// Claim winnings from a resolved pool. Returns the amount paid out (0 for losers).
    /// PRE: pool.state ≠ Active
    /// POST: HasClaimed(user, pool) = true (INV-3), payout ≤ pool.total_stake (INV-4)
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn claim_winnings(env: Env, user: Address, pool_id: u64) -> Result<i128, PredifiError> {
        Self::require_not_paused(&env);
        user.require_auth();

        // 🛡️ RE-ENTRANCY GUARD: Protect against recursive withdrawal attempts
        // during value transfer to external addresses/contracts (INV-3).
        Self::enter_reentrancy_guard(&env);

        let result: Result<i128, PredifiError> = (|| {
            let pool_key = DataKey::Pool(pool_id);
            let pool: Pool = env
                .storage()
                .persistent()
                .get(&pool_key)
                .expect("Pool not found");
            Self::extend_persistent(&env, &pool_key);

            if pool.state == MarketState::Active {
                return Err(PredifiError::PoolNotResolved);
            }

            let claimed_key = DataKey::Claimed(user.clone(), pool_id);
            if env.storage().persistent().has(&claimed_key) {
                // 🔴 HIGH ALERT: repeated claim attempt on an already-claimed pool.
                SuspiciousDoubleClaimEvent {
                    user: user.clone(),
                    pool_id,
                    timestamp: env.ledger().timestamp(),
                }
                .publish(&env);
                return Err(PredifiError::AlreadyClaimed);
            }

            // --- CHECKS ---

            let pred_key = DataKey::Pred(user.clone(), pool_id);
            let prediction: Option<Prediction> = env.storage().persistent().get(&pred_key);

            if env.storage().persistent().has(&pred_key) {
                Self::extend_persistent(&env, &pred_key);
            }

            let prediction = match prediction {
                Some(p) => p,
                None => {
                    return Ok(0);
                }
            };

            // --- EFFECTS ---

            // Mark as claimed immediately to prevent re-entrancy (INV-3)
            env.storage().persistent().set(&claimed_key, &true);
            Self::bump_ttl(&env, &claimed_key);

            if pool.state == MarketState::Canceled {
                // --- INTERACTIONS (Refund) ---
                let token_client = token::Client::new(&env, &pool.token);
                token_client.transfer(&env.current_contract_address(), &user, &prediction.amount);

                WinningsClaimedEvent {
                    pool_id,
                    user: user.clone(),
                    amount: prediction.amount,
                }
                .publish(&env);

                return Ok(prediction.amount);
            }

            if prediction.outcome != pool.outcome {
                return Ok(0);
            }

            // Get winning stake using optimized batch storage
            // Get winning stake efficiently using single outcome getter
            let winning_stake = Self::get_outcome_stake(env.clone(), pool_id, pool.outcome);

            if winning_stake == 0 {
                return Ok(0);
            }

            // Protocol fee: deducted from pool before distribution
            // Use pool-specific fee (calculated at resolution) if available, else fallback to global
            let fee_bps_i = if pool.fee_bps > 0 || pool.state == MarketState::Resolved {
                pool.fee_bps as i128
            } else {
                let config = Self::get_config(&env);
                config.fee_bps as i128
            };
            let protocol_fee_total =
                SafeMath::percentage(pool.total_stake, fee_bps_i, RoundingMode::ProtocolFavor)
                    .map_err(|_| PredifiError::InvalidAmount)?;
            let payout_pool = pool
                .total_stake
                .checked_sub(protocol_fee_total)
                .ok_or(PredifiError::InvalidAmount)?;

            // Winnings = user's share of the payout pool (after fee)
            let winnings = SafeMath::calculate_share(prediction.amount, winning_stake, payout_pool)
                .map_err(|_| PredifiError::InvalidAmount)?;

            // Verify invariant: winnings ≤ total_stake (INV-4)
            assert!(winnings <= pool.total_stake, "Winnings exceed total stake");

            // --- INTERACTIONS ---
            let token_client = token::Client::new(&env, &pool.token);

            // Referral: portion of protocol fee attributable to this user goes to referrer
            let referrer_key = DataKey::Referrer(user.clone(), pool_id);
            if let Some(referrer) = env.storage().persistent().get::<_, Address>(&referrer_key) {
                Self::extend_persistent(&env, &referrer_key);
                if protocol_fee_total > 0 && pool.total_stake > 0 {
                    let protocol_fee_share = SafeMath::proportion(
                        prediction.amount,
                        pool.total_stake,
                        protocol_fee_total,
                        RoundingMode::Neutral,
                    )
                    .map_err(|_| PredifiError::InvalidAmount)?;
                    let referral_cut_bps = Self::read_referral_cut_bps(&env) as i128;
                    let referral_amount = SafeMath::percentage(
                        protocol_fee_share,
                        referral_cut_bps,
                        RoundingMode::Neutral,
                    )
                    .map_err(|_| PredifiError::InvalidAmount)?;
                    if referral_amount > 0 {
                        token_client.transfer(
                            &env.current_contract_address(),
                            &referrer,
                            &referral_amount,
                        );
                        ReferralPaidEvent {
                            pool_id,
                            referrer: referrer.clone(),
                            referred_user: user.clone(),
                            amount: referral_amount,
                        }
                        .publish(&env);
                    }
                }
            }

            token_client.transfer(&env.current_contract_address(), &user, &winnings);

            WinningsClaimedEvent {
                pool_id,
                user: user.clone(),
                amount: winnings,
            }
            .publish(&env);

            Ok(winnings)
        })();

        Self::exit_reentrancy_guard(&env);
        result
    }

    /// Claim a refund from a canceled pool. Returns the refunded amount.
    /// Only available for canceled pools. User receives their full original stake.
    ///
    /// PRE: pool.state = Canceled, user has a prediction on the pool
    /// POST: HasClaimed(user, pool) = true (INV-3), user receives full stake amount
    ///
    /// # Arguments
    /// * `user` - Address claiming the refund (must provide auth)
    /// * `pool_id` - ID of the canceled pool
    ///
    /// # Returns
    /// Ok(amount) - Refund successfully claimed, returns refunded amount
    /// Err(PredifiError) - Operation failed with specific error code
    ///
    /// # Errors
    /// - `InvalidPoolState` if pool doesn't exist or is not canceled
    /// - `InsufficientBalance` if user has no stake to refund
    /// - `AlreadyClaimed` if user already claimed refund for this pool
    /// - `PoolNotResolved` if pool is resolved (not canceled)
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn claim_refund(env: Env, user: Address, pool_id: u64) -> Result<i128, PredifiError> {
        Self::require_not_paused(&env);
        user.require_auth();

        // 🛡️ RE-ENTRANCY GUARD: Protect against recursive withdrawal attempts
        // during value transfer to external addresses/contracts (INV-3).
        Self::enter_reentrancy_guard(&env);

        let result: Result<i128, PredifiError> = (|| {
            // --- CHECKS ---

            let pool_key = DataKey::Pool(pool_id);
            let pool: Pool = match env.storage().persistent().get(&pool_key) {
                Some(p) => p,
                None => {
                    return Err(PredifiError::InvalidPoolState);
                }
            };
            Self::extend_persistent(&env, &pool_key);

            // Verify pool is canceled
            if pool.state != MarketState::Canceled {
                return Err(PredifiError::InvalidPoolState);
            }

            // Check if user already claimed refund
            let claimed_key = DataKey::Claimed(user.clone(), pool_id);
            if env.storage().persistent().has(&claimed_key) {
                return Err(PredifiError::AlreadyClaimed);
            }

            // Get user's prediction
            let pred_key = DataKey::Pred(user.clone(), pool_id);
            let prediction: Option<Prediction> = env.storage().persistent().get(&pred_key);

            if env.storage().persistent().has(&pred_key) {
                Self::extend_persistent(&env, &pred_key);
            }

            let prediction = match prediction {
                Some(p) => p,
                None => {
                    return Err(PredifiError::InsufficientBalance);
                }
            };

            // Verify user has a non-zero stake
            if prediction.amount <= 0 {
                return Err(PredifiError::InsufficientBalance);
            }

            // --- EFFECTS ---

            // Mark as claimed immediately to prevent re-entrancy (INV-3)
            env.storage().persistent().set(&claimed_key, &true);
            Self::bump_ttl(&env, &claimed_key);

            let refund_amount = prediction.amount;

            // --- INTERACTIONS ---

            let token_client = token::Client::new(&env, &pool.token);
            token_client.transfer(&env.current_contract_address(), &user, &refund_amount);

            RefundClaimedEvent {
                pool_id,
                user: user.clone(),
                amount: refund_amount,
            }
            .publish(&env);

            Ok(refund_amount)
        })();

        Self::exit_reentrancy_guard(&env);
        result
    }

    /// Update the stake limits for an active pool. Caller must have Operator role (1).
    /// PRE: pool.state = Active, operator has role 1
    /// POST: pool.min_stake and pool.max_stake updated
    pub fn set_stake_limits(
        env: Env,
        operator: Address,
        pool_id: u64,
        min_stake: i128,
        max_stake: i128,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        operator.require_auth();
        Self::require_role(&env, &operator, 1)?;

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        if pool.state != MarketState::Active {
            return Err(PredifiError::InvalidPoolState);
        }

        if min_stake <= 0 {
            return Err(PredifiError::StakeBelowMinimum);
        }
        if max_stake != 0 && max_stake < min_stake {
            return Err(PredifiError::StakeAboveMaximum);
        }

        pool.min_stake = min_stake;
        pool.max_stake = max_stake;

        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        StakeLimitsUpdatedEvent {
            pool_id,
            operator,
            min_stake,
            max_stake,
        }
        .publish(&env);

        Ok(())
    }

    /// Get a paginated list of a user's predictions.
    pub fn get_user_predictions(
        env: Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<UserPredictionDetail> {
        let count_key = DataKey::UsrPrdCnt(user.clone());
        let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
        if env.storage().persistent().has(&count_key) {
            Self::extend_persistent(&env, &count_key);
        }

        let mut results = Vec::new(&env);

        if offset >= count || limit == 0 {
            return results;
        }

        let end = core::cmp::min(offset.saturating_add(limit), count);

        for i in offset..end {
            let index_key = DataKey::UsrPrdIdx(user.clone(), i);
            let pool_id: u64 = env
                .storage()
                .persistent()
                .get(&index_key)
                .expect("index not found");
            Self::extend_persistent(&env, &index_key);

            let pred_key = DataKey::Pred(user.clone(), pool_id);
            let prediction: Prediction = env
                .storage()
                .persistent()
                .get(&pred_key)
                .expect("prediction not found");
            Self::extend_persistent(&env, &pred_key);

            let pool_key = DataKey::Pool(pool_id);
            let pool: Pool = env
                .storage()
                .persistent()
                .get(&pool_key)
                .expect("pool not found");
            Self::extend_persistent(&env, &pool_key);

            results.push_back(UserPredictionDetail {
                pool_id,
                amount: prediction.amount,
                user_outcome: prediction.outcome,
                pool_end_time: pool.end_time,
                pool_state: pool.state,
                pool_outcome: pool.outcome,
            });
        }

        results
    }

    /// This function is optimized for markets with many outcomes (e.g., 32+ teams).
    /// Instead of making N storage reads (one per outcome), it makes a single read.
    ///
    /// Returns a Vec of stakes where index corresponds to outcome index.
    /// For example, `stake\[0\]` is the total amount bet on outcome 0.
    pub fn get_pool(env: Env, pool_id: u64) -> Pool {
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);
        pool
    }

    /// Returns the configuration fields of a pool as a `PoolConfig` struct.
    ///
    /// This is a lightweight alternative to `get_pool` when only the
    /// configuration parameters are needed (description, stake limits, etc.)
    /// without fetching the full runtime state (total_stake, outcome, etc.).
    ///
    /// # Panics
    /// Panics with "Pool not found" if no pool exists for the given `pool_id`.
    pub fn get_pool_config(env: Env, pool_id: u64) -> PoolConfig {
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);
        PoolConfig {
            description: pool.description,
            metadata_url: pool.metadata_url,
            min_stake: pool.min_stake,
            max_stake: pool.max_stake,
            min_total_stake: pool.min_total_stake,
            max_total_stake: pool.max_total_stake,
            initial_liquidity: pool.initial_liquidity,
            required_resolutions: pool.required_resolutions,
            private: pool.private,
            whitelist_key: pool.whitelist_key,
            outcome_descriptions: pool.outcome_descriptions,
        }
    }

    pub fn get_pool_outcome_stakes(env: Env, pool_id: u64) -> Vec<i128> {
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        Self::get_outcome_stakes(&env, pool_id, pool.options_count)
    }

    /// Get a specific outcome's stake (backward compatible).
    ///
    /// Optimized to read the batch `OutStakes` key directly when available,
    /// avoiding a full `Pool` struct deserialization. Falls back to loading
    /// the pool only when the batch key is missing (pre-optimization data).
    pub fn get_outcome_stake(env: Env, pool_id: u64, outcome: u32) -> i128 {
        // Optimization: Try individual key first (most common case, cheapest to read)
        let stake_key = DataKey::OutStake(pool_id, outcome);
        if let Some(stake) = env.storage().persistent().get::<_, i128>(&stake_key) {
            Self::extend_persistent(&env, &stake_key);
            return stake;
        }

        // Fallback: Try optimized batch key
        let batch_key = DataKey::OutStakes(pool_id);
        if let Some(stakes) = env.storage().persistent().get::<_, Vec<i128>>(&batch_key) {
            Self::extend_persistent(&env, &batch_key);
            return stakes.get(outcome).unwrap_or(0);
        }

        // Final fallback: reconstructed if neither exists (unlikely in modern version)
        0
    }

    /// Get a paginated list of pool IDs by category.
    pub fn get_pools_by_category(env: Env, category: Symbol, offset: u32, limit: u32) -> Vec<u64> {
        let count_key = DataKey::CatPoolCt(category.clone());
        let count: u32 = if let Some(c) = env.storage().persistent().get(&count_key) {
            Self::extend_persistent(&env, &count_key);
            c
        } else {
            0
        };

        let mut results = Vec::new(&env);

        if offset >= count || limit == 0 {
            return results;
        }

        let start_index = count.saturating_sub(offset).saturating_sub(1);
        let num_to_take = core::cmp::min(limit, count.saturating_sub(offset));

        for i in 0..num_to_take {
            let index = start_index.saturating_sub(i);
            let index_key = DataKey::CatPoolIx(category.clone(), index);
            let pool_id: u64 = env
                .storage()
                .persistent()
                .get(&index_key)
                .expect("index not found");
            Self::extend_persistent(&env, &index_key);

            results.push_back(pool_id);
        }

        results
    }

    /// Get a paginated list of all currently active pool IDs across all categories.
    ///
    /// Returns pool IDs in insertion order (oldest first within each page).
    /// Pools are removed from this list when they are resolved or canceled,
    /// so every ID returned is guaranteed to belong to an active pool.
    ///
    /// # Arguments
    /// * `offset` - Number of entries to skip (0-based).
    /// * `limit`  - Maximum number of entries to return.
    ///
    /// # Returns
    /// A `Vec<u64>` of active pool IDs. Returns an empty vec if `offset`
    /// is beyond the current count or `limit` is 0.
    pub fn get_active_pools(env: Env, offset: u32, limit: u32) -> Vec<u64> {
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::ActivePoolCtr)
            .unwrap_or(0);
        let mut results = Vec::new(&env);

        if offset >= count || limit == 0 {
            return results;
        }

        Self::extend_instance(&env);

        let end = core::cmp::min(offset.saturating_add(limit), count);

        for i in offset..end {
            let slot_key = DataKey::ActivePool(i);
            if let Some(pool_id) = env.storage().persistent().get(&slot_key) {
                Self::extend_persistent(&env, &slot_key);
                results.push_back(pool_id);
            }
        }

        results
    }

    /// Return the total number of currently active (open) pools.
    ///
    /// This is an O(1) read of the `ActivePoolCtr` instance-storage counter
    /// that is maintained by `add_to_active_index` / `remove_from_active_index`.
    /// Frontends can use this to display "Showing N of M active pools" without
    /// fetching every page.
    pub fn get_active_pools_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::ActivePoolCtr)
            .unwrap_or(0)
    }

    /// Add a user to a private pool's whitelist. Only callable by pool creator.
    pub fn add_to_whitelist(
        env: Env,
        creator: Address,
        pool_id: u64,
        user: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        creator.require_auth();

        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        if pool.creator != creator {
            return Err(PredifiError::Unauthorized);
        }

        assert!(pool.private, "Pool is not private");

        let whitelist_key = DataKey::Whitelist(pool_id, user.clone());
        env.storage().persistent().set(&whitelist_key, &true);
        Self::extend_persistent(&env, &whitelist_key);

        AddedToWhitelistEvent {
            pool_id,
            user,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    /// Remove a user from a private pool's whitelist. Only callable by pool creator.
    pub fn remove_from_whitelist(
        env: Env,
        creator: Address,
        pool_id: u64,
        user: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        creator.require_auth();

        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        if pool.creator != creator {
            return Err(PredifiError::Unauthorized);
        }

        assert!(pool.private, "Pool is not private");

        let whitelist_key = DataKey::Whitelist(pool_id, user.clone());
        env.storage().persistent().remove(&whitelist_key);

        RemovedFromWhitelistEvent {
            pool_id,
            user,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    /// Check whether a user has an explicit whitelist entry for a pool.
    ///
    /// This helper only reports stored whitelist membership. It does not treat
    /// public pools, pool creators, or invite-based access as implicit
    /// whitelist membership.
    pub fn is_whitelisted(env: Env, pool_id: u64, user: Address) -> bool {
        let whitelist_key = DataKey::Whitelist(pool_id, user);
        let is_whitelisted = env
            .storage()
            .persistent()
            .get(&whitelist_key)
            .unwrap_or(false);
        if env.storage().persistent().has(&whitelist_key) {
            Self::extend_persistent(&env, &whitelist_key);
        }
        is_whitelisted
    }

    /// Return the number of unique participants in a pool.
    ///
    /// A participant is any address that has placed at least one prediction.
    /// Subsequent top-ups by the same address do not increase the count.
    ///
    /// # Arguments
    /// * `pool_id` - The unique identifier of the pool.
    ///
    /// # Returns
    /// The number of unique participants as a `u32`.
    pub fn get_pool_participants_count(env: Env, pool_id: u64) -> u32 {
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);
        pool.participants_count
    }

    /// Get comprehensive stats for a pool.
    pub fn get_pool_stats(env: Env, pool_id: u64) -> PoolStats {
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        let stakes = Self::get_outcome_stakes(&env, pool_id, pool.options_count);

        let pc_key = DataKey::PartCnt(pool_id);
        let participants_count: u32 = env.storage().persistent().get(&pc_key).unwrap_or(0);
        if env.storage().persistent().has(&pc_key) {
            Self::extend_persistent(&env, &pc_key);
        }

        let mut current_odds = Vec::new(&env);
        for stake in stakes.iter() {
            if stake == 0 {
                current_odds.push_back(0);
            } else {
                // Calculation: (total_stake * 10000) / stake
                // Result is fixed-point with 4 decimal places (e.g., 2.5x odds = 25000)
                let odds = pool
                    .total_stake
                    .checked_mul(10000)
                    .expect("overflow")
                    .checked_div(stake)
                    .unwrap_or(0);
                current_odds.push_back(odds as u64);
            }
        }

        PoolStats {
            pool_id,
            total_stake: pool.total_stake,
            stakes_per_outcome: stakes,
            participants_count,
            current_odds,
        }
    }

    /// Set a price-based condition for automated pool resolution.
    /// Only callable by Operator (role 1).
    pub fn set_price_condition(
        env: Env,
        operator: Address,
        pool_id: u64,
        feed_pair: Symbol,
        target_price: i128,
        operator_type: u32,
        tolerance_bps: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        operator.require_auth();
        Self::require_role(&env, &operator, 1)?; // Role Operator

        let pool_key = DataKey::Pool(pool_id);
        if !env.storage().persistent().has(&pool_key) {
            return Err(PredifiError::PoolNotFound);
        }

        let condition_key = DataKey::PriceCondition(pool_id);
        env.storage().persistent().set(
            &condition_key,
            &(feed_pair, target_price, operator_type, tolerance_bps),
        );
        Self::extend_persistent(&env, &condition_key);
        Ok(())
    }

    /// Update price feed data from an external oracle.
    /// Only callable by authorized oracles.
    pub fn update_price_feed(
        env: Env,
        oracle: Address,
        feed_pair: Symbol,
        price: i128,
        confidence: i128,
        timestamp: u64,
        expires_at: u64,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        oracle.require_auth();

        let feed_key = DataKey::PriceFeed(feed_pair.clone());
        env.storage()
            .persistent()
            .set(&feed_key, &(price, confidence, timestamp, expires_at));
        Self::extend_persistent(&env, &feed_key);

        // Track feed pair for cleanup
        let mut list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&DataKey::PriceFeedList)
            .unwrap_or_else(|| Vec::new(&env));
        if !list.contains(feed_pair.clone()) {
            list.push_back(feed_pair);
            env.storage()
                .persistent()
                .set(&DataKey::PriceFeedList, &list);
        }

        Ok(())
    }

    /// Remove all expired price feeds from storage. Permissionless.
    ///
    /// Returns the number of feeds removed.
    pub fn cleanup_expired_feeds(env: Env) -> u32 {
        let current_time = env.ledger().timestamp();

        let list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&DataKey::PriceFeedList)
            .unwrap_or_else(|| Vec::new(&env));

        let mut remaining: Vec<Symbol> = Vec::new(&env);
        let mut removed: u32 = 0;

        for i in 0..list.len() {
            let pair = list.get(i).unwrap();
            let expired = env
                .storage()
                .persistent()
                .get::<DataKey, (i128, i128, u64, u64)>(&DataKey::PriceFeed(pair.clone()))
                .map(|(_, _, _, expires_at)| expires_at < current_time)
                .unwrap_or(true);

            if expired {
                env.storage()
                    .persistent()
                    .remove(&DataKey::PriceFeed(pair));
                removed += 1;
            } else {
                remaining.push_back(pair);
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::PriceFeedList, &remaining);

        removed
    }

    /// Automatically resolve a pool based on its configured price condition.
    /// Anyone can trigger this once the pool's end time and resolution delay have passed.
    pub fn resolve_pool_from_price(env: Env, pool_id: u64) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);

        let condition_key = DataKey::PriceCondition(pool_id);
        let (feed_pair, target_price, op, _tolerance): (Symbol, i128, u32, u32) = env
            .storage()
            .persistent()
            .get(&condition_key)
            .expect("Condition not found");

        let feed_key = DataKey::PriceFeed(feed_pair);
        let (price, _conf, _ts, expires_at): (i128, i128, u64, u64) = env
            .storage()
            .persistent()
            .get(&feed_key)
            .expect("Feed not found");

        if env.ledger().timestamp() > expires_at {
            return Err(PredifiError::InvalidPoolState);
        }

        // Logic matched to price_feed_integration_test.rs:
        // ComparisonOp: 0=LT, 1=GT
        // Outcome: 0=No, 1=Yes
        let outcome = if op == 1 {
            if price > target_price {
                1
            } else {
                0
            }
        } else if price < target_price {
            0
        } else {
            1
        };

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        if pool.state != MarketState::Active {
            return Err(PredifiError::InvalidPoolState);
        }

        let current_time = env.ledger().timestamp();
        let config = Self::get_config(&env);

        if current_time < pool.end_time.saturating_add(config.resolution_delay) {
            return Err(PredifiError::ResolutionDelayNotMet);
        }

        // Apply resolution
        pool.state = MarketState::Resolved;
        pool.outcome = outcome;
        pool.fee_bps = Self::calculate_dynamic_fee(&env, &pool);

        env.storage().persistent().set(&pool_key, &pool);
        Self::bump_ttl(&env, &pool_key);

        PoolResolvedEvent {
            pool_id,
            operator: env.current_contract_address(), // System resolved
            outcome,
        }
        .publish(&env);

        Ok(())
    }
    pub fn set_fee_tiers(
        env: Env,
        admin: Address,
        tiers: Vec<FeeTier>,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_fee_tiers")?;

        for i in 0..tiers.len() {
            if let Some(tier) = tiers.get(i) {
                if tier.fee_bps > 10_000 {
                    return Err(PredifiError::InvalidFeeBps);
                }
            }
        }

        env.storage().persistent().set(&DataKey::FeeTiers, &tiers);
        Self::bump_ttl(&env, &DataKey::FeeTiers);

        FeeTiersUpdateEvent {
            admin,
            tiers_count: tiers.len(),
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_fee_tiers(env: Env) -> Vec<FeeTier> {
        env.storage()
            .persistent()
            .get(&DataKey::FeeTiers)
            .unwrap_or_else(|| Vec::new(&env))
    }

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

#[contractimpl]
impl OracleCallback for PredifiContract {
    fn oracle_resolve(
        env: Env,
        oracle: Address,
        pool_id: u64,
        outcome: u32,
        proof: String,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        oracle.require_auth();

        Self::require_oracle_role_for_resolution(&env, &oracle, pool_id)?;

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        // if pool.state != MarketState::Active {
        //     return Err(PredifiError::InvalidPoolState);
        // }
        if !Self::is_pool_active(&pool) {
            return Err(PredifiError::InvalidPoolState);
        }

        let current_time = env.ledger().timestamp();
        let config = Self::get_config(&env);

        if current_time < pool.end_time.saturating_add(config.resolution_delay) {
            return Err(PredifiError::ResolutionDelayNotMet);
        }

        // Validate: outcome must be within the valid options range
        if outcome >= pool.options_count {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidOutcome);
        }

        // --- Multi-oracle Voting Logic ---

        let vote_key = DataKey::ResVote(pool_id, oracle.clone());
        if env.storage().persistent().has(&vote_key) {
            return Err(PredifiError::OracleAlreadyVoted);
        }

        // Record the oracle's vote
        env.storage().persistent().set(&vote_key, &outcome);
        Self::extend_persistent(&env, &vote_key);

        // Increment total number of votes cast for this pool
        let total_votes_key = DataKey::ResTotal(pool_id);
        let total_votes: u32 = env
            .storage()
            .persistent()
            .get(&total_votes_key)
            .unwrap_or(0);
        let new_total_votes = total_votes + 1;
        env.storage()
            .persistent()
            .set(&total_votes_key, &new_total_votes);
        Self::extend_persistent(&env, &total_votes_key);

        // Increment specific outcome vote count
        let outcome_votes_key = DataKey::ResVoteCt(pool_id, outcome);
        let outcome_votes: u32 = env
            .storage()
            .persistent()
            .get(&outcome_votes_key)
            .unwrap_or(0);
        let new_outcome_votes = outcome_votes + 1;
        env.storage()
            .persistent()
            .set(&outcome_votes_key, &new_outcome_votes);
        Self::extend_persistent(&env, &outcome_votes_key);

        // Detect conflicts: if there are ANY votes for a different outcome
        if new_total_votes > new_outcome_votes {
            // A conflict exists. Find at least one other voted outcome for the event.
            for i in 0..pool.options_count {
                if i == outcome {
                    continue;
                }
                let other_key = DataKey::ResVoteCt(pool_id, i);
                if env.storage().persistent().has(&other_key) {
                    ResolutionConflictEvent {
                        pool_id,
                        oracle: oracle.clone(),
                        outcome,
                        existing_outcome: i,
                    }
                    .publish(&env);
                    break;
                }
            }
        }

        OracleResolvedEvent {
            pool_id,
            oracle: oracle.clone(),
            outcome,
            proof,
        }
        .publish(&env);

        // Check if the required threshold has been met
        if new_outcome_votes >= pool.required_resolutions {
            pool.state = MarketState::Resolved;
            pool.outcome = outcome;
            pool.fee_bps = Self::calculate_dynamic_fee(&env, &pool);

            env.storage().persistent().set(&pool_key, &pool);
            Self::bump_ttl(&env, &pool_key);

            Self::extend_persistent(&env, &pool_key);
            // Remove from global active index now that the pool is resolved.
            Self::remove_from_active_index(&env, pool_id);

            // Retrieve winning-outcome stake for the diagnostic event efficiently
            let winning_stake = Self::get_outcome_stake(env.clone(), pool_id, outcome);

            // Emit resolution events once threshold is met
            PoolResolvedEvent {
                pool_id,
                operator: oracle,
                outcome,
            }
            .publish(&env);

            PoolResolvedDiagEvent {
                pool_id,
                outcome,
                total_stake: pool.total_stake,
                winning_stake,
                timestamp: env.ledger().timestamp(),
            }
            .publish(&env);
        }

        Ok(())
    }
}

mod fee_tiers_test;
mod integration_test;
mod test;
