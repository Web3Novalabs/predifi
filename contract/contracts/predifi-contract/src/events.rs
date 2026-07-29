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
/// This event is emitted exactly once during the contract lifecycle when the `initialize`
/// function is called. It captures the immutable and mutable governance parameters that
/// define the protocol's initial operating state.
///
/// # When Emitted
/// During the execution of `initialize` by the contract deployer. This is a one-time event
/// that occurs at contract deployment.
///
/// # Event Fields
/// - `access_control` - The address of the external access control contract that manages
///   role-based permissions (Admin, Operator, Oracle roles). This contract is immutable
///   after initialization.
/// - `treasury` - The address that receives all protocol fees from pool creation and
///   resolution. Can be updated later via `set_treasury`.
/// - `fee_bps` - The default protocol fee rate in basis points (1 bp = 0.01%). For example,
///   250 bps = 2.5%. This fee is charged on pool creation and can be updated via `set_fee_bps`.
/// - `resolution_delay` - The mandatory delay period in seconds after a pool's `end_time`
///   before it can be resolved. This prevents premature resolution and allows for dispute
///   windows. Can be updated via `set_resolution_delay`.
/// - `min_pool_duration` - The minimum allowed duration for any prediction pool in seconds.
///   Pools with shorter durations cannot be created. Can be updated via `set_min_pool_duration`.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=init
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["init"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **State Initialization**: Indexers should use this event to initialize their local
///   protocol state with the initial governance parameters.
/// - **Version Tracking**: This event marks the contract's genesis and can be used to
///   track contract deployment versions across network upgrades.
/// - **Treasury Monitoring**: Treasury monitoring services should subscribe to this event
///   to establish the initial fee recipient address.
/// - **Historical Analysis**: Analytics engines use this event to understand protocol
///   parameter evolution when combined with subsequent parameter update events.
///
/// # Payload Size
/// Approximately 80-100 bytes depending on address encoding. Compact due to primitive types.
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
/// This event is emitted when an authorized admin calls the `pause` function, which
/// disables all state-mutating operations including pool creation, prediction placement,
/// and claims. Read-only operations remain available.
///
/// # When Emitted
/// Inside `pause` when an admin with the PAUSE_ROLE invokes the function. Can be called
/// during emergencies, security incidents, or planned maintenance.
///
/// # Event Fields
/// - `admin` - The address of the administrator who initiated the pause. This address
///   must hold the PAUSE_ROLE in the access control contract.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=pause
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["pause"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Frontend State**: Frontend applications should immediately disable all state-mutating
///   UI components (create pool, place prediction, claim buttons) upon receiving this event.
/// - **Keeper Bots**: Automated keeper bots should pause all background tasks (oracle updates,
///   resolution attempts) when the contract is paused.
/// - **Security Monitoring**: Security teams should investigate the cause of any unexpected
///   pause events, as they may indicate a security incident.
/// - **User Notifications**: Notification services should alert active users that the protocol
///   is paused and their pending actions cannot be completed.
///
/// # Payload Size
/// Approximately 35-45 bytes (single address). Very compact for efficient monitoring.
#[contractevent(topics = ["pause"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseEvent {
    /// Address of the admin who initiated the pause.
    pub admin: Address,
}

/// Emitted when contract operations are resumed by an admin.
///
/// This event is emitted when an authorized admin calls the `unpause` function, which
/// re-enables all state-mutating operations that were disabled during the paused state.
/// Normal protocol operation resumes immediately after this event.
///
/// # When Emitted
/// Inside `unpause` when an admin with the PAUSE_ROLE invokes the function. Typically
/// called after emergency resolution, security fixes, or planned maintenance completion.
///
/// # Event Fields
/// - `admin` - The address of the administrator who unpaused the contract. This address
///   must hold the PAUSE_ROLE in the access control contract.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=unpause
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["unpause"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Frontend State**: Frontend applications should re-enable all disabled UI components
///   and display a "protocol operational" status message.
/// - **Keeper Bots**: Automated keeper bots should resume all background tasks including
///   oracle updates and resolution attempts.
/// - **User Notifications**: Notification services should alert users that normal operations
///   have resumed and they can complete pending actions.
/// - **Uptime Tracking**: Analytics services use pause/unpause event pairs to calculate
///   protocol uptime and downtime metrics.
///
/// # Payload Size
/// Approximately 35-45 bytes (single address). Very compact for efficient monitoring.
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
/// This is the primary event for tracking new prediction markets. It contains all
/// essential information about the pool including its configuration, liquidity,
/// outcome structure, and metadata reference. This event is critical for market
/// discovery and indexing.
///
/// # When Emitted
/// Inside `create_pool` when a user successfully creates a new prediction market.
/// This event is emitted only after all validations pass and the pool is stored.
///
/// # Event Fields
/// - `pool_id` - The unique identifier for this pool, monotonically increasing.
///   Used as the primary key for all pool-related operations and queries.
/// - `creator` - The address of the user who created the pool and provided initial
///   liquidity. This address may have special permissions for pool management.
/// - `end_time` - Unix timestamp when the pool's betting period closes. No predictions
///   can be placed after this time. Resolution can occur after `end_time + resolution_delay`.
/// - `token` - The Stellar Asset contract address used for staking in this pool.
///   All stakes, liquidity, and payouts use this token.
/// - `options_count` - The number of distinct outcome options. For binary markets this is 2
///   (Yes/No), for multi-choice markets it can be any value > 2.
/// - `metadata_url` - An off-chain URI (IPFS or HTTPS) pointing to detailed market
///   information including the question, resolution criteria, and source references.
/// - `initial_liquidity` - The amount of tokens the creator deposited as seed liquidity.
///   This ensures the pool has sufficient depth for early participants.
/// - `category` - A classification tag for grouping markets (e.g., Sports, Crypto, Politics).
///   Used for filtering and categorization in UIs.
/// - `required_resolutions` - The number of oracle/operator confirmations required for
///   consensus resolution. Higher values provide more security but slower resolution.
/// - `max_total_stake` - The maximum total stake allowed across all outcomes. Prevents
///   excessive exposure on any single market.
/// - `min_total_stake` - The minimum total stake required for the pool to be valid.
///   Pools below this threshold may be canceled.
/// - `outcome_descriptions` - A vector of human-readable labels for each outcome option.
///   The length must equal `options_count`. Used for UI display.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=pool_created
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["pool_created"]]
///     }
///   }
/// }
///
/// Filter by creator:
/// {
///   "topics": [["pool_created"], "{creator_address}"]]
/// }
///
/// Filter by token:
/// {
///   "topics": [["pool_created"], "{token_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Market Discovery**: This is the primary event for indexing new markets in UI
///   dashboards, search engines, and analytics platforms.
/// - **Metadata Fetching**: Indexers should fetch the `metadata_url` content to enrich
///   their database with market details, resolution criteria, and source references.
/// - **Category Indexing**: The `category` field should be indexed to enable filtering
///   and faceted search in user interfaces.
/// - **Timeline Tracking**: The `end_time` field should be indexed to support time-based
///   queries (e.g., "markets ending in the next 24 hours").
/// - **Token Filtering**: The `token` field enables filtering markets by staking asset,
///   useful for users who only want to bet with specific tokens.
/// - **Creator Analytics**: The `creator` field enables tracking creator performance,
///   success rates, and reputation scoring.
/// - **Liquidity Monitoring**: The `initial_liquidity` field helps identify well-funded
///   markets vs. those with minimal liquidity.
///
/// # Payload Size
/// Approximately 200-400 bytes depending on the length of `metadata_url` and
/// `outcome_descriptions`. Larger for markets with many outcomes or long metadata URLs.
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
/// This event marks the final resolution of a prediction market, determining the
/// winning outcome and enabling users to claim their winnings. It is the critical
/// event that transitions a pool from "active" to "resolved" state.
///
/// # When Emitted
/// Inside `resolve_pool` when an authorized operator submits the final resolution
/// for a pool. This occurs after the pool's `end_time + resolution_delay` has passed
/// and the operator has determined the correct outcome.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the resolved pool. Used to correlate
///   resolution with the pool's creation and prediction events.
/// - `operator` - The address of the operator who finalized the resolution. This
///   address must hold the OPERATOR_ROLE in the access control contract.
/// - `outcome` - The winning outcome index (0-indexed). All predictions on this
///   outcome are winners, all others are losers. Must be less than `options_count`.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=pool_resolved
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["pool_resolved"]]
///     }
///   }
/// }
///
/// Filter by pool:
/// {
///   "topics": [["pool_resolved"], "{pool_id}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Claim Window Start**: This event signals that the claim window has opened.
///   Indexers should update pool status to "resolved" and enable claim buttons in UIs.
/// - **Winner Notification**: Notification services should alert users who placed
///   winning predictions that they can now claim their winnings.
/// - **Payout Calculation**: Indexers should calculate potential payouts for all
///   winning predictions based on the final outcome stakes.
/// - **Operator Analytics**: The `operator` field enables tracking operator performance,
///   resolution accuracy, and speed.
/// - **Market History**: This event completes the market lifecycle in analytics databases.
/// - **Liquidity Release**: Indexers should track when liquidity is released back to
///   the creator or burned based on pool rules.
///
/// # Payload Size
/// Approximately 50-60 bytes. Compact for efficient resolution tracking.
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
/// This event is emitted when an oracle node provides a resolution for a pool,
/// typically as part of a multi-oracle consensus mechanism. It includes a proof
/// field that can be used to verify the oracle's data source and decision process.
///
/// # When Emitted
/// Inside `resolve_pool_with_oracle` when an authorized oracle submits a resolution
/// with supporting proof. This is part of the consensus-building process where
/// multiple oracles may submit outcomes before final resolution.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the pool being resolved by the oracle.
///   Used to correlate oracle submissions with the pool's lifecycle.
/// - `oracle` - The address of the oracle node submitting the resolution. This
///   address must hold the ORACLE_ROLE in the access control contract.
/// - `outcome` - The outcome index the oracle has determined to be correct.
///   Used in consensus calculation to determine the final winning outcome.
/// - `proof` - A string containing a proof or URI pointing to off-chain validation
///   data. This can be a cryptographic signature, IPFS hash of source data, or
///   HTTP URL to the oracle's data source for auditability.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=oracle_resolved
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["oracle_resolved"]]
///     }
///   }
/// }
///
/// Filter by oracle:
/// {
///   "topics": [["oracle_resolved"], "{oracle_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Consensus Tracking**: Indexers should aggregate oracle submissions to track
///   consensus progress and identify when sufficient confirmations are reached.
/// - **Audit Trail**: The `proof` field enables auditors to verify oracle decisions
///   by fetching and validating the referenced data sources.
/// - **Oracle Performance**: The `oracle` field enables tracking oracle accuracy,
///   response time, and reliability metrics.
/// - **Conflict Detection**: If multiple oracles submit different outcomes for the
///   same pool, indexers should flag this for operator review.
/// - **Transparency**: The proof linkage provides transparency into the resolution
///   process, enabling users to verify outcomes independently.
///
/// # Payload Size
/// Approximately 100-200 bytes depending on the length of the `proof` string.
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
/// This event is emitted when a pool is canceled before resolution, typically due to
/// invalid market conditions, source data issues, or emergency circumstances. All
/// users receive full refunds of their stakes instead of winnings.
///
/// # When Emitted
/// Inside `cancel_pool` or `emergency_cancel_pool` when an authorized party cancels
/// a pool. This can happen before or after the pool's `end_time`, but before final
/// resolution.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the canceled pool. Used to correlate
///   cancellation with the pool's creation and prediction events.
/// - `caller` - The address that initiated the cancellation. This could be the
///   creator, an admin, or an automated system depending on the cancellation path.
/// - `reason` - A human-readable explanation for why the pool was canceled.
///   This is displayed to users to provide transparency about the cancellation.
/// - `operator` - The address of the operator confirming the cancellation. This
///   address must hold the OPERATOR_ROLE and provides authorization for the action.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=pool_canceled
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["pool_canceled"]]
///     }
///   }
/// }
///
/// Filter by pool:
/// {
///   "topics": [["pool_canceled"], "{pool_id}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Refund Window Start**: This event signals that the refund window has opened.
///   Indexers should update pool status to "canceled" and enable refund buttons in UIs.
/// - **User Notification**: Notification services should alert all pool participants
///   that the market has been canceled and they can claim full refunds.
/// - **Refund Calculation**: Indexers should calculate refund amounts for all users
///   based on their total stakes (no fees are charged on refunds).
/// - **Cancellation Analytics**: The `reason` field enables tracking cancellation patterns
///   (e.g., invalid source data, market manipulation, technical issues).
/// - **Liquidity Return**: Indexers should track when the creator's initial liquidity
///   is returned after all refunds are processed.
/// - **Trust Metrics**: High cancellation rates may indicate protocol issues and
///   should be monitored for trust and reputation scoring.
///
/// # Payload Size
/// Approximately 150-250 bytes depending on the length of the `reason` string.
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
/// This is the core telemetry event for the protocol, emitted every time a user
/// places a prediction. It drives real-time updates for odds calculation, market
/// volume tracking, and user portfolio management.
///
/// # When Emitted
/// Inside `place_prediction` when a user successfully stakes tokens on a specific
/// outcome. This event is emitted after all validations pass (sufficient balance,
/// pool not ended, stake within limits, etc.).
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the pool receiving the prediction.
///   Used to correlate predictions with their parent market.
/// - `user` - The address of the user placing the prediction. This address will
///   receive winnings if the chosen outcome is correct.
/// - `amount` - The amount of tokens staked on this prediction. This amount is
///   locked until resolution or refund.
/// - `outcome` - The index of the chosen outcome (0-indexed). Must be less than
///   the pool's `options_count`. Used to determine if the prediction wins.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=prediction_placed
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["prediction_placed"]]
///     }
///   }
/// }
///
/// Filter by pool:
/// {
///   "topics": [["prediction_placed"], "{pool_id}"]]
/// }
///
/// Filter by user:
/// {
///   "topics": [["prediction_placed"], "{user_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Real-Time Odds**: Indexers should aggregate predictions by outcome to calculate
///   implied probabilities and update live odds displays.
/// - **Volume Tracking**: The `amount` field should be summed to track total market volume
///   and identify high-liquidity markets.
/// - **User Portfolios**: The `user` field enables building user prediction histories,
///   calculating potential payouts, and tracking win/loss ratios.
/// - **Market Activity**: High-frequency events indicate active markets. Indexers can
///   use event frequency to rank markets by activity level.
/// - **Outcome Distribution**: Aggregating by `outcome` reveals market sentiment and
///   can be displayed as outcome percentage bars in UIs.
/// - **Risk Management**: Large `amount` values may trigger risk alerts or require
///   additional liquidity monitoring.
///
/// # Payload Size
/// Approximately 60-80 bytes. Compact due to primitive types, making it suitable
/// for high-frequency emission without excessive gas costs.
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
/// This event is emitted when a user calls `claim_winnings` after a pool has been
/// resolved and their chosen outcome was correct. It represents the final financial
/// settlement of a prediction.
///
/// # When Emitted
/// Inside `claim_winnings` when a user successfully claims their winnings. This event
/// is emitted after the claim window has opened, the pool is resolved, the user has
/// a winning prediction, and they have not already claimed.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the resolved pool from which winnings are
///   being claimed. Used to correlate claims with their parent market.
/// - `user` - The address of the user receiving the payout. This address must have
///   placed a winning prediction on the pool.
/// - `amount` - The net winnings amount transferred to the user. This is calculated
///   as: `(stake * winning_outcome_total_stake) / winning_outcome_stake - stake - fees`.
///   Represents the profit after deducting the original stake and protocol fees.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=winnings_claimed
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["winnings_claimed"]]
///     }
///   }
/// }
///
/// Filter by user:
/// {
///   "topics": [["winnings_claimed"], "{user_address}"]]
/// }
///
/// Filter by pool:
/// {
///   "topics": [["winnings_claimed"], "{pool_id}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Leaderboard Rankings**: Aggregating `amount` by `user` enables building profit
///   leaderboards and tracking top performers.
/// - **Transaction History**: This event provides the complete claim history for
///   user transaction history pages and audit trails.
/// - **Payout Auditing**: Financial auditors use this event to verify that all winning
///   predictions were paid correctly and no funds were misappropriated.
/// - **User Analytics**: Tracking claim patterns helps identify user behavior (e.g.,
///   quick claimers vs. delayed claimers) and optimize claim reminders.
/// - **Revenue Tracking**: The difference between total stakes and total claims
///   (minus refunds) represents protocol revenue from fees.
/// - **Tax Reporting**: Users may use claim events for tax reporting purposes,
///   requiring accurate historical records.
///
/// # Payload Size
/// Approximately 50-60 bytes. Very compact for efficient financial tracking.
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
/// This event is emitted when a user claims winnings and a referral relationship exists.
/// The referrer receives a percentage of the protocol fees as a reward for bringing
/// the referred user to the platform.
///
/// # When Emitted
/// Inside `claim_winnings` when a user with a registered referrer successfully claims
/// winnings. The referral reward is calculated as a percentage of the protocol fees
/// from that claim and paid to the referrer.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the pool where the winning prediction occurred.
///   Used to correlate referral rewards with specific market activity.
/// - `referrer` - The address of the referrer receiving the reward. This address
///   must have been registered as the referrer for the `referred_user`.
/// - `referred_user` - The address of the user who placed the winning prediction and
///   generated the fees that fund the referral reward.
/// - `amount` - The referral reward amount transferred to the referrer. This is
///   typically a percentage (e.g., 10-20%) of the protocol fees from the claim.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=referral_paid
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["referral_paid"]]
///     }
///   }
/// }
///
/// Filter by referrer:
/// {
///   "topics": [["referral_paid"], "{referrer_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Affiliate Analytics**: Aggregating `amount` by `referrer` enables building
///   affiliate leaderboards and tracking top performers.
/// - **Referral Tracking**: The relationship between `referrer` and `referred_user`
///   enables building referral trees and tracking network growth.
/// - **Revenue Attribution**: This event helps attribute protocol revenue to specific
///   acquisition channels and calculate ROI on referral programs.
/// - **Fraud Detection**: Unusual referral patterns (e.g., circular referrals, high
///   churn rates) may indicate referral fraud and should be monitored.
/// - **Commission Payouts**: Indexers should track total referral payouts to ensure
///   they remain within expected percentage ranges of total fees.
/// - **User Growth**: New `referred_user` addresses indicate successful user acquisition
///   through referral programs.
///
/// # Payload Size
/// Approximately 80-100 bytes. Compact for efficient referral tracking.
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
///
/// This is a critical security event that should trigger immediate investigation.
/// It may indicate an attempted attack, access control misconfiguration, or
/// compromised credentials.
///
/// # When Emitted
/// Inside `resolve_pool` when a caller attempts to resolve a pool but does not
/// hold the required OPERATOR_ROLE. The resolution attempt is rejected, but this
/// event is emitted for security monitoring.
///
/// # Event Fields
/// - `caller` - The address that attempted the unauthorized resolution. This address
///   should be investigated for potential malicious intent or compromised keys.
/// - `pool_id` - The unique identifier of the pool that was targeted. Used to
///   understand the scope and motivation of the attack attempt.
/// - `timestamp` - The ledger timestamp when the attempt occurred. Used for
///   correlating with other security events and timeline analysis.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=unauthorized_resolution
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["unauthorized_resolution"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Security Alerting**: This event should trigger immediate PagerDuty/Slack alerts
///   to the security team for investigation.
/// - **Attack Pattern Detection**: Aggregating by `caller` can identify repeat offenders
///   and coordinated attack patterns.
/// - **Access Control Audit**: Frequent unauthorized attempts may indicate access
///   control misconfiguration that needs correction.
/// - **IP/Address Blocking**: Security systems may use the `caller` address to block
///   further attempts from the same source.
/// - **Incident Response**: This event should be logged to SIEM systems for forensic
///   analysis and compliance reporting.
///
/// # Payload Size
/// Approximately 60-70 bytes. Compact for efficient security monitoring.
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
///
/// This is a critical security event indicating an attempt to perform privileged
/// governance operations without authorization. It may indicate an attack attempt,
/// access control misconfiguration, or compromised admin credentials.
///
/// # When Emitted
/// Inside any admin-restricted function when a caller attempts the operation but
/// does not hold the required ADMIN_ROLE. The operation is rejected, but this event
/// is emitted for security monitoring.
///
/// # Event Fields
/// - `caller` - The address that attempted the unauthorized admin operation. This
///   address should be investigated for potential malicious intent.
/// - `operation` - A symbol identifying which operation was attempted (e.g.,
///   `set_fee_bps`, `set_treasury`, `pause`, `unpause`). Helps understand the
///   attacker's intent and target.
/// - `timestamp` - The ledger timestamp when the attempt occurred. Used for
///   correlating with other security events and timeline analysis.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=unauthorized_admin_op
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["unauthorized_admin_op"]]
///     }
///   }
/// }
///
/// Filter by operation:
/// {
///   "topics": [["unauthorized_admin_op"], "{operation_symbol}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Security Alerting**: This event should trigger immediate PagerDuty/Slack alerts
///   to the security team for investigation.
/// - **Privilege Escalation Detection**: Attempts to perform sensitive operations
///   indicate potential privilege escalation attacks.
/// - **Access Control Audit**: Frequent unauthorized attempts may indicate access
///   control misconfiguration that needs correction.
/// - **Operation-Specific Response**: The `operation` field enables different response
///   protocols based on the sensitivity of the targeted operation.
/// - **Compliance Logging**: This event should be logged to SIEM systems for forensic
///   analysis and regulatory compliance.
///
/// # Payload Size
/// Approximately 70-80 bytes. Compact for efficient security monitoring.
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
///
/// This event indicates suspicious behavior where a user is attempting to claim
/// winnings they have already received. While the claim is rejected, this pattern
/// may indicate a re-entrancy attack probe, frontend bug, or user confusion.
///
/// # When Emitted
/// Inside `claim_winnings` when a user attempts to claim winnings for a pool
/// they have already claimed from. The claim is rejected, but this event is
/// emitted for security monitoring.
///
/// # Event Fields
/// - `user` - The address that attempted the double-claim. This address should
///   be investigated for potential malicious intent or user experience issues.
/// - `pool_id` - The unique identifier of the pool for which the claim was already
///   made. Used to understand the context of the attempt.
/// - `timestamp` - The ledger timestamp when the attempt occurred. Used for
///   correlating with other events and timeline analysis.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=double_claim_attempt
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["double_claim_attempt"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Security Alerting**: This event should trigger alerts to the security team
///   for investigation, especially if patterns emerge.
/// - **Re-entrancy Detection**: Repeated double-claim attempts from the same user
///   may indicate a re-entrancy attack probe.
/// - **Frontend Bug Detection**: Widespread double-claim attempts may indicate
///   a frontend bug where claim buttons are not disabled after successful claims.
/// - **User Support**: Support teams should reach out to users with repeated
///   attempts to provide guidance and prevent frustration.
/// - **Pattern Analysis**: Aggregating by `user` and `pool_id` can identify systematic
///   abuse vs. isolated user confusion.
///
/// # Payload Size
/// Approximately 60-70 bytes. Compact for efficient security monitoring.
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
///
/// This is a dedicated alert event for monitoring systems. While `PauseEvent`
/// provides the operational data, this event is specifically designed for
/// high-priority alerting and incident response.
///
/// # When Emitted
/// Inside `pause` when an authorized admin successfully pauses the contract.
/// This event is emitted in addition to `PauseEvent` for dedicated alerting.
///
/// # Event Fields
/// - `admin` - The address of the admin who triggered the pause. Used to
///   identify who initiated the pause and contact them for context.
/// - `timestamp` - The ledger timestamp when the pause occurred. Used for
///   incident timeline and duration tracking.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=contract_paused_alert
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["contract_paused_alert"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Zero-Tolerance Alerting**: This dedicated topic enables setting up
///   PagerDuty rules with zero tolerance for contract pauses.
/// - **Incident Response**: This event should trigger immediate incident response
///   procedures and on-call notifications.
/// - **Uptime Monitoring**: Combined with `ContractUnpausedAlertEvent` (if implemented),
///   enables precise uptime/downtime tracking.
/// - **Admin Accountability**: The `admin` field enables tracking which admins
///   are initiating pauses and for what reasons.
/// - **SLA Monitoring**: Frequent pauses may impact SLA compliance and should be
///   monitored for service level agreement violations.
///
/// # Payload Size
/// Approximately 50-60 bytes. Very compact for efficient alerting.
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
///
/// This event helps identify large bets that may impact market liquidity,
/// indicate whale activity, or signal potential market manipulation. It's
/// useful for risk management and liquidity monitoring.
///
/// # When Emitted
/// Inside `place_prediction` when a user stakes an amount that meets or exceeds
/// the configured `HIGH_VALUE_THRESHOLD`. The prediction succeeds normally, but
/// this additional event is emitted for monitoring.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the pool receiving the large stake.
///   Used to understand which markets are attracting large bets.
/// - `user` - The address of the user placing the large prediction. Used to
///   identify whale accounts and track their betting patterns.
/// - `amount` - The staked amount that triggered the threshold. Used to understand
///   the magnitude of the bet and its impact on pool liquidity.
/// - `outcome` - The outcome index chosen. Used to understand which side of the
///   market the large bet is supporting.
/// - `threshold` - The threshold value that was breached. Used for display in
///   dashboards and to understand the sensitivity of the alert.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=high_value_prediction
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["high_value_prediction"]]
///     }
///   }
/// }
///
/// Filter by user:
/// {
///   "topics": [["high_value_prediction"], "{user_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Liquidity Monitoring**: Large bets can significantly impact pool liquidity
///   and odds. Indexers should alert liquidity managers to these events.
/// - **Whale Tracking**: Aggregating by `user` identifies whale accounts and their
///   betting patterns for relationship management.
/// - **Market Manipulation Detection**: Sudden large bets on specific outcomes may
///   indicate insider trading or market manipulation attempts.
/// - **Risk Management**: Risk teams should monitor these events to ensure the
///   protocol has sufficient liquidity to cover potential payouts.
/// - **Dashboard Display**: These events should be highlighted in real-time
///   dashboards for operators and liquidity providers.
///
/// # Payload Size
/// Approximately 80-90 bytes. Compact for efficient monitoring.
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
///
/// This diagnostic event provides additional context about pool resolution that
/// is useful for analytics, anomaly detection, and payout calculations. It helps
/// identify unusual resolution scenarios and validate payout calculations.
///
/// # When Emitted
/// Inside `resolve_pool` when a pool is successfully resolved. This event is
/// emitted in addition to `PoolResolvedEvent` to provide enriched diagnostic data.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the resolved pool. Used to correlate
///   diagnostic data with the resolution event.
/// - `outcome` - The winning outcome index. Used to identify which outcome
///   won and calculate payout ratios.
/// - `total_stake` - The total stake across all outcomes at resolution time.
///   Used to calculate the total pool size and payout ratios.
/// - `winning_stake` - The stake on the winning outcome. If this is 0, it
///   indicates a notable anomaly where there are no winners.
/// - `timestamp` - The ledger timestamp at resolution time. Used for timeline
///   analysis and correlation with other events.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=pool_resolved_diag
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["pool_resolved_diag"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Payout Validation**: Indexers can use `total_stake` and `winning_stake` to
///   validate payout calculations and detect discrepancies.
/// - **Anomaly Detection**: A `winning_stake` of 0 indicates no winners, which may
///   indicate a problem with the market or resolution process.
/// - **Payout Ratio Calculation**: The ratio `total_stake / winning_stake` determines
///   the payout multiplier for winning predictions.
/// - **Market Health Analysis**: Low `winning_stake` relative to `total_stake` may
///   indicate unpopular outcomes or market inefficiency.
/// - **Revenue Tracking**: The difference between `total_stake` and payouts represents
///   protocol revenue from fees.
///
/// # Payload Size
/// Approximately 50-60 bytes. Compact for efficient diagnostic monitoring.
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
///
/// This event provides a summary of stake updates for markets with many outcomes.
/// Instead of emitting individual events for each outcome (which could be 32+ for
/// tournament brackets), this single event provides aggregate data for efficiency.
///
/// # When Emitted
/// Inside bulk stake update operations when multiple outcomes are updated in a
/// single transaction. Typically used for markets with many outcomes where
/// per-outcome events would be impractical.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the pool being updated. Used to
///   correlate stake updates with the pool's lifecycle.
/// - `options_count` - The number of outcomes in this pool. Used to understand
///   the scale of the market and the granularity of stake distribution.
/// - `total_stake` - The total stake across all outcomes after the update.
///   Used to track overall pool liquidity and market depth.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=outcome_stakes_updated
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["outcome_stakes_updated"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Bulk Update Tracking**: Indexers should use this event to track bulk stake
///   updates for multi-outcome markets without processing per-outcome events.
/// - **Liquidity Monitoring**: The `total_stake` field enables tracking overall pool
///   liquidity for complex markets.
/// - **Market Scale**: The `options_count` field helps identify large-scale markets
///   (e.g., tournaments) that may require special handling.
/// - **Efficiency**: This event reduces the number of events that need to be processed
///   for markets with many outcomes, improving indexer performance.
/// - **Odds Calculation**: For markets with many outcomes, indexers may need to
///   query contract state directly for per-outcome stakes since this event only
///   provides aggregate data.
///
/// # Payload Size
/// Approximately 40-50 bytes. Very compact for efficient bulk update tracking.
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
/// This event is emitted when an admin adds a new token to the protocol's whitelist,
/// enabling users to create pools and place predictions using that token. Only
/// whitelisted tokens can be used for staking in prediction markets.
///
/// # When Emitted
/// Inside `add_token_to_whitelist` when an authorized admin adds a token to the
/// protocol's collateral whitelist. The token must pass validation checks before
/// being added.
///
/// # Event Fields
/// - `admin` - The address of the admin adding the token. This address must hold
///   the ADMIN_ROLE in the access control contract.
/// - `token` - The Stellar Asset contract address being added to the whitelist.
///   This token can now be used for pool creation and predictions.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=token_whitelist_added
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["token_whitelist_added"]]
///     }
///   }
/// }
///
/// Filter by token:
/// {
///   "topics": [["token_whitelist_added"], "{token_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Token Selector Updates**: Frontends should update their token selector UIs
///   to include the newly whitelisted token.
/// - **Pool Creation**: Users can now create pools using this token, so pool
///   creation forms should support it.
/// - **Token Discovery**: Indexers should maintain a list of whitelisted tokens
///   for display in market discovery interfaces.
/// - **Liquidity Tracking**: The protocol should track liquidity and volume for each
///   whitelisted token separately.
/// - **Risk Management**: Each new token introduces new risk factors (volatility,
///   liquidity, regulatory) that should be monitored.
///
/// # Payload Size
/// Approximately 50-60 bytes. Compact for efficient whitelist management.
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
/// This event is emitted when an admin removes a token from the protocol's whitelist.
/// Existing pools using this token continue to operate, but new pools cannot be
/// created using the removed token.
///
/// # When Emitted
/// Inside `remove_token_from_whitelist` when an authorized admin removes a token
/// from the protocol's collateral whitelist. This is typically done for deprecated
/// tokens or those that no longer meet protocol standards.
///
/// # Event Fields
/// - `admin` - The address of the admin removing the token. This address must hold
///   the ADMIN_ROLE in the access control contract.
/// - `token` - The Stellar Asset contract address being removed from the whitelist.
///   New pools cannot be created using this token, but existing pools continue.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=token_whitelist_removed
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["token_whitelist_removed"]]
///     }
///   }
/// }
///
/// Filter by token:
/// {
///   "topics": [["token_whitelist_removed"], "{token_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Token Selector Updates**: Frontends should remove the token from their token
///   selector UIs for new pool creation.
/// - **Existing Pool Handling**: Existing pools using this token should continue to
///   display and operate normally, but new pool creation should be disabled.
/// - **User Notification**: Users with active positions in pools using this token
///   should be notified that the token is deprecated for new markets.
/// - **Liquidity Migration**: Indexers should track whether liquidity is being
///   migrated from deprecated tokens to supported alternatives.
/// - **Phase-out Tracking**: The protocol should track the phase-out process to ensure
///   all deprecated tokens are eventually fully resolved.
///
/// # Payload Size
/// Approximately 50-60 bytes. Compact for efficient whitelist management.
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
/// This event is emitted when an admin withdraws accumulated protocol fees from
/// the treasury to an external recipient. This is typically done for revenue distribution,
/// operational expenses, or treasury management.
///
/// # When Emitted
/// Inside `withdraw_treasury` when an authorized admin withdraws funds from the
/// protocol treasury. The admin must hold the ADMIN_ROLE and the withdrawal must
/// not exceed the available treasury balance.
///
/// # Event Fields
/// - `admin` - The address of the admin authorizing the withdrawal. This address
///   must hold the ADMIN_ROLE in the access control contract.
/// - `token` - The Stellar Asset contract address being withdrawn. The protocol
///   maintains separate treasury balances for each whitelisted token.
/// - `amount` - The amount withdrawn from the treasury. This reduces the treasury
///   balance for the specified token.
/// - `recipient` - The address receiving the withdrawn funds. This can be an
///   operational wallet, multisig, or any valid Stellar address.
/// - `remaining_balance` - The treasury balance after the withdrawal. Used to
///   track remaining available funds and prevent over-withdrawal.
/// - `timestamp` - The ledger timestamp of the withdrawal. Used for financial
///   accounting and audit trails.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=treasury_withdrawn
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["treasury_withdrawn"]]
///     }
///   }
/// }
///
/// Filter by token:
/// {
///   "topics": [["treasury_withdrawn"], "{token_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Revenue Tracking**: Financial accounting tools use this event to track
///   protocol revenue movements and treasury balance changes.
/// - **Audit Trail**: This event provides a complete audit trail of treasury withdrawals
///   for compliance and financial reporting.
/// - **Balance Validation**: The `remaining_balance` field enables validation that
///   withdrawals are not exceeding available funds.
/// - **Multi-Token Treasury**: Indexers should track treasury balances separately for
///   each token since the protocol supports multiple collateral types.
/// - **Cash Flow Analysis**: Aggregating withdrawals by time period enables cash flow
///   analysis and financial planning.
///
/// # Payload Size
/// Approximately 120-140 bytes. Larger due to multiple address and amount fields.
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
/// This event is emitted when a user claims their full stake refund from a canceled
/// pool. Refunds are available after a pool is canceled and users receive 100% of
/// their stake back without any fee deductions.
///
/// # When Emitted
/// Inside `claim_refund` when a user successfully claims their refund from a canceled
/// pool. This can only be done after the pool has been canceled and the refund window
/// has opened.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the canceled pool. Used to correlate
///   refunds with the cancellation event and pool lifecycle.
/// - `user` - The address of the user receiving the refund. This address must have
///   placed predictions on the canceled pool.
/// - `amount` - The refunded stake amount returned to the user. This is 100% of
///   the user's total stake with no fee deductions.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=refund_claimed
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["refund_claimed"]]
///     }
///   }
/// }
///
/// Filter by user:
/// {
///   "topics": [["refund_claimed"], "{user_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Refund Auditing**: Financial auditors use this event to verify that all
///   refunds were processed correctly for canceled markets.
/// - **User Refund Tracking**: Indexers should track which users have claimed refunds
///   to identify unclaimed refunds and send reminders.
/// - **Refund Rate Analysis**: The ratio of claimed refunds to total refunds indicates
///   user awareness and claim completion rates.
/// - **Treasury Impact**: Refunds reduce the pool's locked funds and should be tracked
///   for liquidity management.
/// - **Cancellation Cost Analysis**: Total refunds for a canceled pool represent the
///   financial impact of the cancellation.
///
/// # Payload Size
/// Approximately 50-60 bytes. Compact for efficient refund tracking.
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
/// This event is emitted when an admin proposes a contract upgrade by committing
/// a new WASM hash. This is the first step in the upgrade process, followed by a
/// timelock period before the upgrade can be applied.
///
/// # When Emitted
/// Inside `commit_upgrade` when an authorized admin proposes a contract upgrade.
/// The admin must hold the ADMIN_ROLE and the WASM hash must be valid. The upgrade
/// cannot be applied immediately; there is a mandatory timelock period.
///
/// # Event Fields
/// - `admin` - The address of the admin initiating the upgrade proposal. This address
///   must hold the ADMIN_ROLE in the access control contract.
/// - `new_wasm_hash` - The 32-byte cryptographic hash of the proposed WASM binary.
///   This hash uniquely identifies the new contract code and enables verification.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=upgrade
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["upgrade"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Governance Alerts**: This event should alert governance teams and security
///   monitoring of pending code changes that need review.
/// - **Upgrade Tracking**: Indexers should track upgrade proposals to ensure they
///   are applied within the expected timelock period.
/// - **Security Review**: Security teams should review the proposed WASM code
///   (referenced by the hash) before the upgrade is applied.
/// - **Version Planning**: This event enables planning for contract version changes
/// and coordination with frontend and infrastructure teams.
/// - **Audit Trail**: This event provides a complete audit trail of all upgrade
///   proposals for compliance and security analysis.
///
/// # Payload Size
/// Approximately 60-70 bytes. Compact due to fixed-size hash.
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
/// This event is emitted when a contract upgrade is successfully applied after the
/// mandatory timelock period. It marks the transition from the old contract version
/// to the new version.
///
/// # When Emitted
/// Inside `apply_upgrade` when the contract upgrade is successfully executed.
/// This can only be done after the timelock period has elapsed since the upgrade
/// was proposed via `commit_upgrade`.
///
/// # Event Fields
/// - `old_version` - The previous version integer of the contract. Used to track
///   version history and understand what was upgraded.
/// - `new_version` - The new version integer after the upgrade. Used to identify
///   the currently active contract version.
/// - `upgraded_by` - The address that executed the upgrade. This address must
///   hold the ADMIN_ROLE and can be different from the admin who proposed it.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=contract_upgraded
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["contract_upgraded"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Version Bump**: Indexers should update their internal version tracking to
///   reflect the new contract version.
/// - **Event Schema Changes**: New contract versions may introduce new event types
///   or change event schemas. Indexers should update their parsing logic accordingly.
/// - **Frontend Coordination**: Frontend applications may need to coordinate with
///   contract upgrades to ensure compatibility.
/// - **Feature Flags**: New versions may introduce new features that require frontend
///   support or configuration changes.
/// - **Rollback Planning**: The version history enables planning for potential
///   rollbacks if issues are discovered with the new version.
///
/// # Payload Size
/// Approximately 50-60 bytes. Compact for efficient version tracking.
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
/// This event is emitted when the oracle integration is configured for the first time.
/// It sets up the connection to the Pyth Network oracle and configures validation
/// parameters for price data freshness and confidence.
///
/// # When Emitted
/// Inside `init_oracle` when an authorized admin initializes the oracle integration.
/// This is a one-time setup that configures the Pyth contract address and validation
/// parameters for automated price-based resolution.
///
/// # Event Fields
/// - `admin` - The address of the admin initializing oracle parameters. This address
///   must hold the ADMIN_ROLE in the access control contract.
/// - `pyth_contract` - The address of the external Pyth Network oracle contract on Stellar.
///   This contract provides decentralized price feeds for various asset pairs.
/// - `max_price_age` - The maximum allowed staleness of price updates in seconds.
///   Price data older than this threshold is rejected during resolution.
/// - `min_confidence_ratio` - The minimum required confidence ratio in basis points.
///   Price data with higher uncertainty (lower confidence) is rejected.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=oracle_init
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["oracle_init"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Oracle Verification**: Off-chain price feed aggregators use this event to verify
///   the Pyth connection setup and validate the oracle configuration.
/// - **Price Feed Monitoring**: This event signals that price-based resolution is now
///   available, and indexers should start monitoring price feed updates.
/// - **Parameter Tracking**: The validation parameters should be tracked to understand
///   the protocol's price data quality requirements.
/// - **Market Discovery**: Markets with price conditions can now be resolved automatically,
///   so indexers should identify and track these markets.
///
/// # Payload Size
/// Approximately 80-90 bytes. Compact for efficient oracle setup tracking.
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
/// This event is emitted when fresh price data is received from the Pyth Network
/// oracle and stored in the contract. This data is used for automated price-based
/// resolution of prediction markets.
///
/// # When Emitted
/// Inside `update_price_feed` when an authorized oracle submits fresh price data.
/// This is typically called periodically by off-chain keepers to ensure price data
/// remains current for resolution.
///
/// # Event Fields
/// - `oracle` - The address of the oracle submitting the price update. This address
///   must hold the ORACLE_ROLE in the access control contract.
/// - `feed_pair` - The asset pair symbol (e.g., BTC/USD, ETH/USD). Used to identify
///   which market's price is being updated.
/// - `price` - The observed price value formatted with fixed precision (typically
///   8 decimals for Pyth). Used for price condition evaluation.
/// - `confidence` - The confidence interval margin provided by Pyth, representing
///   the uncertainty range (± value) around the price.
/// - `timestamp` - The Unix timestamp when the price was observed by the oracle.
///   Used for staleness validation during resolution.
/// - `expires_at` - The Unix timestamp when this price data expires. Price data
///   after this time is considered invalid for resolution.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=price_feed_updated
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["price_feed_updated"]]
///     }
///   }
/// }
///
/// Filter by feed pair:
/// {
///   "topics": [["price_feed_updated"], "{feed_pair_symbol}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Price Feed Auditing**: This event logs external price data feeds for auditing
///   financial market resolution and ensuring data integrity.
/// - **Staleness Monitoring**: Indexers should track the `timestamp` and `expires_at`
///   to monitor price data freshness and identify stale feeds.
/// - **Confidence Tracking**: The `confidence` field enables tracking price data quality
///   and identifying periods of high market volatility.
/// - **Market Resolution**: Indexers should correlate price updates with pools that have
///   price conditions to identify when automated resolution becomes possible.
/// - **Oracle Performance**: The frequency and timing of price updates can be used to
///   track oracle performance and reliability.
///
/// # Payload Size
/// Approximately 100-120 bytes. Moderate size due to multiple numeric fields.
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
/// This event is emitted when a price condition is set for a pool, enabling automated
/// resolution based on oracle price data. The condition specifies a target price,
/// comparison operator, and tolerance for determining the winning outcome.
///
/// # When Emitted
/// Inside `set_price_condition` when a price condition is configured for a pool.
/// This can be done during pool creation or later to enable automated resolution.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the target pool. Used to correlate the
///   price condition with the pool's lifecycle.
/// - `feed_pair` - The target asset pair symbol (e.g., BTC/USD). This must match
///   a price feed that is updated by the oracle.
/// - `target_price` - The target price threshold required to trigger resolution.
///   Specified in the same decimal format as the oracle feed (typically 8 decimals).
/// - `operator` - The comparison operator flag: 0 (Equal/GTE), 1 (Greater Than),
///   2 (Less Than). Defines how the current price is compared to the target.
/// - `tolerance_bps` - The allowed price tolerance in basis points (1 bp = 0.01%).
///   Creates a buffer around the target price to prevent resolution flips due to noise.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=price_condition_set
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["price_condition_set"]]
///     }
///   }
/// }
///
/// Filter by pool:
/// {
///   "topics": [["price_condition_set"], "{pool_id}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Automated Resolution**: Automated resolution bots monitor price conditions
///   against live price feeds to trigger resolution when conditions are met.
/// - **Market Discovery**: Indexers should identify pools with price conditions to
///   highlight markets that will resolve automatically based on oracle data.
/// - **Price Monitoring**: Indexers should monitor the specified `feed_pair` for price
///   updates that could trigger resolution for pools with conditions.
/// - **Condition Validation**: The condition parameters should be validated to ensure
///   they are reasonable and achievable (e.g., target price within historical range).
/// - **Resolution Prediction**: Indexers can predict when resolution will occur by
///   comparing current prices to target prices with tolerance.
///
/// # Payload Size
/// Approximately 80-90 bytes. Compact for efficient condition tracking.
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
/// This event is emitted when a pool is resolved automatically based on oracle price
/// data meeting the configured price condition. It connects the specific price data
/// point directly to the final market resolution outcome for auditability.
///
/// # When Emitted
/// Inside `resolve_by_price` when a pool is resolved automatically based on price
/// condition evaluation. This occurs after the pool's end time, resolution delay,
/// and when the current price meets the condition criteria.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the target pool. Used to correlate the
///   resolution with the pool's lifecycle and price condition.
/// - `feed_pair` - The asset pair symbol evaluated (e.g., BTC/USD). Identifies
///   which price feed was used for resolution.
/// - `current_price` - The actual price at resolution time from the oracle feed.
///   This is the price that was evaluated against the condition.
/// - `target_price` - The target price defined for the market condition. Used to
///   understand the condition that was evaluated.
/// - `outcome` - The determined winning outcome index based on the condition
///   evaluation. All predictions on this outcome are winners.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=price_resolved
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["price_resolved"]]
///     }
///   }
/// }
///
/// Filter by pool:
/// {
///   "topics": [["price_resolved"], "{pool_id}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Price Audit Trail**: This event connects the price feed data point directly
///   to the final market resolution outcome for complete auditability.
/// - **Resolution Verification**: Indexers can verify that the resolution was correct
///   by comparing `current_price` to `target_price` with the configured tolerance.
/// - **Oracle Accountability**: This event links specific oracle data to resolution
///   outcomes, enabling oracle performance tracking and accountability.
/// - **Market Transparency**: Users can verify that automated resolutions were fair
///   and based on actual oracle data, not arbitrary decisions.
/// - **Historical Analysis**: Historical price resolutions enable analysis of how
///   different price conditions performed and which were most accurate.
///
/// # Payload Size
/// Approximately 80-90 bytes. Compact for efficient resolution tracking.
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
/// This event is emitted when multiple oracles report different outcomes for the
/// same pool during multi-oracle consensus resolution. This indicates a disagreement
/// that requires operator intervention to resolve.
///
/// # When Emitted
/// Inside multi-oracle resolution when an oracle reports an outcome that conflicts
/// with a previously reported outcome for the same pool. This triggers a dispute
/// resolution process.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the target pool. Used to correlate the
///   conflict with the pool's lifecycle and resolution process.
/// - `oracle` - The reporting oracle address that provided the conflicting outcome.
///   This oracle disagrees with the previously reported outcome.
/// - `outcome` - The outcome reported by this oracle. The conflicting outcome that
///   triggered the conflict event.
/// - `existing_outcome` - The previously reported outcome on record. The outcome
///   that was already reported by another oracle.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=resolution_conflict
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["resolution_conflict"]]
///     }
///   }
/// }
/// ```
///
/// # Indexing Implications
/// - **Dispute Alert**: This event should trigger immediate operator dispute review
///   to resolve the conflict and determine the correct outcome.
/// - **Oracle Reliability**: Frequent conflicts from specific oracles may indicate
///   unreliable or malicious oracle behavior that needs investigation.
/// - **Resolution Delay**: Conflicts delay resolution until they are resolved, so
///   indexers should track conflict resolution time.
/// - **Consensus Tracking**: Indexers should track the consensus process to understand
///   how often conflicts occur and how they are resolved.
/// - **Security Monitoring**: Systematic conflicts may indicate coordinated oracle
///   attacks or data source manipulation.
///
/// # Payload Size
/// Approximately 60-70 bytes. Compact for efficient conflict monitoring.
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
/// This event is emitted when a user is granted permission to participate in a
/// private prediction pool. Private pools restrict participation to whitelisted
/// addresses only, providing exclusive access control.
///
/// # When Emitted
/// Inside `add_to_whitelist` when an authorized admin or pool creator adds a user
/// to the pool's participant whitelist. The user can then place predictions on the pool.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the target pool. Used to correlate the
///   whitelist addition with the pool's access control configuration.
/// - `user` - The user address added to the whitelist. This address can now place
///   predictions on the private pool.
/// - `added_by` - The address of the admin or creator who added the user. This
///   address must have permission to modify the pool's whitelist.
/// - `timestamp` - The ledger timestamp when the user was added. Used for audit
///   trails and access control history.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=added_to_whitelist
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["added_to_whitelist"]]
///     }
///   }
/// }
///
/// Filter by pool:
/// {
///   "topics": [["added_to_whitelist"], "{pool_id}"]]
/// }
///
/// Filter by user:
/// {
///   "topics": [["added_to_whitelist"], "{user_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Access Control Updates**: Private market dashboards update access control
///   eligibility to show which users can participate in exclusive markets.
/// - **User Notifications**: Notification services should alert users when they are
///   granted access to private pools.
/// - **Whitelist Management**: Indexers should maintain the current whitelist state
///   for each private pool to validate participation permissions.
/// - **Access Audit**: This event provides an audit trail of who granted access to
///   whom and when, useful for compliance and security.
/// - **Pool Discovery**: Users can discover private pools they have access to by
///   querying their whitelist additions.
///
/// # Payload Size
/// Approximately 80-90 bytes. Compact for efficient access control tracking.
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
/// This event is emitted when a user's permission to participate in a private
/// prediction pool is revoked. After removal, the user cannot place new predictions
/// on the pool, though existing predictions remain valid.
///
/// # When Emitted
/// Inside `remove_from_whitelist` when an authorized admin or pool creator removes
/// a user from the pool's participant whitelist. The user loses access to place new
/// predictions but retains existing positions.
///
/// # Event Fields
/// - `pool_id` - The unique identifier of the target pool. Used to correlate the
///   whitelist removal with the pool's access control configuration.
/// - `user` - The user address removed from the whitelist. This address can no longer
///   place new predictions on the private pool.
/// - `removed_by` - The address of the admin or creator who removed the user. This
///   address must have permission to modify the pool's whitelist.
/// - `timestamp` - The ledger timestamp when the user was removed. Used for audit
///   trails and access control history.
///
/// # Subscription Example
/// ```text
/// Horizon API:
/// GET /events?contract={contract_id}&topic=removed_from_whitelist
///
/// Soroban RPC:
/// {
///   "jsonrpc": "2.0",
///   "method": "getEvents",
///   "params": {
///     "filter": {
///       "contractIds": ["{contract_id}"],
///       "topics": [["removed_from_whitelist"]]
///     }
///   }
/// }
///
/// Filter by pool:
/// {
///   "topics": [["removed_from_whitelist"], "{pool_id}"]]
/// }
///
/// Filter by user:
/// {
///   "topics": [["removed_from_whitelist"], "{user_address}"]]
/// }
/// ```
///
/// # Indexing Implications
/// - **Access Revocation**: Frontends should immediately disable prediction placement
///   for the removed user on the private pool.
/// - **User Notifications**: Notification services should alert users when their access
///   to private pools is revoked.
/// - **Whitelist Management**: Indexers should update the current whitelist state
///   to reflect the removal and prevent unauthorized access.
/// - **Existing Positions**: Users with existing predictions on the pool should still
///   be able to claim winnings if their outcome wins, despite removal.
/// - **Access Audit**: This event provides an audit trail of access revocations for
///   compliance and security monitoring.
///
/// # Payload Size
/// Approximately 80-90 bytes. Compact for efficient access control tracking.
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
