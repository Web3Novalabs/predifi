//! Pool domain: prediction pool lifecycle — creation, configuration,
//! resolution, cancellation and pool queries.

use soroban_sdk::{Address, contractevent, contractimpl, contracttype, Env, log, String, Symbol, token, Vec};

use crate::gas_opt;
use crate::{
    Config,
    DataKey,
    MarketState,
    PoolResolvedDiagEvent,
    PredifiContract,
    PredifiContractArgs,
    PredifiContractClient,
    PredifiError,
    ResolutionConflictEvent,
    ResolutionVoteCastEvent,
    CANCELATION_DELAY,
    DEFAULT_MIN_POOL_DURATION,
    EMERGENCY_CANCEL_MULTISIG_THRESHOLD,
    INITIAL_LIQUIDITY_SAFETY_MARGIN_BPS,
    MAX_INITIAL_LIQUIDITY,
    MAX_OPTIONS_COUNT,
    MAX_POOL_DURATION,
    UNRESOLVED_OUTCOME,
};

#[contractimpl]
impl PredifiContract {
    /// Create a new prediction pool with configurable parameters.
    ///
    /// This function creates a new prediction market pool where users can stake tokens on
    /// specific outcomes. The pool is initialized in the `Active` state and accepts
    /// predictions once `start_time` is reached (or immediately if `start_time` is 0).
    ///
    /// There is no separate `pool_type` argument. The `category` argument is the pool's
    /// classification/type and must be one of the canonical category symbols accepted by
    /// the contract, such as `CATEGORY_SPORTS`, `CATEGORY_CRYPTO`, or `CATEGORY_FINANCE`.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment, providing access to storage, ledger, and auth
    /// - `creator` - The address creating the pool (must authenticate via `require_auth`)
    /// - `end_time` - Unix timestamp after which no more predictions are accepted. It must be
    ///   later than `start_time`, the current ledger timestamp, and the configured minimum
    ///   pool duration, and no later than `MAX_POOL_DURATION` from the current ledger time.
    /// - `token` - The Stellar token contract address used for staking. It must be whitelisted;
    ///   the token contract itself is called only when a stake or initial liquidity is transferred.
    /// - `options_count` - Number of possible outcomes (must be >= 2 and <= MAX_OPTIONS_COUNT = 100)
    /// - `category` - Market category symbol (e.g., `CATEGORY_SPORTS`, `CATEGORY_FINANCE`)
    /// - `config` - Pool configuration parameters via `PoolConfig` struct:
    ///   - `start_time` - Unix timestamp when the pool opens for predictions (0 = open immediately)
    ///   - `description` - Short human-readable description (max 256 bytes)
    ///   - `metadata_url` - URL to extended metadata, e.g., IPFS link (max 512 bytes)
    ///   - `min_stake` - Minimum stake amount per prediction (must be > 0)
    ///   - `max_stake` - Maximum stake per prediction (0 = no limit, else must be >= min_stake)
    ///   - `min_total_stake` - Minimum total stake required for resolution (must be > 0).
    ///   - `max_total_stake` - Hard cap on total stake (0 = no limit); when non-zero, any
    ///     supplied initial liquidity must satisfy the configured safety margin.
    ///   - `initial_liquidity` - Optional house money transferred from the creator to the
    ///     contract (must be non-negative and no greater than `MAX_INITIAL_LIQUIDITY`).
    ///   - `required_resolutions` - Number of oracle/operator votes needed for resolution (must be >= 1)
    ///   - `private` - If true, only whitelisted addresses can participate
    ///   - `whitelist_key` - Optional symbol for private pool access; when present it must
    ///     pass referral-code validation.
    ///   - `outcome_descriptions` - Human-readable labels for each outcome. The number of
    ///     labels must equal `options_count`, and every label must be 1..=`MAX_OUTCOME_DESCRIPTION_LEN`
    ///     bytes long.
    ///
    /// # Returns
    ///
    /// Returns `Ok(pool_id)` - The unique identifier of the newly created pool.
    /// The pool ID is auto-incremented and can be used to reference the pool in all subsequent
    /// operations. The returned pool starts in `MarketState::Active` with
    /// `UNRESOLVED_OUTCOME`; its total stake initially equals `config.initial_liquidity`.
    ///
    /// The `Err` result is used for `PredifiError::ContractPaused` and for invalid
    /// `options_count`. Other validation failures are raised as Soroban contract errors or
    /// assertion failures, as described below.
    ///
    /// # Emitted Events
    ///
    /// This function emits the following events:
    ///
    /// 1. **`PoolCreatedEvent`** (always emitted):
    ///   - `pool_id` - The new pool's unique identifier
    ///   - `creator` - The address that created the pool
    ///   - `end_time` - The pool's betting deadline
    ///   - `token` - The token contract used for staking
    ///   - `options_count` - Number of possible outcomes
    ///   - `metadata_url` - URL to extended metadata
    ///   - `initial_liquidity` - Amount of house money provided
    ///   - `category` - Market category
    ///   - `required_resolutions` - Resolution threshold
    ///   - `max_total_stake` - Total stake cap
    ///
    /// 2. **`InitialLiquidityProvidedEvent`** (conditional):
    ///   - Emitted only if `initial_liquidity > 0`
    ///   - `pool_id` - The pool receiving liquidity
    ///   - `creator` - The address providing liquidity
    ///   - `amount` - The amount of liquidity transferred
    ///
    /// # Errors and Validation Failures
    ///
    /// The function returns or raises the following typed protocol errors:
    ///
    /// - `ContractPaused` - The contract is currently paused; all state-mutating operations are blocked
    /// - `InvalidData` - `options_count` is less than 2 or greater than `MAX_OPTIONS_COUNT`.
    /// - `TokenNotWhitelisted` - The specified token is not on the allowed betting whitelist
    /// - `InvalidTimestamp` - `end_time` is not in the future, exceeds MAX_POOL_DURATION, or `end_time <= start_time`
    /// - `DeadlineInPast` - `end_time` or `start_time` is in the past (issue #1130)
    /// - `MetadataUrlInvalid` - `metadata_url` exceeds 512 bytes
    /// - `InitialLiquidityBelowSafetyMargin` - Initial liquidity is insufficient relative to `max_total_stake` (issue #1131)
    /// - `RequiredResolutionsExceedOperators` - `required_resolutions` > number of active operators
    /// - `OutcomeDescriptionTooLong` - An outcome description exceeds MAX_OUTCOME_DESCRIPTION_LEN (128 bytes)
    /// - `OutcomeDescriptionEmpty` / `OutcomeDescriptionTooLong` - An outcome label is
    ///   outside the allowed length range.
    ///
    /// Authentication failure for `creator.require_auth()` is reported by the Soroban
    /// authorization system, not as `PredifiError::Unauthorized`. Invalid description,
    /// stake-limit, liquidity, resolution-count, and minimum-duration values use the
    /// contract's assertion diagnostics. A failed initial-liquidity token transfer also
    /// propagates the token contract's failure. These failures abort the transaction and
    /// do not leave a partially initialized pool.
    ///
    /// # Validation Rules
    ///
    /// The function performs extensive validation before pool creation:
    ///
    /// **Time Validation:**
    /// - `end_time` must be > current_time (INV-8)
    /// - `end_time` must be > `start_time`
    /// - `end_time` must not exceed current_time + MAX_POOL_DURATION (365 days)
    /// - Pool duration must be >= min_pool_duration (default: 1 hour)
    /// - If `start_time > 0`, it must be > current_time (cannot schedule past start)
    ///
    /// **Token Validation:**
    /// - Token must be whitelisted via `is_token_whitelisted`
    /// - Token address validation is deferred to actual transfer (Soroban pattern)
    ///
    /// **Outcome Validation:**
    /// - `options_count` must be >= 2 (binary or multi-outcome)
    /// - `options_count` must be <= MAX_OPTIONS_COUNT (100)
    /// - `outcome_descriptions` length must equal `options_count`
    /// - Each outcome description must be between 1 and 128 bytes
    ///
    /// **Stake Validation:**
    /// - `min_stake` must be > 0
    /// - `max_stake` must be 0 or >= `min_stake`
    /// - `min_total_stake` must be > 0
    /// - `max_total_stake` must be >= 0
    ///
    /// **Liquidity Validation:**
    /// - `initial_liquidity` must be >= 0
    /// - `initial_liquidity` must not exceed MAX_INITIAL_LIQUIDITY
    /// - If `max_total_stake > 0` and `initial_liquidity > 0`: liquidity must be at least
    ///   `(max_total_stake * INITIAL_LIQUIDITY_SAFETY_MARGIN_BPS) / 10_000` (1% safety margin)
    ///
    /// **Resolution Validation:**
    /// - `required_resolutions` must be >= 1
    /// - `required_resolutions` must not exceed the number of active operators
    ///   (unless operator_count is 0, in which case oracle resolution is allowed)
    ///
    /// **Category Validation:**
    /// - Category must be one of the canonical category symbols (e.g., CATEGORY_SPORTS)
    ///
    /// **Private Pool Validation:**
    /// - If `whitelist_key` is provided, it must pass `validate_referral_code`
    ///
    /// # Storage Initialization
    ///
    /// The function initializes the following storage entries:
    ///
    /// - `DataKey::Pool(pool_id)` - The pool's full state
    /// - `DataKey::PartCnt(pool_id)` - Participant count (initialized to 0)
    /// - `DataKey::OutStakes(pool_id)` - Batch storage for outcome stakes (initialized to zeros)
    /// - `DataKey::CatPoolCt(category)` - Category pool count (incremented)
    /// - `DataKey::CatPoolIx(category, index)` - Category pool index (appended)
    /// - `DataKey::PoolIdCtr` - Pool ID counter (incremented)
    ///
    /// # Initial Liquidity Transfer
    ///
    /// If `initial_liquidity > 0`, the function transfers tokens from the creator to the contract:
    /// - Tokens are transferred via `token_client.transfer(creator, contract, initial_liquidity)`
    /// - This represents "house money" that participates in the pool
    /// - Initial liquidity is part of `pool.total_stake` but typically excluded from fee calculations
    /// - The transfer occurs after all validation to avoid wasting gas on invalid pools
    ///
    /// # Category Indexing
    ///
    /// Pools are indexed by category for efficient querying:
    /// - Each category maintains a count of pools (`CatPoolCt`)
    /// - Each category maintains an ordered list of pool IDs (`CatPoolIx`)
    /// - This enables category-based pool enumeration and discovery
    ///
    /// # Pre-conditions
    ///
    /// - `end_time > current_time` (INV-8)
    /// - Token must be whitelisted
    /// - Creator must have sufficient balance for `initial_liquidity` (if provided)
    /// - Access control contract must be deployed and have sufficient operators (if required_resolutions > 0)
    ///
    /// # Post-conditions
    ///
    /// - `Pool.state = Active`
    /// - `Pool.total_stake = initial_liquidity` (if provided, else 0)
    /// - `Pool.outcome = UNRESOLVED_OUTCOME`
    /// - `PoolIdCtr` is incremented
    /// - Category indexes are updated
    /// - `PoolCreatedEvent` is emitted
    /// - `InitialLiquidityProvidedEvent` is emitted (if liquidity provided)
    ///
    /// # Usage Examples
    ///
    /// ## Basic Binary Pool
    ///
    /// ```ignore
    /// use soroban_sdk::{Address, Symbol, symbol_short};
    ///
    /// let creator = Address::generate(&env);
    /// let token = Address::generate(&env);
    /// let end_time = env.ledger().timestamp() + 86400; // 24 hours from now
    ///
    /// let config = PoolConfig {
    ///     start_time: 0, // Open immediately
    ///     description: String::from_str(&env, "Will BTC exceed $100k?"),
    ///     metadata_url: String::from_str(&env, "ipfs://QmHash"),
    ///     min_stake: 1000,
    ///     max_stake: 1000000,
    ///     min_total_stake: 10000,
    ///     max_total_stake: 10000000,
    ///     initial_liquidity: 0,
    ///     required_resolutions: 1,
    ///     private: false,
    ///     whitelist_key: None,
    ///     outcome_descriptions: vec![&env,
    ///         String::from_str(&env, "No"),
    ///         String::from_str(&env, "Yes")
    ///     ],
    /// };
    ///
    /// let pool_id = contract.create_pool(
    ///     env,
    ///     creator,
    ///     end_time,
    ///     token,
    ///     2, // Binary: No/Yes
    ///     CATEGORY_CRYPTO,
    ///     config,
    /// )?;
    /// ```
    ///
    /// ## Pool with Initial Liquidity
    ///
    /// ```ignore
    /// let config = PoolConfig {
    ///     // ... other fields ...
    ///     initial_liquidity: 1000000, // Creator provides house money
    ///     max_total_stake: 10000000,
    ///     // ... other fields ...
    /// };
    ///
    /// let pool_id = contract.create_pool(env, creator, end_time, token, 2, CATEGORY_SPORTS, config)?;
    /// // Creator must have approved the contract to spend 1000000 tokens
    /// ```
    ///
    /// ## Private Pool with Whitelist
    ///
    /// ```ignore
    /// let config = PoolConfig {
    ///     // ... other fields ...
    ///     private: true,
    ///     whitelist_key: Some(symbol_short!("SECRET_KEY")),
    ///     // ... other fields ...
    /// };
    ///
    /// let pool_id = contract.create_pool(env, creator, end_time, token, 2, CATEGORY_FINANCE, config)?;
    /// // Only whitelisted users or those with the invite key can participate
    /// # Function Signature Parameters
    ///
    /// ## `env: Env`
    /// The Soroban environment providing ledger state, time, and contract context.
    /// Contains the current timestamp for validation and storage for persistent data.
    ///
    /// ## `creator: Address`
    /// The address creating the pool. Must be authenticated via `creator.require_auth()`.
    /// The creator receives `PoolCreatedEvent` and can modify certain pool parameters later.
    /// Must have sufficient token balance if `initial_liquidity > 0`.
    ///
    /// ## `end_time: u64`
    /// Unix timestamp when staking closes (deadline). Must be:
    /// - Strictly greater than `current_time` (INV-8)
    /// - Greater than `config.start_time`
    /// - Within `current_time + MAX_POOL_DURATION` (365 days default)
    /// - At least `current_time + min_pool_duration` (default: 1 hour)
    /// - If `config.start_time > 0`, must be > `config.start_time`
    /// - A `start_time` of 0 means "open immediately"
    ///
    /// ## `token: Address`
    /// The betting token contract address. Must be whitelisted via `is_token_whitelisted`.
    /// Token address validity is deferred to actual transfer (Soroban pattern).
    /// All predictions and winnings use this token.
    ///
    /// ## `options_count: u32`
    /// Number of possible outcomes (2 for binary, more for multi-outcome markets).
    /// Constraints:
    /// - Minimum: 2 (binary outcome)
    /// - Maximum: MAX_OPTIONS_COUNT (100)
    /// - Must match `config.outcome_descriptions.len()`
    /// - Each outcome must have a unique, non-empty description (1-128 bytes)
    ///
    /// ## `category: Symbol`
    /// Market category symbol for indexing (e.g., `CATEGORY_CRYPTO`, `CATEGORY_SPORTS`).
    /// Must be a canonical category symbol. Pools are indexed by category for:
    /// - Efficient pool enumeration by category
    /// - Category-based analytics and discovery
    /// - `CatPoolCt(category)` - pool count per category
    /// - `CatPoolIx(category, index)` - ordered pool IDs per category
    ///
    /// ## `config: PoolConfig`
    /// Detailed pool configuration struct with the following fields:
    ///
    /// - `start_time: u64` - When pool opens (0 = immediately, >0 = future timestamp)
    /// - `description: String` - Pool title (max 256 bytes)
    /// - `metadata_url: String` - Extended metadata URL (max 512 bytes, e.g., IPFS)
    /// - `min_stake: i128` - Minimum per-prediction stake (> 0)
    /// - `max_stake: i128` - Maximum per-prediction stake (0 = unlimited, otherwise >= min_stake)
    /// - `min_total_stake: i128` - Minimum total stake for valid resolution (> 0)
    /// - `max_total_stake: i128` - Hard cap on total stake (0 = unlimited)
    /// - `initial_liquidity: i128` - Creator-provided seed liquidity (typically fee-excluded)
    /// - `required_resolutions: u32` - Independent resolutions needed (1+)
    /// - `private: bool` - If true, only whitelisted users can participate
    /// - `whitelist_key: Option<Symbol>` - Secret symbol for private pool access
    /// - `outcome_descriptions: Vec<String>` - Labels for each outcome
    ///
    /// # Return Value
    ///
    /// Returns `Ok(u64)` with the unique pool ID on success. Pool IDs are monotonically
    /// assigned starting from 0. The ID can be used with:
    /// - `get_pool(pool_id)` - Retrieve pool state
    /// - `place_prediction(pool_id, outcome, amount)` - Place bets
    /// - `resolve_pool(pool_id, outcome)` - Finalize pool
    /// - `claim_winnings(user, pool_id)` - Collect winnings
    ///
    /// Returns `Err(PredifiError)` on validation failures:
    /// - `ContractPaused` - Contract is paused
    /// - `InvalidTimestamp` - Time constraints violated
    /// - `DeadlineInPast` - end_time is in the past
    /// - `TokenNotWhitelisted` - Token not on whitelist
    /// - `InvalidData` - Invalid options_count, stake limits, or liquidity
    /// - `InitialLiquidityBelowSafetyMargin` - Insufficient seed liquidity
    /// - `RequiredResolutionsExceedOperators` - Not enough operators
    /// - `MetadataUrlInvalid` - metadata_url exceeds 512 bytes
    /// - `Unauthorized` - Creator not authenticated (via Soroban auth)
    ///
    /// # Events Emitted
    ///
    /// - `PoolCreatedEvent` - Pool creation with all parameters
    /// - `InitialLiquidityProvidedEvent` - If liquidity provided
    ///
    /// # Storage Updated
    ///
    /// - `DataKey::Pool(pool_id)` - Full pool state
    /// - `DataKey::PartCnt(pool_id)` - Participant count (0)
    /// - `DataKey::OutStakes(pool_id)` - Outcome stakes (zeros)
    /// - `DataKey::CatPoolCt(category)` - Category count (incremented)
    /// - `DataKey::CatPoolIx(category, index)` - Category pool index
    /// - `DataKey::PoolIdCtr` - Counter (incremented)
    /// - `DataKey::ActivePoolIndex` - Active pool tracking
    ///
    /// # Post-conditions
    ///
    /// - `Pool.state == MarketState::Active`
    /// - `Pool.outcome == UNRESOLVED_OUTCOME`
    /// - `Pool.total_stake == initial_liquidity` (if provided)
    /// - `Pool.fee_bps` set from current config
    /// - Initial liquidity transferred from creator
    /// - Pool added to active index
    ///
    /// # Safety Guarantees
    ///
    /// - No partial pool state on failure (all-or-nothing)
    /// - No storage leaks on invalid inputs
    /// - Token transfers only after validation success
    /// - Index updates are atomic with pool creation
    /// - All invariants validated before persistence
    ///
    /// # Examples
    ///
    /// See the full examples in the main documentation above.
    #[allow(clippy::too_many_arguments)]
    pub fn create_pool(
        env: Env,
        creator: Address,
        end_time: u64,
        token: Address,
        options_count: u32,
        category: Symbol,
        config: PoolConfig,
    ) -> Result<u64, PredifiError> {
        Self::require_not_paused(&env)?;
        creator.require_auth();

        // Validate: category must be in the allowed list, return error if invalid
        let normalized_category = match Self::validate_category(&env, &category) {
            Ok(cat) => cat,
            Err(e) => soroban_sdk::panic_with_error!(&env, e),
        };

        // Validate: token must be on the allowed betting whitelist
        if !Self::is_token_whitelisted(&env, &token) {
            soroban_sdk::panic_with_error!(&env, PredifiError::TokenNotWhitelisted);
        }

        let current_time = env.ledger().timestamp();

        // Validate: end_time must be greater than start_time
        if end_time <= config.start_time {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidTimestamp);
        }

        // Issue #1130 — staking deadlines must be in the future. End_time
        // strictly in the past raises the `DeadlineInPast` protocol error
        // (typed, machine-checkable). The legacy `assert!` below still
        // catches the boundary case `end_time == current_time` with its
        // diagnostic message for backwards-compatible test expectations.
        // A `start_time` of 0 is a sentinel for "open immediately"; any
        // non-zero start_time must be strictly in the future so a creator
        // cannot schedule a pool that opens in the past.
        if end_time < current_time {
            soroban_sdk::panic_with_error!(&env, PredifiError::DeadlineInPast);
        }
        if config.start_time > 0 && config.start_time < current_time {
            soroban_sdk::panic_with_error!(&env, PredifiError::DeadlineInPast);
        }

        // Validate: end_time must be in the future (legacy assert kept for
        // diagnostic clarity; the DeadlineInPast check above is the
        // authoritative protocol error).
        assert!(end_time > current_time, "end_time must be in the future");

        // Validate: end_time must not exceed MAX_POOL_DURATION from now
        if end_time > current_time + MAX_POOL_DURATION {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidTimestamp);
        }

        let min_pool_duration = env
            .storage()
            .instance()
            .get::<DataKey, Config>(&DataKey::Config)
            .map(|c| c.min_pool_duration)
            .unwrap_or(DEFAULT_MIN_POOL_DURATION);

        // Validate: minimum pool duration
        if min_pool_duration > 0 {
            assert!(
                end_time >= current_time + min_pool_duration,
                "end_time must be at least min_pool_duration in the future"
            );
        }

        // Validate: options_count must be at least 2 (binary or more outcomes)
        if options_count < 2 {
            return Err(PredifiError::InvalidData);
        }

        // Validate: options_count must not exceed maximum limit
        if options_count > MAX_OPTIONS_COUNT {
            return Err(PredifiError::InvalidData);
        }

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

        // Issue #1131 — initial-liquidity safety margin. When the creator
        // caps the pool at `max_total_stake > 0`, the seeded liquidity must
        // cover at least INITIAL_LIQUIDITY_SAFETY_MARGIN_BPS basis points of
        // that cap, so that early high-value predictions cannot drain a
        // thinly-seeded pool faster than it can be cancelled.
        if config.max_total_stake > 0 && config.initial_liquidity > 0 {
            // safety_min = max_total_stake * bps / 10_000 — saturating to
            // avoid arithmetic panics on extreme inputs.
            let bps = INITIAL_LIQUIDITY_SAFETY_MARGIN_BPS as i128;
            let safety_min = config
                .max_total_stake
                .checked_mul(bps)
                .map(|v| v / 10_000)
                .unwrap_or(i128::MAX);
            if config.initial_liquidity < safety_min {
                soroban_sdk::panic_with_error!(
                    &env,
                    PredifiError::InitialLiquidityBelowSafetyMargin
                );
            }
        }

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
            // Use try_invoke_contract so that an unreachable access-control contract maps
            // to OracleNotInitialized rather than causing an unhandled panic.
            let operator_count: u32 = env
                .try_invoke_contract::<u32, PredifiError>(
                    &cfg.access_control,
                    &Symbol::new(&env, "get_operator_count"),
                    soroban_sdk::vec![&env],
                )
                .map_err(|_| PredifiError::OracleNotInitialized)
                .and_then(|inner| inner.map_err(|_| PredifiError::OracleNotInitialized))
                .unwrap_or(0u32); // default: treat as zero operators so creation can proceed
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
        assert!(
            config.min_total_stake > 0,
            "min_total_stake must be greater than zero"
        );
        assert!(config.max_total_stake >= 0, "max_total_stake must be >= 0");

        if let Some(ref whitelist_key) = config.whitelist_key {
            if let Err(e) = Self::validate_referral_code(&env, whitelist_key) {
                soroban_sdk::panic_with_error!(&env, e);
            }
        }

        // outcome_descriptions validation is now handled by validate_pool_invariants
        // called right after pool structure is initialized.

        let pool_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PoolIdCtr)
            .unwrap_or(0);
        // Initialize pool data structure
        let pool = Pool {
            start_time: config.start_time,
            end_time,
            state: MarketState::Active,
            outcome: UNRESOLVED_OUTCOME,
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
            fee_bps: Self::get_config(&env).fee_bps,
            participants_count: 0,
            resolution_timestamp: None, // Set when pool is resolved
        };

        Self::validate_pool_invariants(&pool);

        let pool_key = DataKey::Pool(pool_id);
        env.storage().persistent().set(&pool_key, &pool);
        Self::bump_ttl(&env, &pool_key);

        // Initialize optimized batch storage with zeros to avoid expensive fallback reads
        let initial_stakes = gas_opt::alloc_zero_stakes(&env, options_count);
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

        Ok(pool_id)
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
        Self::require_not_paused(&env)?;
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

        // Issue #1142: emit event so frontends/indexers can update the displayed cap.
        MaxTotalStakeIncreasedEvent {
            pool_id,
            creator,
            new_max_total_stake,
        }
        .publish(&env);

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
        Self::require_not_paused(&env)?;
        caller.require_auth();

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::validate_pool_invariants(&pool);
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
        // We read the pool's participants_count field which is the authoritative participant counter.
        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env.storage().persistent().get(&pool_key).expect("Pool not found");
        if pool.participants_count > 0 {
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

    /// Finalises a prediction pool by recording an operator's vote for a winning outcome.
    ///
    /// This is the manual, operator-backed resolution entry point for prediction markets.
    /// Resolution uses a **multi-vote / threshold model** to ensure consensus among trusted
    /// operators before finalizing a pool's result. Each address with Operator role (role `1`)
    /// calls this function once to cast a vote for a specific outcome. The pool transitions
    /// to [`MarketState::Resolved`] only when one outcome accumulates at least
    /// `pool.required_resolutions` votes.
    ///
    /// ## Multi-Vote Threshold Model
    ///
    /// The protocol implements a Byzantine fault-tolerant resolution mechanism:
    ///
    /// ```text
    /// ┌─────────────────────────────────────────────────────────────────────────────┐
    /// │ 1. Operator votes for outcome (this function)                               │
    /// │    - Operator calls resolve_pool with pool_id and winning_outcome           │
    /// │    - Vote stored in temporary storage (DataKey::ResVote)                    │
    /// │    - Vote count incremented per outcome (DataKey::ResVoteCt)                │
    /// │    - ResolutionVoteCastEvent emitted for observability                      │
    /// │                                                                              │
    /// │ 2. Threshold check after each vote                                          │
    /// │    - new_outcome_votes >= pool.required_resolutions?                        │
    /// │    - If YES: pool transitions to Resolved, outcome locked                   │
    /// │    - If NO: pool stays Active, more votes needed                            │
    /// │                                                                              │
    /// │ 3. Conflict detection                                                       │
    /// │    - If different operators vote for different outcomes                     │
    /// │    - ResolutionConflictEvent emitted                                        │
    /// │    - Error returned, pool stays Active                                      │
    /// │    - Allows manual adjudication before final resolution                     │
    /// └─────────────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// ## Operator Roles and Trust Model
    ///
    /// Operators are trusted entities configured by the contract admin:
    ///
    /// * **Role**: Operators hold role `1` defined in the role management system
    /// * **Authentication**: Each operator must sign the transaction via `require_auth()`
    /// * **Voting**: Each operator may vote exactly once per pool (duplicate votes rejected)
    /// * **Consensus**: `pool.required_resolutions` votes determine the final outcome
    /// * **Disagreement**: Conflicting votes trigger `ResolutionConflictEvent` for review
    ///
    /// ## Resolution Delay Enforcement
    ///
    /// A configurable cooling-off period prevents premature resolution:
    ///
    /// * **Formula**: `eligible_at = pool.end_time + config.resolution_delay`
    /// * **Guard**: `current_ledger_timestamp >= eligible_at`
    /// * **Purpose**: Allows time for:
    ///   * Late price-feed updates to propagate
    ///   * All oracle observations to settle
    ///   * Off-chain price reconciliation before locking in outcome
    /// * **Saturating arithmetic**: Prevents timestamp overflow from making pools resolvable earlier
    /// * **Inclusive comparison**: Resolution allowed at the exact `eligible_at` timestamp
    ///
    /// ## Resolution Paths Comparison
    ///
    /// The protocol provides multiple resolution paths, all sharing the same guard and event
    /// semantics but differing in how the winning outcome is determined:
    ///
    /// | Path | Function | Outcome Source | Use Case |
    /// |------|----------|----------------|----------|
    /// | Manual (this) | `resolve_pool` | Operator vote | Admin-controlled resolution, dispute resolution |
    /// | Automated | `resolve_pool_from_price` | Price feed evaluation | Fully automated markets with price conditions |
    /// | Oracle-signed | `oracle_resolve` | Oracle signature | Markets requiring signed oracle data |
    ///
    /// * **Manual (`resolve_pool`)**: Operator manually determines and votes for the winning
    ///   outcome. Suitable for markets without automated price feeds or when manual intervention
    ///   is required (e.g., resolving disputes, edge cases).
    /// * **Automated (`resolve_pool_from_price`)**: Protocol reads configured price feed,
    ///   evaluates the condition (e.g., "BTC/USD > 50000"), and automatically persists the
    ///   resulting outcome (0 or 1) without human intervention.
    /// * **Oracle-signed (`oracle_resolve`)**: External oracle provides a cryptographically
    ///   signed outcome that is verified against the oracle registry before being recorded
    ///   as a vote in the threshold model.
    ///
    /// All three paths enforce the same invariants:
    /// * Pool must be in `Active` state
    /// * Resolution delay must have elapsed
    /// * Outcome must be valid (0 <= outcome < options_count, not UNRESOLVED_OUTCOME)
    /// * Pool is removed from active index only after reaching threshold
    /// * `PoolResolvedEvent` and `PoolResolvedDiagEvent` are emitted upon finalization
    ///
    /// # Parameters
    ///
    /// * `env` - The [`Env`](soroban_sdk::Env) providing access to:
    ///   * **Ledger state**: Timestamp, sequence numbers for temporal validations
    ///   * **Storage**: Persistent storage for pool state, temporary storage for votes
    ///   * **Authentication**: `operator.require_auth()` for role verification
    ///   * **Event emission**: Soroban event system for resolution events
    /// * `operator` - The [`Address`](soroban_sdk::Address) casting a resolution vote. Must:
    ///   * Hold Operator role (`1`) for this pool's role management context
    ///   * Sign the transaction via `require_auth()`
    ///   * Not have previously voted for this pool (duplicate guard)
    ///   * Be authorized to participate in the resolution threshold
    /// * `pool_id` - A unique 64-bit identifier referencing the prediction pool to resolve.
    ///   The pool must exist in persistent storage at `DataKey::Pool(pool_id)`.
    /// * `outcome` - A 0-based index of the winning outcome the operator is voting for.
    ///   Must satisfy `0 <= outcome < pool.options_count` and must not equal
    ///   `UNRESOLVED_OUTCOME` (`u32::MAX`), which is reserved for pools that have not
    ///   yet been decided.
    ///
    /// # Resolution Flow
    ///
    /// The resolution process follows a strict sequence of validations and vote tallies.
    /// Each step is idempotent and can be executed in a single transaction, with all
    /// state mutations occurring atomically.
    ///
    /// ## Step-by-Step Flow
    ///
    /// ```text
    /// ┌──────────────────────────────────────────────────────────────────────────────┐
    /// │ Step 1: Auth & Role Check                                                    │
    /// │   • operator.require_auth()                                                  │
    /// │   • require_operator_role_for_resolution(&env, &operator, pool_id)           │
    /// │   • Returns Err(InvalidPoolState) if pool not Active                         │
    /// │                                                                              │
    /// │ Step 2: Resolution Delay Check                                               │
    /// │   • eligible_at = pool.end_time + config.resolution_delay                    │
    /// │   • if current_time < eligible_at: Err(ResolutionDelayNotMet)                │
    /// │                                                                              │
    /// │ Step 3: Outcome Validation                                                   │
    /// │   • if outcome >= pool.options_count: Err(InvalidOutcome)                    │
    /// │   • if outcome == UNRESOLVED_OUTCOME: Err(InvalidOutcome)                    │
    /// │                                                                              │
    /// │ Step 4: Duplicate Vote Guard                                                 │
    /// │   • if DataKey::ResVote(pool_id, operator) exists: Err(OracleAlreadyVoted)   │
    /// │                                                                              │
    /// │ Step 5: Vote Recording                                                       │
    /// │   • Store vote: DataKey::ResVote(pool_id, operator) = outcome                │
    /// │   • Increment total votes: DataKey::ResTotal(pool_id)++                      │
    /// │   • Increment outcome votes: DataKey::ResVoteCt(pool_id, outcome)++          │
    /// │   • Emit ResolutionVoteCastEvent                                             │
    /// │                                                                              │
    /// │ Step 6: Conflict Detection                                                   │
    /// │   • For each outcome i ≠ current_outcome:                                    │
    /// │     - if DataKey::ResVoteCt(pool_id, i) > 0:                                 │
    /// │       • Emit ResolutionConflictEvent                                         │
    /// │       • Return Err(ResolutionConflict)                                       │
    /// │                                                                              │
    /// │ Step 7: Threshold Check & Finalization                                       │
    /// │   • if new_outcome_votes >= pool.required_resolutions:                       │
    /// │     • pool.state = Resolved                                                  │
    /// │     • pool.outcome = outcome                                                 │
    /// │     • pool.fee_bps = calculate_dynamic_fee(&env, &pool)                      │
    /// │     • pool.resolution_timestamp = current_time                               │
    /// │     • Remove from active index (remove_from_active_index)                    │
    /// │     • Emit PoolResolvedEvent                                                 │
    /// │     • Emit PoolResolvedDiagEvent                                             │
    /// └──────────────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// ## Detailed Step Explanations
    ///
    /// ### 1. Auth & Role Check
    ///
    /// The function first verifies that:
    /// * The transaction is signed by the claimed `operator` address
    /// * The `operator` holds the Operator role (`1`) for this pool
    /// * The pool exists and is in `Active` state
    ///
    /// Failure at this stage prevents any state mutations and returns early.
    ///
    /// ### 2. Resolution Delay Check
    ///
    /// The cooling-off period ensures:
    /// * All price updates have time to propagate
    /// * All oracle observations have settled
    /// * No premature finalization before market conditions stabilize
    ///
    /// **Example**: If `pool.end_time = 1000` and `config.resolution_delay = 3600` (1 hour):
    /// * Resolution allowed at or after timestamp 4600
    /// * Attempt at timestamp 4599 returns `ResolutionDelayNotMet`
    /// * Attempt at timestamp 4600 proceeds to next check
    ///
    /// ### 3. Outcome Validation
    ///
    /// Two validation rules ensure outcome integrity:
    /// * `outcome < pool.options_count`: Prevents out-of-bounds indexing
    /// * `outcome != UNRESOLVED_OUTCOME`: Prevents using sentinel value as final outcome
    ///
    /// ### 4. Duplicate Vote Guard
    ///
    /// Each operator may vote exactly once to prevent:
    /// * Operator bias through repeated voting
    /// * Sybil attacks via operator address duplication
    /// * Accidental double-voting from transaction retries
    ///
    /// The vote is stored in **temporary storage** (`DataKey::ResVote`) which is isolated
    /// to the current transaction but visible to all contract functions within that transaction.
    ///
    /// ### 5. Vote Recording
    ///
    /// Three storage keys track the voting state:
    ///
    /// | Key | Type | Value | Purpose |
    /// |-----|------|-------|---------|
    /// | `DataKey::ResVote(pool_id, operator)` | `u32` | outcome | Which outcome this operator voted for |
    /// | `DataKey::ResTotal(pool_id)` | `u32` | total votes | Total number of votes cast for this pool |
    /// | `DataKey::ResVoteCt(pool_id, outcome)` | `u32` | vote count | Number of votes for this specific outcome |
    ///
    /// Temporary storage is automatically cleaned up at the end of the transaction.
    ///
    /// ### 6. Conflict Detection
    ///
    /// After recording the new vote, the function checks if any **other** outcome already
    /// has votes:
    ///
    /// ```rust,no_run
    /// // Pseudocode for conflict detection
    /// if new_total_votes > new_outcome_votes {
    ///     for i in 0..pool.options_count {
    ///         if i == outcome { continue; }  // Skip current outcome
    ///         if outcome_votes[i] > 0 {
    ///             // Conflict detected! Emit event and return error.
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// **Why check if `new_total_votes > new_outcome_votes`?**
    /// * If this is the first vote (`new_total_votes == new_outcome_votes == 1`), no conflict possible
    /// * If the new vote is the second vote but for the same outcome, no conflict (consensus)
    /// * Only if different outcomes have votes do we have a genuine conflict
    ///
    /// **Behavior**: Conflict is non-fatal to the transaction. The current vote is recorded,
    /// the event is emitted, but the pool remains `Active` for manual resolution.
    ///
    /// ### 7. Threshold Check & Finalization
    ///
    /// Once a threshold is reached, the following state transitions occur atomically:
    ///
    /// ```rust,no_run
    /// pool.state = MarketState::Resolved;
    /// pool.outcome = outcome;
    /// pool.fee_bps = Self::calculate_dynamic_fee(&env, &pool);
    /// pool.resolution_timestamp = Some(env.ledger().timestamp());
    /// Self::remove_from_active_index(&env, pool_id);
    /// ```
    ///
    /// **Dynamic Fee Calculation**: The fee rate is determined based on the pool's total
    /// stake at resolution time, using the protocol's dynamic fee tier system.
    ///
    /// **Active Index Removal**: The pool is removed from the global `ActiveIndex` to
    /// prevent further predictions and to enable efficient enumeration of active pools.
    ///
    /// # Oracle and Price Feed Relationship
    ///
    /// This manual resolution function does **not** read or evaluate price feeds. The
    /// outcome is already determined by the calling operator based on external information
    /// (price feeds, off-chain data, market events). For automated price-condition-based
    /// resolution, see [`Self::resolve_pool_from_price`].
    ///
    /// ## Resolution Path Comparison
    ///
    /// The protocol provides two automated resolution paths that integrate with price feeds
    /// and oracles:
    ///
    /// ### Automated Price-Feed Resolution (`resolve_pool_from_price`)
    ///
    /// ```text
    /// ┌──────────────────────────────────────────────────────────────────────────────┐
    /// │ 1. Load PriceCondition: pool.price_condition                               │
    /// │    • feed_id: Price feed identifier                                         │
    /// │    • target: Target price level                                             │
    /// │    • operator: Comparison operator (GT, LT, EQ, GTE, LTE)                   │
    /// │    • tolerance: Acceptable deviation from target                            │
    /// │                                                                              │
    /// │ 2. Fetch Price Feed Data                                                    │
    /// │    • Load FeedConfig for feed_id                                            │
    /// │    • Retrieve latest price from price feed adapter                          │
    /// │    • Validate feed is not stale (timestamp within tolerance)                │
    /// │    • Validate feed is not expired (expiration timestamp not reached)        │
    /// │                                                                              │
    /// │ 3. Evaluate Price Condition                                                 │
    /// │    • normalized_price = feed_price / 10^feed_decimals                       │
    /// │    • condition_met = compare(normalized_price, target, operator, tolerance) │
    /// │       • GT:  normalized_price >  target + tolerance                         │
    /// │       • LT:  normalized_price <  target - tolerance                         │
    /// │       • GTE: normalized_price >= target + tolerance                         │
    /// │       • LTE: normalized_price <= target - tolerance                         │
    /// │       • EQ:  |normalized_price - target| <= tolerance                       │
    /// │                                                                              │
    /// │ 4. Persist Outcome                                                          │
    /// │    • outcome = if condition_met { 1 } else { 0 }                            │
    /// │    • pool.state = Resolved                                                  │
    /// │    • pool.outcome = outcome                                                 │
    /// │    • ... (same finalization as manual resolution)                           │
    /// └──────────────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// ### Oracle-Signed Resolution (`oracle_resolve`)
    ///
    /// The oracle callback path allows external oracles to submit cryptographically
    /// signed outcomes:
    ///
    /// ```text
    /// ┌──────────────────────────────────────────────────────────────────────────────┐
    /// │ 1. Oracle Registry Lookup                                                   │
    /// │    • Verify oracle address is registered                                    │
    /// │    • Load oracle's public key and signature scheme                          │
    /// │                                                                              │
    /// │ 2. Signature Verification                                                   │
    /// │    • Verify signature over pool_id + outcome + timestamp                    │
    /// │    • Verify signature was created by registered oracle                      │
    /// │    • Verify timestamp is within acceptable window                           │
    /// │                                                                              │
    /// │ 3. Record Oracle Vote                                                       │
    /// │    • Store vote in temporary storage (same as manual)                       │
    /// │    • Increment vote tallies (same as manual)                                │
    /// │    • Conflict detection (same as manual)                                    │
    /// │    • Threshold check (same as manual)                                       │
    /// └──────────────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// ## Shared Invariants Across All Paths
    ///
    /// All three resolution paths (`resolve_pool`, `resolve_pool_from_price`, `oracle_resolve`)
    /// enforce identical invariants:
    ///
    /// | Invariant | Description |
    /// |-----------|-------------|
    /// | `pool.state == Active` | Pool must be active to allow resolution |
    /// | `resolution_delay elapsed` | Cooling-off period must be satisfied |
    /// | `valid outcome` | Outcome is within bounds and not sentinel |
    /// | `active index removal` | Pool removed from active index only after resolution |
    /// | `event emission` | `PoolResolvedEvent` and `PoolResolvedDiagEvent` emitted |
    ///
    /// This ensures consistent behavior regardless of how the winning outcome was determined.
    ///
    /// ## Price Feed Integration Details
    ///
    /// For automated resolution, the price feed system provides:
    ///
    /// * **Feed Registration**: Price feeds are registered with metadata (asset pair,
    ///   decimal places, update frequency, source)
    /// * **Data Validation**: Each feed includes staleness and expiration checks
    /// * **Operator Selection**: Configurable comparison operators (>, <, >=, <=, ==)
    /// * **Tolerance Support**: Configurable tolerance for edge cases (e.g., 0.1% drift)
    /// * **Fallback Mechanism**: If feed is unavailable, manual resolution remains available
    ///
    /// See `price_feed.rs` and `price_feed_simple.rs` for complete price feed implementation.
    ///
    /// # Payout Calculation (Post-Resolution)
    ///
    /// This function does **not** distribute funds directly. Instead, it finalizes the
    /// pool's state and prepares it for the `claim_winnings` phase, where winners can
    /// claim their proportional share of the pool.
    ///
    /// ## Two-Phase Resolution and Claiming
    ///
    /// The protocol separates resolution from payout to ensure atomicity and allow
    /// time for dispute windows:
    ///
    /// ```text
    /// ┌──────────────────────────────────────────────────────────────────────────────┐
    /// │ PHASE 1: Resolution (this function)                                          │
    /// │ • Pool transitions from Active → Resolved                                    │
    /// │ • Winning outcome is locked in (pool.outcome)                                │
    /// │ • Protocol fee rate is determined (pool.fee_bps)                             │
    /// │ • Resolution timestamp is recorded (pool.resolution_timestamp)               │
    /// │ • Pool is removed from active index                                          │
    /// │ • Event emitted: PoolResolvedEvent                                           │
    /// │ • NO funds are transferred yet                                               │
    /// └──────────────────────────────────────────────────────────────────────────────┘
    ///                                    │
    ///                                    ▼
    /// ┌──────────────────────────────────────────────────────────────────────────────┐
    /// │ PHASE 2: Claiming (claim_winnings function)                                 │
    /// │ • Winners call claim_winnings with their prediction data                    │
    /// │ • Protocol fee is calculated from pool total                                │
    /// │ • Referral fees are calculated (if applicable)                              │
    /// │ • User's proportional share is computed                                     │
    /// │ • Tokens are transferred to winner and referrer                             │
    /// │ • Claimed sentinel is set to prevent double-claim                           │
    /// └──────────────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// ## Payout Algorithm Details
    ///
    /// When a user calls `claim_winnings` after resolution, the payout is calculated as:
    ///
    /// ```text
    /// Given:
    ///   s = user's stake on winning outcome
    ///   W = total stake on winning outcome (winning_stake)
    ///   T = total pool stake (pool.total_stake)
    ///   f = protocol fee rate = pool.fee_bps / 10_000
    ///
    /// Step 1: Calculate Protocol Fee (from entire pool)
    ///   protocol_fee = T * f
    ///
    /// Step 2: Calculate Payout Pool (remaining after fee)
    ///   payout_pool = T - protocol_fee
    ///
    /// Step 3: Calculate User's Proportional Share of Payout Pool
    ///   user_share = s * payout_pool / W
    ///
    /// Step 4: Calculate Total Winnings (user_share + returned principal)
    ///   winnings = user_share + s
    ///   (This equals s * (payout_pool / W + 1) = s * (T - protocol_fee) / W + s)
    ///
    /// Step 5: Calculate Referral Fee (if applicable)
    ///   referral_fee = protocol_fee * referral_bps / 10_000
    ///   (referral_bps is Config::referral_bps, default 500 = 5%)
    ///
    /// Final Transfers:
    ///   - Referrer receives: referral_fee
    ///   - Treasury receives: protocol_fee - referral_fee
    ///   - User receives: winnings - referral_fee
    /// ```
    ///
    /// ## Example Calculation
    ///
    /// ```text
    /// Pool State (after resolution):
    ///   pool.total_stake = 100,000 tokens
    ///   winning_stake = 60,000 tokens (outcome 1)
    ///   pool.fee_bps = 25 (0.25%)
    ///   referral_bps = 500 (5%)
    ///
    /// User Prediction:
    ///   user.stake = 1,000 tokens (on outcome 1)
    ///   user.referrer = Some("GA...XYZ")
    ///
    /// Payout Calculation:
    ///   Step 1: protocol_fee = 100,000 * 25 / 10_000 = 250 tokens
    ///   Step 2: payout_pool = 100,000 - 250 = 99,750 tokens
    ///   Step 3: user_share = 1,000 * 99,750 / 60,000 = 1,662.5 tokens
    ///   Step 4: winnings = 1,662.5 + 1,000 = 2,662.5 tokens
    ///   Step 5: referral_fee = 250 * 500 / 10_000 = 12.5 tokens
    ///
    /// Final Transfers:
    ///   Referrer receives: 12.5 tokens
    ///   Treasury receives: 250 - 12.5 = 237.5 tokens
    ///   User receives: 2,662.5 - 12.5 = 2,650 tokens
    ///   Total distributed: 2,650 + 12.5 + 237.5 + 97,100 (other winners) = 100,000 ✓
    /// ```
    ///
    /// ## Arithmetic Properties
    ///
    /// * **Checked arithmetic**: All multiplications and divisions use `SafeMath` to
    ///   prevent overflow/underflow
    /// * **Protocol-favouring rounding**: Integer division rounds in favor of the protocol
    ///   (e.g., `floor(s * payout_pool / W)` ensures protocol receives at least its fee)
    /// * **Zero stake handling**: Zero winning stake or zero user stake produces zero winnings
    /// * **No value creation**: No claimant can receive more than the pool's total stake
    ///
    /// ## Edge Cases
    ///
    /// | Case | Behavior |
    /// |------|----------|
    /// | `winning_stake == 0` | All stakes refunded (no pool fee applied) |
    /// | `user_stake == 0` | User receives 0 winnings (no participation) |
    /// | `pool.total_stake == 0` | Invalid state, should not occur |
    /// | `protocol_fee > user_winnings` | User receives 0 (all goes to protocol) |
    /// | `referral_fee > protocol_fee` | Impossible by construction |
    ///
    /// See `claim_winnings_internal` in `prediction.rs` for complete payout implementation.
    ///
    /// # Emitted Events
    ///
    /// This function emits events in two categories: ongoing voting events (emitted on
    /// every successful call) and finalization events (emitted only when threshold is
    /// reached and pool transitions to `Resolved` state).
    ///
    /// ## Ongoing Voting Events
    ///
    /// ### ResolutionVoteCastEvent
    ///
    /// Emitted **every time** a successful vote is recorded, regardless of whether
    /// the threshold has been reached.
    ///
    /// ```rust,ignore
    /// event ResolutionVoteCastEvent {
    ///     pool_id: u64,
    ///     voter: Address,
    ///     outcome: u32,
    ///     vote_count: u32,      // Number of votes for this outcome AFTER this vote
    ///     required_resolutions: u32  // Threshold needed for resolution
    /// }
    /// ```
    ///
    /// **Fields:**
    /// * `pool_id` - The prediction pool being resolved
    /// * `voter` - The operator address casting the vote
    /// * `outcome` - The winning outcome index the operator voted for
    /// * `vote_count` - The **cumulative** vote count for this outcome **after** this vote
    /// * `required_resolutions` - The threshold number of votes required for final resolution
    ///
    /// **Use cases:**
    /// * Live resolution tracking dashboard
    /// * Alerting when a threshold is approaching
    /// * Monitoring operator participation
    ///
    /// **Example timeline:**
    /// ```text
    /// Threshold = 3 votes
    ///
    /// Vote 1 (Operator A votes for outcome 1):
    ///   ResolutionVoteCastEvent {
    ///     pool_id: 1,
    ///     voter: "GA...AAA",
    ///     outcome: 1,
    ///     vote_count: 1,         // First vote for outcome 1
    ///     required_resolutions: 3
    ///   }
    ///
    /// Vote 2 (Operator B votes for outcome 1):
    ///   ResolutionVoteCastEvent {
    ///     pool_id: 1,
    ///     voter: "GA...BBB",
    ///     outcome: 1,
    ///     vote_count: 2,         // Second vote for outcome 1
    ///     required_resolutions: 3
    ///   }
    ///
    /// Vote 3 (Operator C votes for outcome 1):
    ///   ResolutionVoteCastEvent {
    ///     pool_id: 1,
    ///     voter: "GA...CCC",
    ///     outcome: 1,
    ///     vote_count: 3,         // Third vote for outcome 1
    ///     required_resolutions: 3
    ///   }
    ///   → Threshold met! Pool transitions to Resolved.
    /// ```
    ///
    /// ### ResolutionConflictEvent
    ///
    /// Emitted when an operator casts a vote for an outcome that **differs** from an
    /// earlier vote already recorded for this pool.
    ///
    /// ```rust,ignore
    /// event ResolutionConflictEvent {
    ///     pool_id: u64,
    ///     oracle: Address,        // Address casting the conflicting vote
    ///     outcome: u32,           // Outcome this operator just voted for
    ///     existing_outcome: u32   // Outcome that already had votes
    /// }
    /// ```
    ///
    /// **Fields:**
    /// * `pool_id` - The prediction pool with conflicting votes
    /// * `oracle` - The operator address casting the conflicting vote
    /// * `outcome` - The outcome index this operator just voted for
    /// * `existing_outcome` - The outcome index that already had votes (conflict target)
    ///
    /// **Behavior:**
    /// * The current vote **is still recorded** (not discarded)
    /// * The conflict event is emitted for observability
    /// * The function returns `Err(PredifiError::ResolutionConflict)`
    /// * The pool remains in `Active` state for manual resolution
    ///
    /// **Use cases:**
    /// * Operator consensus monitoring
    /// * Dispute detection and escalation
    /// * Operator reputation tracking
    ///
    /// **Example scenario:**
    /// ```text
    /// Initial state: Operator A votes for outcome 1
    /// Current state: outcome 1 has 2 votes, outcome 0 has 0 votes
    ///
    /// New vote: Operator B votes for outcome 0
    ///   → ResolutionConflictEvent emitted:
    ///     oracle: "GA...BBB", outcome: 0, existing_outcome: 1
    ///   → Returns Err(ResolutionConflict)
    ///   → Pool stays Active, administrators investigate dispute
    /// ```
    ///
    /// ## Finalization Events
    ///
    /// These events are emitted **only once** when the pool transitions to `Resolved`
    /// state after reaching the required vote threshold.
    ///
    /// ### PoolResolvedEvent
    ///
    /// Emitted when the vote threshold is reached and the pool transitions from
    /// `Active` to `Resolved` state.
    ///
    /// ```rust,ignore
    /// event PoolResolvedEvent {
    ///     pool_id: u64,
    ///     operator: Address,      // Operator who triggered threshold (final vote caster)
    ///     outcome: u32            // Final winning outcome
    /// }
    /// ```
    ///
    /// **Fields:**
    /// * `pool_id` - The prediction pool that was resolved
    /// * `operator` - The operator address that cast the **final vote** triggering threshold
    /// * `outcome` - The winning outcome index that achieved the required votes
    ///
    /// **Use cases:**
    /// * Off-chain resolution indexing
    /// * Notification of pool finalization
    /// * Triggering claim processing pipelines
    ///
    /// **Example:**
    /// ```json
    /// {
    ///   "topic": ["PoolResolvedEvent"],
    ///   "data": [
    ///     {"val": "u64", "Val": 123},
    ///     {"val": "bytes", "Val": "GA...XYZ"},
    ///     {"val": "u32", "Val": 1}
    ///   ]
    /// }
    /// ```
    ///
    /// ### PoolResolvedDiagEvent
    ///
    /// A diagnostic event emitted alongside `PoolResolvedEvent` to provide stake
    /// analytics for the resolved pool.
    ///
    /// ```rust,ignore
    /// event PoolResolvedDiagEvent {
    ///     pool_id: u64,
    ///     outcome: u32,           // Winning outcome
    ///     total_stake: i128,      // Total tokens staked in pool
    ///     winning_stake: i128,    // Tokens staked on winning outcome
    ///     timestamp: u64          // Ledger timestamp of resolution
    /// }
    /// ```
    ///
    /// **Fields:**
    /// * `pool_id` - The prediction pool that was resolved
    /// * `outcome` - The winning outcome index
    /// * `total_stake` - Total tokens staked in the pool (all outcomes)
    /// * `winning_stake` - Tokens staked on the winning outcome
    /// * `timestamp` - Ledger timestamp when resolution occurred
    ///
    /// **Use cases:**
    /// * Pool performance analytics
    /// * Volume tracking by outcome
    /// * Protocol revenue calculation
    /// * Post-resolution reporting
    ///
    /// **Example:**
    /// ```json
    /// {
    ///   "topic": ["PoolResolvedDiagEvent"],
    ///   "data": [
    ///     {"val": "u64", "Val": 123},
    ///     {"val": "u32", "Val": 1},
    ///     {"val": "i128", "Val": "100000000"},
    ///     {"val": "i128", "Val": "60000000"},
    ///     {"val": "u64", "Val": 1234567890}
    ///   ]
    /// }
    /// ```
    ///
    /// ## Event Ordering Guarantees
    ///
    /// * Events are emitted **after** all validations pass
    /// * Events are emitted **after** state mutations are committed
    /// * `ResolutionVoteCastEvent` and `ResolutionConflictEvent` are emitted **before**
    ///   the function returns (even on error)
    /// * `PoolResolvedEvent` and `PoolResolvedDiagEvent` are emitted **after** pool
    ///   state is updated to `Resolved`
    /// * All events use the Soroban event system (`publish`)
    ///
    /// ## Event Summary Table
    ///
    /// | Event | Trigger | Emitted on Error? | Purpose |
    /// |-------|---------|-------------------|---------|
    /// | `ResolutionVoteCastEvent` | Every vote | No | Track voting progress |
    /// | `ResolutionConflictEvent` | Vote conflict | No | Detect operator disagreement |
    /// | `PoolResolvedEvent` | Threshold reached | No | Finalize pool resolution |
    /// | `PoolResolvedDiagEvent` | Threshold reached | No | Provide stake analytics |
    ///
    /// # Errors
    ///
    /// All errors are of type [`PredifiError`] and cause the transaction to abort
    /// with a Soroban error code. Errors are checked in the following order:
    /// contract pause → operator role → pool state → resolution delay → outcome
    /// validity → duplicate vote → conflict detection.
    ///
    /// ## Error Reference
    ///
    /// | Error | Code | Check Location | Description | Recovery Guidance |
    /// |-------|------|----------------|-------------|-------------------|
    /// | `PredifiError::ContractPaused` | 100 | First | Contract is globally paused; all state-mutating operations suspended | Wait for admin to unpause |
    /// | `PredifiError::InvalidPoolState` | 105 | Second | Pool is not in `Active` state (Resolved, Cancelled, or non-existent) | Verify pool exists and is Active |
    /// | `PredifiError::ResolutionDelayNotMet` | 201 | Third | Current ledger time is before `eligible_at` | Wait for `end_time + resolution_delay` to elapse |
    /// | `PredifiError::InvalidOutcome` | 106 | Fourth | Outcome is out of bounds or equals sentinel | Validate `0 <= outcome < options_count` and `outcome != u32::MAX` |
    /// | `PredifiError::OracleAlreadyVoted` | 202 | Fifth | This operator has already voted for this pool | Use a different operator address or wait for resolution |
    /// | `PredifiError::ResolutionConflict` | 203 | Sixth | A different outcome already has votes; operators disagree | Administrators must adjudicate and resolve the dispute |
    ///
    /// ## Error Code Categories
    ///
    /// * **Codes 100-199**: User input and validation errors (paused contract, invalid state, invalid outcome)
    /// * **Codes 200-299**: Resolution-specific errors (delay not met, duplicate vote, conflict)
    ///
    /// ## Recovery Guidance
    ///
    /// ### For Resolvers (Operators)
    ///
    /// * **ContractPaused**: Monitor contract status via admin events; wait for admin to unpause
    /// * **InvalidPoolState**: Verify pool exists via `get_pool` and check `state == Active`
    /// * **ResolutionDelayNotMet**: Calculate `eligible_at = pool.end_time + config.resolution_delay` and wait
    /// * **InvalidOutcome**: Confirm `pool.options_count` and use valid 0-based index
    /// * **OracleAlreadyVoted**: Check `DataKey::ResVote(pool_id, operator)`; use different operator
    /// * **ResolutionConflict**: Administrators must review conflicting votes and decide on final outcome
    ///
    /// ### For Front-End Integration
    ///
    /// ```rust,no_run
    /// // Example: Checking if resolution is possible before attempting
    /// let pool = client.get_pool(&pool_id);
    /// let config = client.get_config();
    /// let current_time = env.ledger().timestamp();
    /// let eligible_at = pool.end_time + config.resolution_delay;
    ///
    /// if current_time < eligible_at {
    ///     // Resolution not yet possible; show countdown to user
    /// } else if pool.state != MarketState::Active {
    ///     // Pool already resolved or cancelled
    /// } else {
    ///     // Safe to attempt resolution
    ///     client.resolve_pool(&operator, &pool_id, &winning_outcome);
    /// }
    /// ```
    ///
    /// ### For Admin Monitoring
    ///
    /// * Monitor `ResolutionConflictEvent` for operator disagreements requiring intervention
    /// * Monitor `PoolResolvedEvent` to track resolution throughput
    /// * Monitor `ResolutionVoteCastEvent` for slow resolution (approaching but not reaching threshold)
    ///
    /// ## Conflict Resolution Workflow
    ///
    /// When `ResolutionConflict` is encountered:
    ///
    /// ```text
    /// 1. Conflict detected: Operator B votes for outcome 0 while outcome 1 already has votes
    ///    ↓
    /// 2. ResolutionConflictEvent emitted; function returns error
    ///    ↓
    /// 3. Pool remains in Active state with both outcomes having votes
    ///    ↓
    /// 4. Administrators review conflict:
    ///    - Check external data sources for correct outcome
    ///    - Communicate with operators to reach consensus
    ///    - Determine if pool should be manually resolved or cancelled
    ///    ↓
    /// 5. Operators vote again:
    ///    - If consensus reached: vote for agreed outcome
    ///    - If conflict persists: repeat until threshold met for one outcome
    ///    - If dispute unresolved: admin may cancel pool (refunds to participants)
    ///    ↓
    /// 6. Threshold reached for one outcome → PoolResolvedEvent emitted
    /// ```
    ///
    /// # Preconditions
    ///
    /// All preconditions must be satisfied for the function to proceed. Failures return
    /// errors early, before any state mutations. The function checks preconditions in
    /// the order listed below for optimal error reporting.
    ///
    /// ## Auth & Role Preconditions
    ///
    /// | Condition | INV | Description | Verification Method |
    /// |-----------|-----|-------------|---------------------|
    /// | `operator.require_auth()` | INV-1 | Transaction signed by claimed operator | Soroban `require_auth()` |
    /// | `operator` has Operator role | INV-17 | Operator holds role `1` for pool | `require_operator_role_for_resolution()` |
    ///
    /// ## Pool State Preconditions
    ///
    /// | Condition | INV | Description | Error on Failure |
    /// |-----------|-----|-------------|------------------|
    /// | `pool.state == MarketState::Active` | INV-2 | Pool is not Locked, Resolved, or Cancelled | `InvalidPoolState` |
    /// | Pool exists in storage | INV-18 | Pool record present at `DataKey::Pool(pool_id)` | Panic (pool not found) |
    ///
    /// ## Temporal Preconditions
    ///
    /// | Condition | INV | Description | Calculation |
    /// |-----------|-----|-------------|-------------|
    /// | `current_time >= eligible_at` | INV-19 | Resolution delay has elapsed | `eligible_at = pool.end_time + config.resolution_delay` |
    ///
    /// **Example**: If `pool.end_time = 10000` and `config.resolution_delay = 3600` (1 hour):
    /// * Resolution allowed at or after timestamp 13600
    /// * Attempt at timestamp 13599 returns `ResolutionDelayNotMet`
    /// * Saturating addition prevents overflow from making pools resolvable earlier
    ///
    /// ## Outcome Validation Preconditions
    ///
    /// | Condition | INV | Description | Sentinel Value |
    /// |-----------|-----|-------------|----------------|
    /// | `0 <= outcome < pool.options_count` | INV-20 | Outcome within valid range | `UNRESOLVED_OUTCOME` (`u32::MAX`) |
    /// | `outcome != UNRESOLVED_OUTCOME` | INV-21 | Not using sentinel as final outcome | `u32::MAX` |
    ///
    /// ## Vote Integrity Preconditions
    ///
    /// | Condition | INV | Description | Storage Location |
    /// |-----------|-----|-------------|------------------|
    /// | No existing vote from operator | INV-22 | Operator has not voted previously | `DataKey::ResVote(pool_id, operator)` |
    ///
    /// ## Precondition Checklist (for callers)
    ///
    /// ```rust,no_run
    /// // Before calling resolve_pool, verify:
    /// let pool = client.get_pool(&pool_id);
    /// let config = client.get_config();
    /// let current_time = env.ledger().timestamp();
    /// let eligible_at = pool.end_time + config.resolution_delay;
    ///
    /// // 1. Contract not paused (implicit via require_not_paused)
    /// // 2. Operator has correct role (implicit via require_operator_role_for_resolution)
    /// assert_eq!(pool.state, MarketState::Active, "Pool must be Active");
    /// // 3. Resolution delay elapsed
    /// assert!(current_time >= eligible_at, "Resolution delay not met");
    /// // 4. Outcome is valid
    /// assert!(outcome < pool.options_count, "Outcome out of bounds");
    /// assert!(outcome != u32::MAX, "Outcome cannot be UNRESOLVED_OUTCOME");
    /// // 5. Operator has not voted (check via DataKey::ResVote)
    /// // 6. Transaction is signed by operator
    /// ```
    ///
    /// # Postconditions (On Threshold Reached)
    ///
    /// When `new_outcome_votes >= pool.required_resolutions`, the following state
    /// transitions occur atomically. All postconditions are guaranteed if the function
    /// returns `Ok(())` with `PoolResolvedEvent` emitted.
    ///
    /// ## Pool State Transitions
    ///
    /// | Field | Before | After | Description |
    /// |-------|--------|-------|-------------|
    /// | `pool.state` | `MarketState::Active` | `MarketState::Resolved` | Pool finalization |
    /// | `pool.outcome` | undefined | `outcome` | Winning outcome recorded |
    /// | `pool.fee_bps` | dynamic tier (if not set) | `calculate_dynamic_fee(&env, &pool)` | Fee rate determined at resolution |
    /// | `pool.resolution_timestamp` | `None` | `Some(current_time)` | Resolution time stamp |
    ///
    /// ## Storage Mutations
    ///
    /// | Key | Type | Change | Purpose |
    /// |-----|------|--------|---------|
    /// | `DataKey::Pool(pool_id)` | `Pool` | state=Resolved, outcome, fee_bps, resolution_timestamp | Pool finalization |
    /// | `DataKey::ResVote(pool_id, operator)` | `u32` | Removed (temp) | Vote removed from temporary storage |
    /// | `DataKey::ResTotal(pool_id)` | `u32` | Removed (temp) | Vote count cleared |
    /// | `DataKey::ResVoteCt(pool_id, outcome)` | `u32` | Removed (temp) | Outcome vote count cleared |
    /// | ActiveIndex | `Vec<u64>` | `pool_id` removed | Pool no longer in active enumeration |
    ///
    /// **Note**: Temporary storage keys (`ResVote`, `ResTotal`, `ResVoteCt`) are cleared
    /// automatically when the transaction ends, but explicit removal may occur for cleanup.
    ///
    /// ## Event Emissions
    ///
    /// | Event | Trigger | Guaranteed When |
    /// |-------|---------|-----------------|
    /// | `ResolutionVoteCastEvent` | Vote recording | Always (before threshold) |
    /// | `ResolutionConflictEvent` | Conflict detection | If conflict exists (before threshold) |
    /// | `PoolResolvedEvent` | Threshold reached | **Only** when `new_outcome_votes >= required_resolutions` |
    /// | `PoolResolvedDiagEvent` | Threshold reached | **Only** when threshold reached |
    ///
    /// ## Invariant Guarantees
    ///
    /// | INV | Statement | Verification |
    /// |-----|-----------|--------------|
    /// | INV-2 | State transition: `Active → Resolved` | `pool.state == MarketState::Resolved` |
    /// | INV-23 | Outcome recording: `pool.outcome == voted_outcome` | `pool.outcome == outcome` |
    /// | INV-24 | Resolution timestamp: `Some(timestamp)` recorded | `pool.resolution_timestamp.is_some()` |
    /// | INV-25 | Active index removal: Pool not in active set | `get_active_pools()` excludes `pool_id` |
    /// | INV-26 | Fee calculation: `fee_bps` set | `pool.fee_bps > 0` (unless pool has zero stake) |
    ///
    /// ## Claimability Postconditions
    ///
    /// After resolution, predictions are claimable:
    ///
    /// ```text
    /// Claim eligibility:
    ///   • Prediction exists at DataKey::Pred(user, pool_id)
    ///   • Prediction.outcome == pool.outcome (winning outcome)
    ///   • Claimed(user, pool_id) == false (not yet claimed)
    ///
    /// Payout eligibility:
    ///   • Winning-side predictions: proportional payout of (pool.total_stake - protocol_fee)
    ///   • Losing-side predictions: 0 payout (stake returned as part of payout calculation)
    ///   • No prediction: 0 payout (no participation)
    /// ```
    ///
    /// See `claim_winnings_internal` for complete claiming logic.
    ///
    /// ## Atomicity Guarantees
    ///
    /// * All storage mutations occur in a single transaction
    /// * If any mutation fails, all prior mutations are rolled back (Soroban atomicity)
    /// * Events are emitted after all mutations are complete
    /// * No partial resolution is possible; threshold is either met or not met
    ///
    /// ## Edge Case: Threshold Reached in Single Vote
    ///
    /// If `required_resolutions == 1` (single operator pool):
    /// * First vote immediately meets threshold
    /// * `new_outcome_votes == new_total_votes == 1`
    /// * No conflict possible (no other outcomes have votes)
    /// * `PoolResolvedEvent` emitted in same transaction
    ///
    /// This configuration is suitable for admin-controlled test pools or single-operator
    /// markets where the operator is the sole authority for resolution.
    pub fn resolve_pool(
        env: Env,
        operator: Address,
        pool_id: u64,
        outcome: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        operator.require_auth();
        Self::require_operator_role_for_resolution(&env, &operator, pool_id)?;

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        Self::validate_pool_invariants(&pool);

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
            return Err(PredifiError::InvalidOutcome);
        }

        // Validate: outcome cannot be the sentinel value
        if outcome == UNRESOLVED_OUTCOME {
            log!(
                &env,
                "resolve_pool rejected: outcome cannot be sentinel value",
                pool_id,
                operator.clone(),
                outcome
            );
            return Err(PredifiError::InvalidOutcome);
        }

        // --- Multi-resolution Voting Logic ---

        // Check if this operator has already voted for this pool
        let vote_key = DataKey::ResVote(pool_id, operator.clone());
        if env.storage().temporary().has(&vote_key) {
            log!(
                &env,
                "resolve_pool rejected: operator already voted",
                pool_id,
                operator.clone(),
                outcome
            );
            return Err(PredifiError::OracleAlreadyVoted); // Reusing error code for operators
        }

        // Record the operator's vote in temporary storage
        env.storage().temporary().set(&vote_key, &outcome);
        Self::extend_temporary(&env, &vote_key);

        // Increment total number of votes cast for this pool
        let total_votes_key = DataKey::ResTotal(pool_id);
        let total_votes: u32 = env.storage().temporary().get(&total_votes_key).unwrap_or(0);
        let new_total_votes = total_votes + 1;
        env.storage()
            .temporary()
            .set(&total_votes_key, &new_total_votes);
        Self::extend_temporary(&env, &total_votes_key);

        // Increment specific outcome vote count
        let outcome_votes_key = DataKey::ResVoteCt(pool_id, outcome);
        let outcome_votes: u32 = env
            .storage()
            .temporary()
            .get(&outcome_votes_key)
            .unwrap_or(0);
        let new_outcome_votes = outcome_votes + 1;
        env.storage()
            .temporary()
            .set(&outcome_votes_key, &new_outcome_votes);
        Self::extend_temporary(&env, &outcome_votes_key);

        // Emit a ResolutionVoteCastEvent for observability
        ResolutionVoteCastEvent {
            pool_id,
            voter: operator.clone(),
            outcome,
            vote_count: new_outcome_votes,
            required_resolutions: pool.required_resolutions,
        }
        .publish(&env);

        // Detect conflicts
        if new_total_votes > new_outcome_votes {
            for i in 0..pool.options_count {
                if i == outcome {
                    continue;
                }
                let other_key = DataKey::ResVoteCt(pool_id, i);
                if env.storage().temporary().has(&other_key) {
                    ResolutionConflictEvent {
                        pool_id,
                        oracle: operator.clone(),
                        outcome,
                        existing_outcome: i,
                    }
                    .publish(&env);
                    return Err(PredifiError::ResolutionConflict);
                }
            }
        }

        // Check if the required threshold has been met
        if new_outcome_votes >= pool.required_resolutions {
            pool.state = MarketState::Resolved;
            pool.outcome = outcome;
            pool.fee_bps = Self::calculate_dynamic_fee(&env, &pool);
            pool.resolution_timestamp = Some(env.ledger().timestamp()); // Record resolution time

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
        Self::require_not_paused(&env)?;
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        if pool.state != MarketState::Active {
            return Err(PredifiError::InvalidPoolState);
        }

        if env.storage().persistent().has(&DataKey::PoolReady(pool_id)) {
            return Ok(());
        }

        if !env
            .storage()
            .persistent()
            .has(&DataKey::StakingClosed(pool_id))
        {
            return Err(PredifiError::StakingStillOpen);
        }

        let config = Self::get_config(&env);
        let current_time = env.ledger().timestamp();

        if current_time >= pool.end_time.saturating_add(config.resolution_delay) {
            let ready_key = DataKey::PoolReady(pool_id);
            env.storage().persistent().set(&ready_key, &true);
            Self::extend_persistent(&env, &ready_key);
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

    /// Signal that staking has closed for a pool and emit a `StakingClosedEvent`.
    ///
    /// Staking is considered closed once `pool.end_time` has passed — at that
    /// point no more predictions can be placed.  This function provides an
    /// explicit, permissionless on-chain signal of that transition so that
    /// off-chain subscribers (event indexers, front-ends, keepers) can react
    /// without having to poll block timestamps themselves.
    ///
    /// The function is **idempotent**: if it has already been called for the
    /// given pool, subsequent calls succeed silently without re-emitting the
    /// event.  This prevents duplicate events even if multiple callers race to
    /// trigger the transition.
    ///
    /// # Arguments
    /// * `pool_id` - The ID of the pool whose staking window has ended.
    ///
    /// # Errors
    /// - `PoolNotFound`     — no pool exists for `pool_id`.
    /// - `InvalidPoolState` — pool is not in the `Active` state (already
    ///                        resolved or cancelled pools have no open staking
    ///                        window to close).
    /// - `StakingStillOpen` — `pool.end_time` has not yet been reached; the
    ///                        staking window is still open.
    ///
    /// PRE:  pool.state = Active, env.ledger().timestamp() >= pool.end_time
    /// POST: StakingClosed(pool_id) sentinel written; StakingClosedEvent emitted
    ///       exactly once per pool.
    pub fn close_staking(env: Env, pool_id: u64) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .ok_or(PredifiError::PoolNotFound)?;

        // Only pools that are still Active have a meaningful open staking window.
        if pool.state != MarketState::Active {
            return Err(PredifiError::InvalidPoolState);
        }

        let current_time = env.ledger().timestamp();

        // Staking closes when the ledger passes end_time.
        if current_time < pool.start_time || current_time < pool.end_time {
            return Err(PredifiError::StakingStillOpen);
        }

        // Idempotency guard — if the event has already been emitted for this
        // pool, return success without re-emitting.
        let sentinel_key = DataKey::StakingClosed(pool_id);
        if env.storage().persistent().has(&sentinel_key) {
            return Ok(());
        }

        // Mark as closed and emit the event.
        env.storage().persistent().set(&sentinel_key, &true);
        Self::extend_persistent(&env, &sentinel_key);

        StakingClosedEvent {
            pool_id,
            end_time: pool.end_time,
            total_stake: pool.total_stake,
            timestamp: current_time,
        }
        .publish(&env);

        Ok(())
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
        Self::require_not_paused(&env)?;
        operator.require_auth();

        // Protect state-modifying external interactions from reentrancy
        Self::enter_reentrancy_guard(&env);

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
                    Self::exit_reentrancy_guard(&env);
                    return Err(PredifiError::Unauthorized);
                }
                if pool.total_stake > pool.initial_liquidity {
                    Self::exit_reentrancy_guard(&env);
                    return Err(PredifiError::Unauthorized);
                }
            }
        }

        // Ensure only Active pools can be canceled
        // This prevents canceling pools that are already Resolved or Canceled
        if !Self::is_pool_active(&pool) {
            Self::exit_reentrancy_guard(&env);
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

        Self::exit_reentrancy_guard(&env);

        Ok(())
    }

    /// Multi-sig emergency cancellation — propose or approve (issue #1119).
    ///
    /// Single-signer admin cancellation already exists via `cancel_pool`,
    /// but for *emergency* cancellations of pools that already hold
    /// non-trivial stake we want sign-off from more than one privileged
    /// address. Each call by a distinct admin/operator address records an
    /// approval; once `EMERGENCY_CANCEL_MULTISIG_THRESHOLD` distinct
    /// approvals are collected, the pool is moved to `Canceled` exactly
    /// like the single-signer path.
    ///
    /// # Caller
    /// Any address with role 0 (admin) or 1 (operator).
    ///
    /// # Errors
    /// - `Unauthorized` if caller lacks admin/operator role.
    /// - `PoolNotFound` if `pool_id` does not exist.
    /// - `InvalidPoolState` if the pool is not `Active`.
    /// - `EmergencyCancelAlreadyApproved` if the caller has already approved.
    /// - `EmergencyCancelPending` is *not* returned — pending state is
    ///   visible via `get_emergency_cancel_approvals`.
    pub fn emergency_cancel_pool(
        env: Env,
        approver: Address,
        pool_id: u64,
        reason: String,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        approver.require_auth();

        // Only admin (0) or operator (1) may participate in emergency cancel.
        let is_privileged = Self::require_role(&env, &approver, 0).is_ok()
            || Self::require_role(&env, &approver, 1).is_ok();
        if !is_privileged {
            return Err(PredifiError::Unauthorized);
        }

        Self::enter_reentrancy_guard(&env);

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = match env.storage().persistent().get(&pool_key) {
            Some(p) => p,
            None => {
                Self::exit_reentrancy_guard(&env);
                return Err(PredifiError::PoolNotFound);
            }
        };
        Self::extend_persistent(&env, &pool_key);

        if !Self::is_pool_active(&pool) {
            Self::exit_reentrancy_guard(&env);
            return Err(PredifiError::InvalidPoolState);
        }

        // Load existing approvers (or start fresh).
        let approvers_key = DataKey::EmergencyCancelApprovers(pool_id);
        let mut approvers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&approvers_key)
            .unwrap_or_else(|| Vec::new(&env));

        // Reject duplicate approval from the same address.
        for a in approvers.iter() {
            if a == approver {
                Self::exit_reentrancy_guard(&env);
                return Err(PredifiError::EmergencyCancelAlreadyApproved);
            }
        }

        approvers.push_back(approver.clone());
        env.storage().persistent().set(&approvers_key, &approvers);

        // Capture the first reason; subsequent approvers reaffirm by
        // approving, but the recorded reason is the proposer's.
        let reason_key = DataKey::EmergencyCancelReason(pool_id);
        if !env.storage().persistent().has(&reason_key) {
            env.storage().persistent().set(&reason_key, &reason);
        }

        // Below threshold → still pending; surface state but do not cancel.
        if approvers.len() < EMERGENCY_CANCEL_MULTISIG_THRESHOLD {
            Self::exit_reentrancy_guard(&env);
            return Ok(());
        }

        // Threshold reached — cancel the pool exactly like cancel_pool would.
        let stored_reason: String = env
            .storage()
            .persistent()
            .get(&reason_key)
            .unwrap_or(reason);

        pool.state = MarketState::Canceled;
        env.storage().persistent().set(&pool_key, &pool);
        Self::bump_ttl(&env, &pool_key);
        Self::remove_from_active_index(&env, pool_id);

        PoolCanceledEvent {
            pool_id,
            caller: approver.clone(),
            reason: stored_reason,
            operator: approver,
        }
        .publish(&env);

        // Cleanup pending multisig state once the cancel has executed.
        env.storage().persistent().remove(&approvers_key);
        env.storage().persistent().remove(&reason_key);

        Self::exit_reentrancy_guard(&env);
        Ok(())
    }

    /// Read-only view of pending emergency-cancel approvers for `pool_id`.
    /// Returns an empty vec when no proposal is currently pending or once
    /// the threshold has been met and the pool has been cancelled (the
    /// approvers set is cleared on execution).
    pub fn get_emergency_cancel_approvals(env: Env, pool_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::EmergencyCancelApprovers(pool_id))
            .unwrap_or_else(|| Vec::new(&env))
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
        Self::require_not_paused(&env)?;
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

        // Validate new stake limits before applying
        Self::validate_stake_limits(&env, &pool, min_stake, max_stake)?;

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
            start_time: pool.start_time,
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
    /// Prefers the batch `OutStakes` key (single read for all outcomes). Falls
    /// back to the legacy per-outcome `OutStake` key for pre-migration data.
    pub fn get_outcome_stake(env: Env, pool_id: u64, outcome: u32) -> i128 {
        // Prefer batch key — canonical after gas optimization (no dual-write)
        let batch_key = DataKey::OutStakes(pool_id);
        if let Some(stakes) = env.storage().persistent().get::<_, Vec<i128>>(&batch_key) {
            Self::extend_persistent(&env, &batch_key);
            return stakes.get(outcome).unwrap_or(0);
        }

        // Legacy fallback: individual key
        let stake_key = DataKey::OutStake(pool_id, outcome);
        if let Some(stake) = env.storage().persistent().get::<_, i128>(&stake_key) {
            Self::extend_persistent(&env, &stake_key);
            return stake;
        }

        0
    }

    /// Get a paginated list of pool IDs by category.
    ///
    /// # Errors
    /// Returns `PredifiError::InvalidPagination` if `offset + limit` overflows `u32`.
    pub fn get_pools_by_category(
        env: Env,
        category: Symbol,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<u64>, PredifiError> {
        // Guard against offset + limit wrapping around u32::MAX.
        offset
            .checked_add(limit)
            .ok_or(PredifiError::InvalidPagination)?;

        let count_key = DataKey::CatPoolCt(category.clone());
        let count: u32 = if let Some(c) = env.storage().persistent().get(&count_key) {
            Self::extend_persistent(&env, &count_key);
            c
        } else {
            0
        };

        let mut results = Vec::new(&env);

        if offset >= count || limit == 0 {
            return Ok(results);
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

        Ok(results)
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
    /// # Errors
    /// Returns `PredifiError::InvalidPagination` if `offset + limit` overflows `u32`.
    pub fn get_active_pools(env: Env, offset: u32, limit: u32) -> Result<Vec<u64>, PredifiError> {
        // Guard against offset + limit wrapping around u32::MAX.
        let end_check = offset
            .checked_add(limit)
            .ok_or(PredifiError::InvalidPagination)?;

        let ctr_key = DataKey::ActivePoolCtr;
        let count: u32 = env.storage().persistent().get(&ctr_key).unwrap_or(0);
        let mut results = Vec::new(&env);

        if offset >= count || limit == 0 {
            return Ok(results);
        }

        // Only extend the counter TTL if the key actually exists.
        if count > 0 {
            Self::extend_persistent(&env, &ctr_key);
        }

        let end = core::cmp::min(end_check, count);

        for i in offset..end {
            let slot_key = DataKey::ActivePool(i);
            if let Some(pool_id) = env.storage().persistent().get(&slot_key) {
                Self::extend_persistent(&env, &slot_key);
                results.push_back(pool_id);
            }
        }

        Ok(results)
    }

    /// Return the total number of currently active (open) pools.
    ///
    /// This is an O(1) read of the `ActivePoolCtr` persistent-storage counter
    /// that is maintained by `add_to_active_index` / `remove_from_active_index`.
    /// Frontends can use this to display "Showing N of M active pools" without
    /// fetching every page.
    pub fn get_active_pools_count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ActivePoolCtr)
            .unwrap_or(0)
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
    ///
    /// Gas notes: uses the in-pool `participants_count` and computes odds from the
    /// already-loaded stakes vec via [`gas_opt::odds_from_stakes`].
    pub fn get_pool_stats(env: Env, pool_id: u64) -> PoolStats {
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        let stakes = Self::get_outcome_stakes(&env, pool_id, pool.options_count);
        let current_odds = gas_opt::odds_from_stakes(&env, &stakes, pool.total_stake);

        PoolStats {
            pool_id,
            total_stake: pool.total_stake,
            stakes_per_outcome: stakes,
            participants_count: pool.participants_count,
            current_odds,
        }
    }

    /// Flag a pool as disputed. Only callable by a Moderator (role 2).
    ///
    /// Transitions the pool state from `Active` to `Disputed`, preventing
    /// further participation or resolution until the dispute is handled.
    ///
    /// # Errors
    /// - `Unauthorized` – caller does not hold the Moderator role.
    /// - `InvalidPoolState` – pool is not currently `Active`.
    pub fn flag_disputed_pool(
        env: Env,
        moderator: Address,
        pool_id: u64,
        reason: String,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        moderator.require_auth();
        Self::require_role(&env, &moderator, 2)?;

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        if pool.state != MarketState::Active {
            return Err(PredifiError::InvalidPoolState);
        }

        pool.state = MarketState::Disputed;
        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        env.storage()
            .persistent()
            .set(&DataKey::Disputed(pool_id), &());
        Self::extend_persistent(&env, &DataKey::Disputed(pool_id));

        PoolDisputedEvent {
            pool_id,
            moderator,
            reason,
        }
        .publish(&env);

        Ok(())
    }
}

// ── Domain types & events (moved from lib.rs, issue #1970) ─────────────────

/// Parameters for creating a new prediction pool.
///
/// This struct is used internally to validate and organize pool creation data.
/// All fields must pass validation before a pool can be created.
#[contracttype]
#[derive(Clone)]
pub struct CreatePoolParams {
    /// Unix timestamp at which the pool opens for predictions.
    pub start_time: u64,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pool {
    /// Unix timestamp at which the pool opens for predictions.
    pub start_time: u64,
    /// Unix timestamp after which no more predictions (stakes) are accepted.
    /// This defines the end of the "betting window". Must be > start_time.
    pub end_time: u64,
    /// Current operational state of the market.
    /// Possible values: `Active` (betting open), `Resolved` (result final), `Canceled` (refunds available).
    pub state: MarketState,
    /// The winning outcome index (0-based) after resolution.
    /// Only meaningful if `state` is `Resolved`.
    /// Uses UNRESOLVED_OUTCOME (u32::MAX) as sentinel for "not yet resolved".
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
    /// Unix timestamp when the pool was resolved. None for pools created before this feature.
    /// Used to enforce claim window expiration. Set when pool transitions to MarketState::Resolved.
    pub resolution_timestamp: Option<u64>,
}

/// Configuration parameters for creating a prediction pool.
///
/// This struct is passed to `create_pool` to define the pool's immutable (or near-immutable)
/// blueprint. It separates creation-time parameters from the runtime state managed in `Pool`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolConfig {
    /// Unix timestamp at which the pool opens for predictions.
    /// - `0` means the pool opens immediately upon creation.
    /// - Any positive value must be strictly in the future (greater than current ledger timestamp).
    /// - Must be less than `end_time`.
    /// - When non-zero, allows creators to schedule pool opening for a future time.
    /// - Example: `env.ledger().timestamp() + 3600` opens in 1 hour.
    pub start_time: u64,
    /// A short, human-readable title or question for the prediction market.
    /// - Maximum length: 256 bytes.
    /// - Should clearly state the prediction being made (e.g., "Will BTC exceed $100k by 2025?").
    /// - Displayed on pool cards and detail pages.
    /// - Example: "ETH/USD crossing $3500 this week?".
    pub description: String,
    /// A URL (typically IPFS) pointing to extended metadata, rules, or rich media.
    /// - Maximum length: 512 bytes.
    /// - Can point to JSON metadata, images, videos, or documentation.
    /// - Off-chain systems use this URL for pool discovery and enrichment.
    /// - Example: `"ipfs://QmHash..."` or `"https://api.example.com/metadata/123"`.
    pub metadata_url: String,
    /// Minimum amount a user must stake in a single prediction.
    /// - Must be greater than zero (`min_stake > 0`).
    /// - Users attempting to stake less will receive an error.
    /// - Can be used to prevent spam or micro-betting.
    /// - Example: `1000` requires at least 1000 tokens per prediction.
    pub min_stake: i128,
    /// Maximum amount a user can stake in a single prediction.
    /// - `0` means no per-user limit (unlimited betting).
    /// - If non-zero, must be greater than or equal to `min_stake`.
    /// - Protects against single-user dominance of a pool.
    /// - Example: `1000000` caps individual bets at 1 million tokens.
    pub max_stake: i128,
    /// Minimum `total_stake` required for the pool to be considered valid for resolution.
    /// - Must be greater than zero (`min_total_stake > 0`).
    /// - Ensures the pool has meaningful participation before a result is finalized.
    /// - Prevents resolution of low-liquidity pools that may not reflect true market sentiment.
    /// - Example: `10000` requires at least 10,000 total tokens staked.
    pub min_total_stake: i128,
    /// Hard cap on the `total_stake` the pool can accept.
    /// - `0` means no limit (unlimited total stake).
    /// - Must be greater than or equal to zero.
    /// - When combined with `initial_liquidity`, enforces the liquidity safety margin.
    /// - Protects creators from unlimited liability or excessive exposure.
    /// - Example: `10000000` caps total pool stake at 10 million tokens.
    pub max_total_stake: i128,
    /// Seed liquidity provided by the pool creator at initialization ("house money").
    /// - Typically excluded from fee calculations for fairness.
    /// - If `max_total_stake > 0` and `initial_liquidity > 0`:
    ///   `initial_liquidity >= max_total_stake * INITIAL_LIQUIDITY_SAFETY_MARGIN_BPS / 10_000`.
    /// - Safety margin prevents early high-value predictions from draining a thinly seeded pool.
    /// - Default safety margin: 10% (1000 basis points).
    /// - Example: `1000000` seeds the pool with 1 million tokens.
    pub initial_liquidity: i128,
    /// Number of independent oracle/operator resolutions required before the pool is finalized.
    /// - Must be at least 1 (`required_resolutions >= 1`).
    /// - Multi-resolution provides safety against single-oracle failure or manipulation.
    /// - When `operator_count > 0`, `required_resolutions <= operator_count`.
    /// - If `operator_count == 0`, oracle resolution is allowed (fallback).
    /// - Example: `2` requires two independent resolutions before finalization.
    pub required_resolutions: u32,
    /// If true, only whitelisted addresses can participate in this pool.
    /// - Whitelisted users: those in the access control whitelist.
    /// - Private pool users: those possessing the `whitelist_key`.
    /// - Public pools (`private: false`) allow any address to participate.
    /// - Example: `true` for exclusive prediction markets.
    pub private: bool,
    /// A unique symbol or secret used as an invite key for accessing private pools.
    /// - If `None`, no invite key is required (use access control whitelist only).
    /// - If `Some(symbol)`, users can join by proving knowledge of this key.
    /// - The key is validated via `validate_referral_code`.
    /// - Example: `Some(symbol_short!("SECRET_INVITE_KEY"))`.
    pub whitelist_key: Option<Symbol>,
    /// Human-readable labels for each outcome (length must equal `options_count`).
    /// - Each label must be between 1 and 128 bytes.
    ///- Labels are ordered: index 0 is the first outcome, index 1 is the second, etc.
    /// - Users bet on outcome indices (0, 1, 2, ...).
    /// - Example: `["No", "Yes"]` for binary, `["Team A", "Team B", "Draw"]` for 3 outcomes.
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

#[contractevent(topics = ["pool_ready"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolReadyForResolutionEvent {
    pub pool_id: u64,
    pub timestamp: u64,
}

/// Emitted exactly once per pool when its staking window closes, i.e. the
/// first time `close_staking` is successfully called after `pool.end_time`
/// has elapsed.
///
/// Off-chain subscribers (event indexers, front-ends, keepers) should watch
/// the `"staking_closed"` topic on this contract to react to the transition —
/// for example to hide a "Place Prediction" button or to queue a resolution
/// workflow.  The event is guaranteed to fire **at most once** per pool; the
/// `StakingClosed(pool_id)` storage sentinel prevents duplicate emission even
/// if multiple callers race to trigger the transition.
#[contractevent(topics = ["staking_closed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakingClosedEvent {
    /// The pool whose staking window has just closed.
    pub pool_id: u64,
    /// The pool's configured `end_time` — the boundary at which staking closed.
    pub end_time: u64,
    /// Total stake locked in the pool at the moment staking was closed.
    pub total_stake: i128,
    /// Ledger timestamp when this event was emitted.
    pub timestamp: u64,
}

#[contractevent(topics = ["pool_created"])]
#[contracttype(export = false)]
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
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialLiquidityProvidedEvent {
    pub pool_id: u64,
    pub creator: Address,
    pub amount: i128,
}

#[contractevent(topics = ["pool_resolved"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolResolvedEvent {
    pub pool_id: u64,
    pub operator: Address,
    pub outcome: u32,
}

#[contractevent(topics = ["pool_canceled"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolCanceledEvent {
    pub pool_id: u64,
    pub caller: Address,
    pub reason: String,
    pub operator: Address,
}

#[contractevent(topics = ["pool_disputed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolDisputedEvent {
    pub pool_id: u64,
    pub moderator: Address,
    pub reason: String,
}

#[contractevent(topics = ["stake_limits_updated"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeLimitsUpdatedEvent {
    pub pool_id: u64,
    pub operator: Address,
    pub min_stake: i128,
    pub max_stake: i128,
}

#[contractevent(topics = ["pool_description_updated"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolDescriptionUpdatedEvent {
    pub pool_id: u64,
    pub caller: Address,
    pub new_description: String,
}

/// Emitted when `increase_max_total_stake` successfully raises the stake cap
/// for a pool. Useful for frontends that display the current pool capacity.
#[contractevent(topics = ["max_stake_increased"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxTotalStakeIncreasedEvent {
    /// The pool whose cap was increased.
    pub pool_id: u64,
    /// The creator/caller that raised the cap.
    pub creator: Address,
    /// The new `max_total_stake` value (0 = unlimited).
    pub new_max_total_stake: i128,
}

