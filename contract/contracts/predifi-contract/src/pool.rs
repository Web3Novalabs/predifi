//! Pool domain: prediction pool lifecycle — creation, configuration,
//! resolution, cancellation and pool queries.

use soroban_sdk::{contractimpl, log, token, Address, Env, String, Symbol, Vec};

use crate::gas_opt;
use crate::{
    Config, DataKey, InitialLiquidityProvidedEvent, MarketState, MaxTotalStakeIncreasedEvent, Pool,
    PoolCanceledEvent, PoolConfig, PoolCreatedEvent, PoolDescriptionUpdatedEvent,
    PoolDisputedEvent, PoolReadyForResolutionEvent, PoolResolvedDiagEvent, PoolResolvedEvent,
    PoolStats, PredifiContract, PredifiContractArgs, PredifiContractClient, PredifiError,
    ResolutionConflictEvent, ResolutionVoteCastEvent, StakeLimitsUpdatedEvent, StakingClosedEvent,
    CANCELATION_DELAY, DEFAULT_MIN_POOL_DURATION, EMERGENCY_CANCEL_MULTISIG_THRESHOLD,
    INITIAL_LIQUIDITY_SAFETY_MARGIN_BPS, MAX_INITIAL_LIQUIDITY, MAX_OPTIONS_COUNT,
    MAX_POOL_DURATION, UNRESOLVED_OUTCOME,
};

#[contractimpl]
impl PredifiContract {
    /// Create a new prediction pool with configurable parameters.
    ///
    /// This function creates a new prediction market pool where users can stake tokens on
    /// specific outcomes. The pool is initialized in the `Active` state and immediately
    /// accepts predictions once the `start_time` is reached (or immediately if `start_time` is 0).
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment, providing access to storage, ledger, and auth
    /// - `creator` - The address creating the pool (must authenticate via `require_auth`)
    /// - `end_time` - Unix timestamp after which no more predictions are accepted (must be > current_time)
    /// - `token` - The Stellar token contract address used for staking (must be whitelisted)
    /// - `options_count` - Number of possible outcomes (must be >= 2 and <= MAX_OPTIONS_COUNT = 100)
    /// - `category` - Market category symbol (e.g., `CATEGORY_SPORTS`, `CATEGORY_FINANCE`)
    /// - `config` - Pool configuration parameters via `PoolConfig` struct:
    ///   - `start_time` - Unix timestamp when the pool opens for predictions (0 = open immediately)
    ///   - `description` - Short human-readable description (max 256 bytes)
    ///   - `metadata_url` - URL to extended metadata, e.g., IPFS link (max 512 bytes)
    ///   - `min_stake` - Minimum stake amount per prediction (must be > 0)
    ///   - `max_stake` - Maximum stake per prediction (0 = no limit, else must be >= min_stake)
    ///   - `min_total_stake` - Minimum total stake required for resolution (must be > 0)
    ///   - `max_total_stake` - Hard cap on total stake (0 = no limit)
    ///   - `initial_liquidity` - Optional house money provided by creator (must be >= 0)
    ///   - `required_resolutions` - Number of oracle/operator votes needed for resolution (must be >= 1)
    ///   - `private` - If true, only whitelisted addresses can participate
    ///   - `whitelist_key` - Optional symbol for private pool access
    ///   - `outcome_descriptions` - Human-readable labels for each outcome (length must equal options_count)
    ///
    /// # Return Value
    ///
    /// Returns `Ok(pool_id)` - The unique identifier of the newly created pool.
    /// The pool_id is auto-incremented and can be used to reference the pool in all subsequent operations.
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
    ///   - `outcome_descriptions` - Labels for each outcome
    ///
    /// 2. **`InitialLiquidityProvidedEvent`** (conditional):
    ///   - Emitted only if `initial_liquidity > 0`
    ///   - `pool_id` - The pool receiving liquidity
    ///   - `creator` - The address providing liquidity
    ///   - `amount` - The amount of liquidity transferred
    ///
    /// # Error Conditions
    ///
    /// The function can return the following errors:
    ///
    /// - `ContractPaused` - The contract is currently paused; all state-mutating operations are blocked
    /// - `Unauthorized` - Caller is not authorized (creator authentication failed)
    /// - `TokenNotWhitelisted` - The specified token is not on the allowed betting whitelist
    /// - `InvalidTimestamp` - `end_time` is not in the future, exceeds MAX_POOL_DURATION, or `end_time <= start_time`
    /// - `DeadlineInPast` - `end_time` or `start_time` is in the past (issue #1130)
    /// - `InvalidData` - `options_count` < 2 or > MAX_OPTIONS_COUNT
    /// - `MetadataUrlInvalid` - `metadata_url` exceeds 512 bytes
    /// - `InvalidTargetPrice` - Invalid target price (for price-based pools)
    /// - `InitialLiquidityBelowSafetyMargin` - Initial liquidity is insufficient relative to `max_total_stake` (issue #1131)
    /// - `RequiredResolutionsExceedOperators` - `required_resolutions` > number of active operators
    /// - `OutcomeDescriptionTooLong` - An outcome description exceeds MAX_OUTCOME_DESCRIPTION_LEN (128 bytes)
    /// - `OutcomeDescriptionEmpty` - An outcome description is empty or below MIN_OUTCOME_DESCRIPTION_LEN (1 byte)
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
    /// ```rust
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
    /// ```rust
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
    /// ```rust
    /// let config = PoolConfig {
    ///     // ... other fields ...
    ///     private: true,
    ///     whitelist_key: Some(symbol_short!("SECRET_KEY")),
    ///     // ... other fields ...
    /// };
    ///
    /// let pool_id = contract.create_pool(env, creator, end_time, token, 2, CATEGORY_FINANCE, config)?;
    /// // Only whitelisted users or those with the invite key can participate
    /// ```
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
        assert!(
            end_time >= current_time + min_pool_duration,
            "end_time must be at least min_pool_duration in the future"
        );

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
        // Validate: min_total_stake must be strictly positive (> 0)
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
            fee_bps: 0, // Will be set at resolution
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
    /// Resolution uses a **multi-vote / threshold model**: each address with Operator
    /// role (role `1`) calls this function once to cast a vote.  The pool transitions
    /// to [`MarketState::Resolved`] only when a single outcome accumulates at least
    /// `pool.required_resolutions` votes.  If operators disagree on the outcome a
    /// [`PredifiError::ResolutionConflict`] is returned and the pool stays active so
    /// administrators can intervene.
    ///
    /// # Parameters
    ///
    /// - `env` — The Soroban execution environment (ledger, storage, auth).
    /// - `operator` — The address casting a resolution vote; must hold Operator role (`1`)
    ///   and must sign the transaction via `require_auth`.
    /// - `pool_id` — The unique identifier of the pool to resolve.  The pool must exist
    ///   in persistent storage.
    /// - `outcome` — The 0-based index of the winning outcome the operator is voting for.
    ///   Must satisfy `0 <= outcome < pool.options_count` and must not equal
    ///   `UNRESOLVED_OUTCOME` (`u32::MAX`).
    ///
    /// # Resolution flow
    ///
    /// 1. **Auth & role check** — `operator` must sign the transaction and hold the
    ///    Operator role for `pool_id` (`require_operator_role_for_resolution`).
    /// 2. **State guard** — the pool must be in `MarketState::Active`; Locked,
    ///    Resolved, and Cancelled pools are rejected with [`PredifiError::InvalidPoolState`].
    /// 3. **Resolution delay** — `current_ledger_time >= pool.end_time + config.resolution_delay`
    ///    must hold.  This cooling-off period allows late price feeds to settle before
    ///    any outcome is locked in.  Violations return [`PredifiError::ResolutionDelayNotMet`].
    /// 4. **Outcome validation** — `outcome` must satisfy
    ///    `0 <= outcome < pool.options_count` and must not equal `UNRESOLVED_OUTCOME`
    ///    (the sentinel value reserved for pools that have not yet been decided).
    ///    Invalid values return [`PredifiError::InvalidOutcome`].
    /// 5. **Duplicate-vote guard** — each operator may vote exactly once per pool.
    ///    A second call from the same address returns [`PredifiError::OracleAlreadyVoted`].
    /// 6. **Vote recording** — the vote is stored in temporary ledger storage under
    ///    `DataKey::ResVote(pool_id, operator)`.  Per-outcome tallies are maintained
    ///    in `DataKey::ResVoteCt(pool_id, outcome)` and the total vote count in
    ///    `DataKey::ResTotal(pool_id)`.
    /// 7. **Conflict detection** — if any other outcome already has votes, a
    ///    [`ResolutionConflictEvent`] is emitted and [`PredifiError::ResolutionConflict`]
    ///    is returned, leaving the pool in `Active` so administrators can adjudicate.
    /// 8. **Threshold check & finalisation** — once `new_outcome_votes >= pool.required_resolutions`:
    ///    - `pool.state` is set to `MarketState::Resolved`.
    ///    - `pool.outcome` is recorded.
    ///    - A dynamic protocol fee (`pool.fee_bps`) is calculated via
    ///      [`Self::calculate_dynamic_fee`].
    ///    - `pool.resolution_timestamp` is stamped with the current ledger time.
    ///    - The pool is removed from the global active index.
    ///    - [`PoolResolvedEvent`] and [`PoolResolvedDiagEvent`] are published.
    ///
    /// # Payout calculation (post-resolution)
    ///
    /// This function does **not** distribute funds directly.  Winners call
    /// `claim_winnings` (or a payout helper) after resolution.  The payout for a
    /// winning stake `s` out of a total winning-side stake `W` and a total pool
    /// stake `T` is:
    ///
    /// ```text
    /// gross_payout = s * T / W
    /// fee          = gross_payout * pool.fee_bps / 10_000
    /// net_payout   = gross_payout - fee
    /// ```
    ///
    /// # Emitted events
    ///
    /// | Event | When |
    /// |---|---|
    /// | [`ResolutionVoteCastEvent`] | Every successful vote, regardless of threshold |
    /// | [`ResolutionConflictEvent`] | When a vote conflicts with an earlier outcome |
    /// | [`PoolResolvedEvent`] | When the threshold is reached and the pool is finalised |
    /// | [`PoolResolvedDiagEvent`] | Same trigger as `PoolResolvedEvent`; carries stake diagnostics |
    ///
    /// # Errors
    ///
    /// | Error | Condition |
    /// |---|---|
    /// | [`PredifiError::ContractPaused`] | The contract is globally paused |
    /// | [`PredifiError::InvalidPoolState`] | Pool is not `Active` |
    /// | [`PredifiError::ResolutionDelayNotMet`] | Too early to resolve |
    /// | [`PredifiError::InvalidOutcome`] | `outcome >= options_count` or equals sentinel |
    /// | [`PredifiError::OracleAlreadyVoted`] | This operator already voted for this pool |
    /// | [`PredifiError::ResolutionConflict`] | Operators disagree on the winning outcome |
    ///
    /// # Preconditions
    ///
    /// - `pool.state == MarketState::Active`
    /// - `operator` holds Operator role (`1`) for `pool_id`
    /// - `env.ledger().timestamp() >= pool.end_time + config.resolution_delay`
    ///
    /// # Postconditions (on threshold reached)
    ///
    /// - `pool.state == MarketState::Resolved` (INV-2 state transition satisfied)
    /// - `pool.outcome == outcome`
    /// - `pool.resolution_timestamp == Some(current_time)`
    /// - Pool is removed from the global active index
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
