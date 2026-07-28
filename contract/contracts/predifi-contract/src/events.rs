#![allow(dead_code)]

//! # Contract Events Module (`events.rs`)
//!
//! This module defines all strongly-typed event structures emitted by the PrediFi contract.
//! Events are published on-chain via Soroban's event system and serve as the primary
//! real-time data stream for off-chain indexers, user interfaces, analytics engines,
//! and security monitoring services.
//!
//! ## Event Architecture & Categorization
//!
//! PrediFi events are grouped into four main categories:
//! 1. **Administrative & Governance Events**: Track governance actions, contract pausing, parameter tweaks, whitelist updates, and WASM upgrades.
//! 2. **Market & Lifecycle Events**: Track prediction market creation, liquidity provision, prediction placement, resolution, and cancellation.
//! 3. **Financial & Claim Events**: Track payout claims (winnings, refunds, referral bonuses) and treasury withdrawals.
//! 4. **Monitoring & Alert Events**: High-priority events designed for SIEM, Horizon streaming, and Grafana alerting systems (e.g. unauthorized attempts, double-claim probes, high-value predictions).
//!
//! ## Subscribing to Events Off-Chain
//!
//! Off-chain services (such as Horizon event indexers or Soroban RPC listeners) can subscribe
//! to events by filtering on topics:
//! - **Topic 0**: Represents the event type name (e.g., `Symbol::new(env, "prediction_placed")`).
//! - **Topic 1..N**: Additional indexed filter topics such as `pool_id` or `user` address when defined.
//!
//! ## Indexing & Performance Implications
//!
//! - Event payloads use standard Soroban data structures (`Address`, `Symbol`, `String`, `Vec`).
//! - Storage costs for event emission are borne by the transaction caller; event payloads are kept compact while remaining self-describing.
//! - Indexers must process events sequentially according to ledger sequence and transaction order.

use soroban_sdk::{contractevent, contracttype, Address, BytesN, String, Symbol, Vec};

// ── Administrative & Governance Events ───────────────────────────────────────

/// Emitted when the contract is first initialized with core configuration parameters.
///
/// **When Emitted**: During the execution of `initialize`.
/// **Indexing Implications**: Used by indexers to capture global initial contract settings.
#[contractevent(topics = ["init"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitEvent {
    /// Address of the access control or admin contract.
    pub access_control: Address,
    /// Address of the treasury receiving protocol fees.
    pub treasury: Address,
    /// Default protocol fee in basis points (1 bps = 0.01%).
    pub fee_bps: u32,
    /// Delay in seconds after pool end time before resolution can occur.
    pub resolution_delay: u64,
    /// Minimum required pool duration in seconds.
    pub min_pool_duration: u64,
}

/// Emitted when contract functionality is paused by an authorized administrator.
///
/// **When Emitted**: Inside `pause`.
/// **Indexing Implications**: Frontends should disable state-mutating actions (staking, claims).
#[contractevent(topics = ["pause"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseEvent {
    /// Address of the admin who initiated the pause.
    pub admin: Address,
}

/// Emitted when contract operations are resumed by an admin.
///
/// **When Emitted**: Inside `unpause`.
/// **Indexing Implications**: Frontends re-enable prediction placement and claims.
#[contractevent(topics = ["unpause"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnpauseEvent {
    /// Address of the admin who unpaused the contract.
    pub admin: Address,
}

/// Emitted when global protocol fee in basis points is updated.
///
/// **When Emitted**: Inside `set_fee_bps` or `apply_fee_bps`.
/// **Indexing Implications**: Used by analytics to compute fee structure changes over time.
#[contractevent(topics = ["fee_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeUpdateEvent {
    /// Admin address authorizing the fee update.
    pub admin: Address,
    /// New fee rate in basis points (e.g., 250 = 2.5%).
    pub fee_bps: u32,
}

/// Emitted when tiered fee schedules are modified.
///
/// **When Emitted**: Inside `set_fee_tiers`.
/// **Indexing Implications**: Allows off-chain fee calculators to update cached tier maps.
#[contractevent(topics = ["fee_tiers_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeTiersUpdateEvent {
    /// Admin address modifying fee tiers.
    pub admin: Address,
    /// Total number of fee tiers currently configured.
    pub tiers_count: u32,
}

/// Emitted when the protocol treasury recipient address is updated.
///
/// **When Emitted**: Inside `set_treasury`.
/// **Indexing Implications**: Updates off-chain treasury monitoring tools to track the new destination address.
#[contractevent(topics = ["treasury_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryUpdateEvent {
    /// Admin address updating the treasury destination.
    pub admin: Address,
    /// New treasury wallet address.
    pub treasury: Address,
}

/// Emitted when the global market resolution delay is updated.
///
/// **When Emitted**: Inside `set_resolution_delay`.
/// **Indexing Implications**: Influences expected resolution timeline estimates shown to users.
#[contractevent(topics = ["resolution_delay_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionDelayUpdateEvent {
    /// Admin address performing the update.
    pub admin: Address,
    /// New resolution delay in seconds.
    pub delay: u64,
}

/// Emitted when the user claim window duration is updated.
///
/// **When Emitted**: Inside `set_claim_window`.
/// **Indexing Implications**: Used to display claim expiration warnings on frontends.
#[contractevent(topics = ["claim_window_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimWindowUpdateEvent {
    /// Admin address updating the claim window.
    pub admin: Address,
    /// Duration of the claim window in seconds.
    pub claim_window_seconds: u64,
}

/// Emitted when the global minimum pool duration requirement is updated.
///
/// **When Emitted**: Inside `set_min_pool_duration`.
/// **Indexing Implications**: Form validation rules on pool creation UI should update accordingly.
#[contractevent(topics = ["min_pool_duration_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinPoolDurationUpdateEvent {
    /// Admin address setting the minimum duration.
    pub admin: Address,
    /// Minimum allowed pool duration in seconds.
    pub duration: u64,
}

/// Emitted when the global minimum stake requirement per prediction is updated.
///
/// **When Emitted**: Inside `set_min_stake`.
/// **Indexing Implications**: Frontends use this to set input field validation boundaries.
#[contractevent(topics = ["min_stake_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinStakeUpdateEvent {
    /// Admin address performing the update.
    pub admin: Address,
    /// Minimum required stake amount in native token units.
    pub min_stake: i128,
}

// ── Market Lifecycle Events ──────────────────────────────────────────────────

/// Emitted when a market pool becomes ready for oracle or operator resolution.
///
/// **When Emitted**: Inside `mark_pool_ready`.
/// **Indexing Implications**: Triggers automated keeper bots to initiate oracle resolution workflows.
#[contractevent(topics = ["pool_ready"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolReadyForResolutionEvent {
    /// Unique identifier of the market pool.
    pub pool_id: u64,
    /// Ledger timestamp when the pool became ready.
    pub timestamp: u64,
}

/// Emitted when a new prediction pool is created.
///
/// **When Emitted**: Inside `create_pool`.
/// **Indexing Implications**: Primary event for indexing new prediction markets in UI dashboards and search engines.
#[contractevent(topics = ["pool_created"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolCreatedEvent {
    /// Unique identifier of the created pool.
    pub pool_id: u64,
    /// Creator wallet address.
    pub creator: Address,
    /// Unix timestamp when betting closes for this pool.
    pub end_time: u64,
    /// Stellar Asset / Token address used for staking.
    pub token: Address,
    /// Number of distinct outcome options (e.g., 2 for binary, N for multi-choice).
    pub options_count: u32,
    /// Off-chain IPFS / HTTPS metadata link describing the market question and rules.
    pub metadata_url: String,
    /// Initial liquidity deposited by the creator.
    pub initial_liquidity: i128,
    /// Category classification tag (e.g. Sports, Crypto, Politics).
    pub category: Symbol,
    /// Number of oracle/operator resolutions required for consensus.
    pub required_resolutions: u32,
    /// Maximum total stake cap for this pool.
    pub max_total_stake: i128,
    /// Minimum total stake requirement for pool validity.
    pub min_total_stake: i128,
    /// Text labels for each available outcome option.
    pub outcome_descriptions: Vec<String>,
}

/// Emitted when initial seed liquidity is deposited into a newly created pool.
///
/// **When Emitted**: Inside `create_pool` or liquidity addition helpers.
/// **Indexing Implications**: Allows tracking market depth and initial creator backing.
#[contractevent(topics = ["initial_liquidity_provided"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialLiquidityProvidedEvent {
    /// Identifier of the pool receiving liquidity.
    pub pool_id: u64,
    /// Address of the liquidity provider / creator.
    pub creator: Address,
    /// Amount of initial liquidity deposited.
    pub amount: i128,
}

/// Emitted when a market pool is resolved by an operator.
///
/// **When Emitted**: Inside `resolve_pool`.
/// **Indexing Implications**: Notifies UI and winners that claims can now be processed for outcome.
#[contractevent(topics = ["pool_resolved"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolResolvedEvent {
    /// Unique identifier of the resolved pool.
    pub pool_id: u64,
    /// Address of the operator finalizing resolution.
    pub operator: Address,
    /// Winning outcome index (0-indexed).
    pub outcome: u32,
}

/// Emitted when an oracle submits a resolution outcome with proof.
///
/// **When Emitted**: Inside `resolve_pool_with_oracle`.
/// **Indexing Implications**: Provides cryptographic / off-chain proof linkage for auditability.
#[contractevent(topics = ["oracle_resolved"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleResolvedEvent {
    /// Identifier of the resolved pool.
    pub pool_id: u64,
    /// Oracle node address that provided the resolution result.
    pub oracle: Address,
    /// Decided winning outcome index.
    pub outcome: u32,
    /// String proof or URI pointing to oracle data validation payload.
    pub proof: String,
}

/// Emitted when a pool is canceled by an authorized party or due to emergency conditions.
///
/// **When Emitted**: Inside `cancel_pool` or `emergency_cancel_pool`.
/// **Indexing Implications**: Instructs users to claim full refunds rather than winnings.
#[contractevent(topics = ["pool_canceled"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolCanceledEvent {
    /// Identifier of the canceled pool.
    pub pool_id: u64,
    /// Address that initiated the cancellation.
    pub caller: Address,
    /// Human-readable reason for market cancellation.
    pub reason: String,
    /// Operator address confirming the cancellation action.
    pub operator: Address,
}

/// Emitted when per-pool minimum or maximum stake bounds are adjusted.
///
/// **When Emitted**: Inside `set_stake_limits`.
/// **Indexing Implications**: Used by betting interfaces to enforce current pool bet limits.
#[contractevent(topics = ["stake_limits_updated"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeLimitsUpdatedEvent {
    /// Identifier of the targeted pool.
    pub pool_id: u64,
    /// Operator authorizing the stake limit changes.
    pub operator: Address,
    /// Minimum allowed stake per user prediction.
    pub min_stake: i128,
    /// Maximum allowed stake per user prediction.
    pub max_stake: i128,
}

/// Emitted when a user places a prediction stake on a pool outcome.
///
/// **When Emitted**: Inside `place_prediction`.
/// **Indexing Implications**: Core telemetry event for updating live odds, market volumes, and user portfolios.
#[contractevent(topics = ["prediction_placed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionPlacedEvent {
    /// Identifier of the target pool.
    pub pool_id: u64,
    /// Address of the user placing the prediction.
    pub user: Address,
    /// Amount staked on the selected outcome.
    pub amount: i128,
    /// Chosen outcome index (0-indexed).
    pub outcome: u32,
}

// ── Financial & Payout Events ───────────────────────────────────────────────

/// Emitted when a winning user successfully claims their payout.
///
/// **When Emitted**: Inside `claim_winnings`.
/// **Indexing Implications**: Used for leaderboard rankings, transaction history, and payout auditing.
#[contractevent(topics = ["winnings_claimed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WinningsClaimedEvent {
    /// Identifier of the pool from which winnings were claimed.
    pub pool_id: u64,
    /// Address of the user receiving the payout.
    pub user: Address,
    /// Net winnings amount transferred to user.
    pub amount: i128,
}

/// Emitted when a referral reward bonus is paid out to a referrer.
///
/// **When Emitted**: Inside `claim_winnings` when a referral relationship exists.
/// **Indexing Implications**: Tracks referral commission statistics for affiliate programs.
#[contractevent(topics = ["referral_paid"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferralPaidEvent {
    /// Pool identifier where the winning prediction occurred.
    pub pool_id: u64,
    /// Wallet address of the referrer receiving the reward.
    pub referrer: Address,
    /// Wallet address of the referred user who generated the fee.
    pub referred_user: Address,
    /// Referral reward amount transferred.
    pub amount: i128,
}

// ── Monitoring & Alert Events ─────────────────────────────────────────────────
// These events are classified by severity and are intended for consumption by
// off-chain monitoring tools (Horizon event streaming, Grafana, SIEM, etc.).
// See MONITORING.md at the repo root for scraping patterns and alert rules.

/// 🔴 HIGH ALERT — emitted when `resolve_pool` is called by an address that
/// does not hold the Operator role. Indicates a potential attack or
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
/// already been claimed for the same (user, pool) pair. Repeated attempts may
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
/// successfully paused. Having a dedicated alert topic makes it easy to set
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
/// meets or exceeds `HIGH_VALUE_THRESHOLD`. Useful for liquidity monitoring
/// and detecting unusual betting patterns.
#[contractevent(topics = ["high_value_prediction"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighValuePredictionEvent {
    /// Identifier of the pool receiving the large stake.
    pub pool_id: u64,
    /// User address placing the large prediction.
    pub user: Address,
    /// Staked amount that triggered the threshold.
    pub amount: i128,
    /// Outcome index chosen.
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
    /// Identifier of the resolved pool.
    pub pool_id: u64,
    /// Winning outcome index.
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
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeStakesUpdatedEvent {
    /// Identifier of the pool updated.
    pub pool_id: u64,
    /// Number of outcomes in this pool.
    pub options_count: u32,
    /// Total stake across all outcomes after the update.
    pub total_stake: i128,
}

/// Emitted when a token is added to the allowed collateral whitelist.
///
/// **When Emitted**: Inside `add_token_to_whitelist`.
/// **Indexing Implications**: Frontends use this to render valid payment token selectors.
#[contractevent(topics = ["token_whitelist_added"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenWhitelistAddedEvent {
    /// Admin address adding the token.
    pub admin: Address,
    /// Token contract address added.
    pub token: Address,
}

/// Emitted when a token is removed from the collateral whitelist.
///
/// **When Emitted**: Inside `remove_token_from_whitelist`.
/// **Indexing Implications**: Restricts users from selecting this token for new pools.
#[contractevent(topics = ["token_whitelist_removed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenWhitelistRemovedEvent {
    /// Admin address removing the token.
    pub admin: Address,
    /// Token contract address removed.
    pub token: Address,
}

/// Emitted when protocol fees are withdrawn from treasury storage.
///
/// **When Emitted**: Inside `withdraw_treasury`.
/// **Indexing Implications**: Financial accounting tools track protocol revenue movements.
#[contractevent(topics = ["treasury_withdrawn"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryWithdrawnEvent {
    /// Admin authorizing the withdrawal.
    pub admin: Address,
    /// Token asset withdrawn.
    pub token: Address,
    /// Amount withdrawn.
    pub amount: i128,
    /// Recipient address receiving funds.
    pub recipient: Address,
    /// Remaining treasury balance post-withdrawal.
    pub remaining_balance: i128,
    /// Ledger timestamp.
    pub timestamp: u64,
}

/// Emitted when a user claims a refund from a canceled pool.
///
/// **When Emitted**: Inside `claim_refund`.
/// **Indexing Implications**: Auditing refund disbursements for canceled markets.
#[contractevent(topics = ["refund_claimed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundClaimedEvent {
    /// Identifier of the canceled pool.
    pub pool_id: u64,
    /// Address of user receiving refund.
    pub user: Address,
    /// Refunded stake amount returned to user.
    pub amount: i128,
}

/// Emitted when a proposal to upgrade contract WASM code is published.
///
/// **When Emitted**: Inside `commit_upgrade`.
/// **Indexing Implications**: Alerts governance and security monitoring of pending code changes.
#[contractevent(topics = ["upgrade"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeEvent {
    /// Admin initiating code upgrade proposal.
    pub admin: Address,
    /// 32-byte cryptographic hash of proposed WASM binary.
    pub new_wasm_hash: BytesN<32>,
}

/// Emitted when contract code is successfully upgraded on-chain.
///
/// **When Emitted**: Inside `apply_upgrade`.
/// **Indexing Implications**: Informs indexers of contract version bump.
#[contractevent(topics = ["contract_upgraded"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUpgradedEvent {
    /// Previous version integer.
    pub old_version: u32,
    /// New version integer.
    pub new_version: u32,
    /// Address that executed the upgrade.
    pub upgraded_by: Address,
}

/// Emitted when Pyth / Price Feed Oracle integration is initialized.
///
/// **When Emitted**: Inside `init_oracle`.
/// **Indexing Implications**: Off-chain price feed aggregators verify Pyth connection setup.
#[contractevent(topics = ["oracle_init"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleInitEvent {
    /// Admin address initializing oracle parameters.
    pub admin: Address,
    /// Address of external Pyth oracle contract.
    pub pyth_contract: Address,
    /// Maximum allowed staleness of price updates in seconds.
    pub max_price_age: u64,
    /// Minimum required confidence ratio in basis points.
    pub min_confidence_ratio: u32,
}

/// Emitted when an updated price observation is received from an oracle feed.
///
/// **When Emitted**: Inside `update_price_feed`.
/// **Indexing Implications**: Logs external price data feeds for auditing financial market resolution.
#[contractevent(topics = ["price_feed_updated"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceFeedUpdatedEvent {
    /// Address of oracle submitting price update.
    pub oracle: Address,
    /// Asset pair symbol (e.g. BTC/USD).
    pub feed_pair: Symbol,
    /// Observed price value formatted with fixed precision.
    pub price: i128,
    /// Confidence interval margin provided by Pyth.
    pub confidence: i128,
    /// Observation timestamp.
    pub timestamp: u64,
    /// Expiration timestamp of this price data point.
    pub expires_at: u64,
}

/// Emitted when automated price target condition is configured for a market pool.
///
/// **When Emitted**: Inside `set_price_condition`.
/// **Indexing Implications**: Automated resolution bots monitor price conditions against live price feeds.
#[contractevent(topics = ["price_condition_set"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceConditionSetEvent {
    /// Identifier of target pool.
    pub pool_id: u64,
    /// Target asset pair symbol.
    pub feed_pair: Symbol,
    /// Target price threshold required to trigger resolution.
    pub target_price: i128,
    /// Comparison operator flag (e.g. 0: GTE, 1: LTE, 2: EQ).
    pub operator: u32,
    /// Allowed price tolerance in basis points.
    pub tolerance_bps: u32,
}

/// Emitted when a market pool is resolved via automated price feed condition evaluation.
///
/// **When Emitted**: Inside `resolve_by_price`.
/// **Indexing Implications**: Connects price feed data point directly to final market resolution outcome.
#[contractevent(topics = ["price_resolved"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceResolvedEvent {
    /// Identifier of target pool.
    pub pool_id: u64,
    /// Asset pair symbol evaluated.
    pub feed_pair: Symbol,
    /// Actual price at resolution time.
    pub current_price: i128,
    /// Target price defined for market condition.
    pub target_price: i128,
    /// Determined winning outcome index.
    pub outcome: u32,
}

/// 🔴 HIGH ALERT — Emitted when multiple oracle reports conflict on market outcome.
///
/// **When Emitted**: Inside multi-oracle resolution when conflicting outcome is reported.
/// **Indexing Implications**: Triggers immediate operator dispute review.
#[contractevent(topics = ["resolution_conflict"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionConflictEvent {
    /// Identifier of target pool.
    pub pool_id: u64,
    /// Reporting oracle address.
    pub oracle: Address,
    /// Outcome reported by this oracle.
    pub outcome: u32,
    /// Previously reported outcome on record.
    pub existing_outcome: u32,
}

/// Emitted when a user address is added to a private pool's allowed participant whitelist.
///
/// **When Emitted**: Inside `add_to_whitelist`.
/// **Indexing Implications**: Private market dashboards update access control eligibility.
#[contractevent(topics = ["added_to_whitelist"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddedToWhitelistEvent {
    /// Target pool identifier.
    pub pool_id: u64,
    /// User address added to whitelist.
    pub user: Address,
    /// Address of admin/creator who added the user.
    pub added_by: Address,
    /// Ledger timestamp.
    pub timestamp: u64,
}

/// Emitted when a user address is removed from a private pool's whitelist.
///
/// **When Emitted**: Inside `remove_from_whitelist`.
/// **Indexing Implications**: Revokes user betting access on private pool.
#[contractevent(topics = ["removed_from_whitelist"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemovedFromWhitelistEvent {
    /// Target pool identifier.
    pub pool_id: u64,
    /// User address removed from whitelist.
    pub user: Address,
    /// Address of admin/creator who removed the user.
    pub removed_by: Address,
    /// Ledger timestamp.
    pub timestamp: u64,
}
