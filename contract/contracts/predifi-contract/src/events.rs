#![allow(dead_code)]
use soroban_sdk::{contractevent, Address, BytesN, String, Symbol, Vec};

// ── Events ───────────────────────────────────────────────────────────────────

#[contractevent(topics = ["init"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitEvent {
    pub access_control: Address,
    pub treasury: Address,
    pub fee_bps: u32,
    pub resolution_delay: u64,
    pub min_pool_duration: u64,
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
    pub min_total_stake: i128,
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

#[contractevent(topics = ["EmergencyWithdraw"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyWithdrawEvent {
    pub admin: Address,
    pub token: Address,
    pub destination: Address,
    pub amount: i128,
}
