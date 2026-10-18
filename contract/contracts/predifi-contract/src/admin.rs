//! # Admin Operations & System Governance
//!
//! This module handles administrative operations, pause control, protocol fee structures,
//! token whitelists, timelocked governance changes, private pool access control, and contract upgrades.
//!
//! ## Admin Roles and Access Control Summary
//!
//! The contract uses role-based access control (RBAC). Admin operations require authorization
//! from either a caller holding the **Admin role (role index 0)** or, for pool-specific whitelist operations,
//! authorization from the **Pool Creator**.
//!
//! ### Admin-Only Operations (Requires Admin Role 0)
//! An unauthorized caller (lacking Admin role 0) attempting these functions receives `PredifiError::Unauthorized`
//! or a panic (`"Unauthorized: missing required role"`).
//!
//! - **`pause(env, admin)`**: Emergency pause contract execution.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: Panic (`"Unauthorized: missing required role"`)
//! - **`unpause(env, admin)`**: Resume contract execution.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: Panic (`"Unauthorized: missing required role"`)
//! - **`set_fee_bps(env, admin, fee_bps)`**: Queue a fee change proposal subject to a 7-day timelock.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`apply_fee_bps(env, admin)`**: Apply a queued fee change once timelock expires.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`cancel_fee_proposal(env, admin)`**: Cancel a pending fee change proposal.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`set_max_predictions_per_user(env, admin, limit)`**: Set maximum predictions allowed per user.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`set_prediction_cooldown(env, admin, cooldown_seconds)`**: Set minimum cooldown between predictions.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`set_resolution_delay(env, admin, delay)`**: Set resolution delay cap in seconds.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`set_claim_window(env, admin, claim_window_seconds)`**: Set claim window duration.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`set_min_pool_duration(env, admin, duration)`**: Set minimum pool duration requirement.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`set_min_stake(env, admin, amount)`**: Set global minimum stake requirement.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`add_token_to_whitelist(env, admin, token)`**: Add token to allowed betting whitelist.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`remove_token_from_whitelist(env, admin, token)`**: Remove token from allowed betting whitelist.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`batch_add_tokens_to_whitelist(env, admin, tokens)`**: Batch add tokens to whitelist.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`batch_remove_tokens_whitelist(env, admin, tokens)`**: Batch remove tokens from whitelist.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`upgrade_contract(env, admin, new_wasm_hash)`**: Upgrade contract Wasm code.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`migrate_state(env, admin)`**: Execute post-upgrade state migration.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`set_fee_tiers(env, admin, tiers)`**: Set dynamic fee tier thresholds.
//!   - *Role Required*: Admin (0)
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//!
//! ### Pool-Creator Operations (Requires Pool Creator Authentication)
//! An unauthorized caller (not matching `pool.creator`) attempting these functions receives `PredifiError::Unauthorized`.
//!
//! - **`add_to_whitelist(env, creator, pool_id, user)`**: Add a user to a private pool's whitelist.
//!   - *Role Required*: Pool Creator
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`remove_from_whitelist(env, creator, pool_id, user)`**: Remove a user from a private pool's whitelist.
//!   - *Role Required*: Pool Creator
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`batch_add_to_whitelist(env, creator, pool_id, users)`**: Batch add users to a private pool's whitelist.
//!   - *Role Required*: Pool Creator
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//! - **`batch_remove_from_whitelist(env, creator, pool_id, users)`**: Batch remove users from a private pool's whitelist.
//!   - *Role Required*: Pool Creator
//!   - *Error on Unauthorized*: `PredifiError::Unauthorized`
//!
//! ### Permissionless / Public Queries & Utilities (No Authorization Required)
//!
//! - **`init(env, access_control, treasury, fee_bps, resolution_delay, min_pool_duration, max_predictions_per_user)`**: Initialize contract instance.
//! - **`is_contract_paused(env)`**: Query contract pause state (`bool`).
//! - **`get_version(env)`**: Query numeric version code (`u32`).
//! - **`get_version_string(env)`**: Query semantic version string symbol (`Symbol`).
//! - **`get_pending_fee_change(env)`**: Query timelocked fee proposal (`Option<PendingFeeChange>`).
//! - **`get_supported_tokens(env)`**: Query list of whitelisted token addresses (`Vec<Address>`).
//! - **`is_token_allowed(env, token)`**: Check if a token is whitelisted (`bool`).
//! - **`get_fees(env)`**: Query fee info (`FeeInfo`).
//! - **`get_prediction_cooldown(env)`**: Query prediction cooldown seconds (`u64`).
//! - **`get_contract_info(env)`**: Query aggregated contract info (`ContractInfo`).
//! - **`is_whitelisted(env, pool_id, user)`**: Query pool whitelist status (`bool`).
//! - **`batch_check_whitelist(env, pool_id, users)`**: Query whitelist status for multiple users (`Result<Vec<bool>, PredifiError>`).
//! - **`get_fee_tiers(env)`**: Query configured fee tiers (`Vec<FeeTier>`).
//! - **`renew_storage_ttl(env, pool_id)`**: Permissionless storage TTL maintenance (`Result<(), PredifiError>`).
//! - **`get_contract_metadata(env)`**: Query comprehensive contract metadata (`ContractMetadata`).

use soroban_sdk::{Address, BytesN, contractevent, contractimpl, contracttype, Env, Symbol, Vec};

use crate::{
    ClaimWindowUpdateEvent,
    ContractPausedAlertEvent,
    DataKey,
    Pool,
    PredifiContract,
    PredifiContractArgs,
    PredifiContractClient,
    PredifiError,
    CONTRACT_VERSION,
    DEFAULT_GLOBAL_MIN_STAKE,
    DEFAULT_PREDICTION_COOLDOWN_SECONDS,
    FEE_CHANGE_TIMELOCK_SECONDS,
    MAX_CLAIM_WINDOW,
    MAX_RESOLUTION_DELAY,
    MIN_CLAIM_WINDOW,
};

#[contractimpl]
impl PredifiContract {
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
        if env.storage().instance().has(&DataKey::Config) {
            soroban_sdk::panic_with_error!(&env, PredifiError::AlreadyInitializedOrConfigNotSet);
        }

        // Enforce the same 30-day cap on resolution_delay that
        // set_resolution_delay enforces, so the contract cannot be
        // initialised with an unbounded delay.
        if resolution_delay > MAX_RESOLUTION_DELAY {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidData);
        }

        // Validate fee_bps on init — consistent with set_fee_bps (INV-6)
        if !Self::is_valid_fee_bps(fee_bps) {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidFeeBps);
        }

        let config = Config {
            fee_bps,
            treasury: treasury.clone(),
            access_control: access_control.clone(),
            resolution_delay,
            min_pool_duration,
            min_stake: DEFAULT_GLOBAL_MIN_STAKE,
            max_predictions_per_user,
            prediction_cooldown_seconds: DEFAULT_PREDICTION_COOLDOWN_SECONDS,
            referral_bps: 5000,              // default 50%
            claim_window_seconds: 2_592_000, // default 30 days
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

    /// Pause the contract.
    ///
    /// # Authorization
    /// Requires authentication from a caller with the Admin role.
    ///
    /// # Effects
    /// - Marks the contract as paused.
    /// - Emits `ContractPausedAlertEvent`.
    /// - Emits `PauseEvent`.
    ///
    /// While paused, administrative checks continue to work, but
    /// state-changing operations guarded by the pause flag are rejected.
    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        if Self::require_admin_role(&env, &admin, "pause").is_err() {
            panic!("Unauthorized: missing required role");
        }
        if Self::is_paused(&env) {
            panic!("Contract already paused");
        }
        env.storage().instance().set(&DataKey::Paused, &true);
        Self::extend_instance(&env);

        ContractPausedAlertEvent {
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        PauseEvent { admin }.publish(&env);
    }

    /// Resume normal contract operation.
    ///
    /// # Authorization
    /// Requires authentication from a caller with the Admin role.
    ///
    /// # Effects
    /// - Clears the paused state.
    /// - Emits `UnpauseEvent`.
    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        if Self::require_admin_role(&env, &admin, "unpause").is_err() {
            panic!("Unauthorized: missing required role");
        }
        if !Self::is_paused(&env) {
            panic!("Contract already unpaused");
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

    /// Return the contract version stored in instance storage.
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

    /// Queue a fee change proposal subject to a [`FEE_CHANGE_TIMELOCK_SECONDS`]-second
    /// delay. The new fee does **not** take effect immediately; the admin must call
    /// [`Self::apply_fee_bps`] once the delay has elapsed.
    ///
    /// # Errors
    /// - [`PredifiError::ContractPaused`]   – contract is paused.
    /// - [`PredifiError::Unauthorized`]     – caller lacks Admin role (0).
    /// - [`PredifiError::InvalidFeeBps`]    – `fee_bps > 10_000`.
    /// - [`PredifiError::FeeChangePending`] – a proposal is already queued;
    ///   call `cancel_fee_proposal` first.
    ///
    /// PRE:  admin has role 0; no pending fee proposal exists.
    /// POST: A [`PendingFeeChange`] is stored with
    ///       `effective_at = now + FEE_CHANGE_TIMELOCK_SECONDS`.
    pub fn set_fee_bps(env: Env, admin: Address, fee_bps: u32) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_fee_bps")?;
        if !Self::is_valid_fee_bps(fee_bps) {
            return Err(PredifiError::InvalidFeeBps);
        }
        // Only one proposal may be pending at a time.
        if env.storage().instance().has(&DataKey::PendingFeeBps) {
            return Err(PredifiError::FeeChangePending);
        }
        let effective_at = env.ledger().timestamp() + FEE_CHANGE_TIMELOCK_SECONDS;
        let pending = PendingFeeChange {
            new_fee_bps: fee_bps,
            effective_at,
            proposed_by: admin.clone(),
        };
        env.storage()
            .instance()
            .set(&DataKey::PendingFeeBps, &pending);
        Self::extend_instance(&env);

        FeeChangeProposeEvent {
            admin,
            new_fee_bps: fee_bps,
            effective_at,
        }
        .publish(&env);
        Ok(())
    }

    /// Apply a pending fee change once the [`FEE_CHANGE_TIMELOCK_SECONDS`]-second
    /// delay has elapsed.
    ///
    /// Emits a [`FeeUpdateEvent`] with the newly committed fee value.
    ///
    /// # Errors
    /// - [`PredifiError::ContractPaused`]      – contract is paused.
    /// - [`PredifiError::Unauthorized`]        – caller lacks Admin role (0).
    /// - [`PredifiError::NoFeeChangePending`]  – no proposal is queued.
    /// - [`PredifiError::TimelockNotExpired`]  – the delay has not yet elapsed.
    ///
    /// PRE:  a [`PendingFeeChange`] exists and `now >= effective_at`.
    /// POST: `Config.fee_bps` is updated; the pending proposal is removed.
    pub fn apply_fee_bps(env: Env, admin: Address) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "apply_fee_bps")?;

        let pending: PendingFeeChange = env
            .storage()
            .instance()
            .get(&DataKey::PendingFeeBps)
            .ok_or(PredifiError::NoFeeChangePending)?;

        if env.ledger().timestamp() < pending.effective_at {
            return Err(PredifiError::TimelockNotExpired);
        }

        let mut config = Self::get_config(&env);
        config.fee_bps = pending.new_fee_bps;
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().remove(&DataKey::PendingFeeBps);
        Self::extend_instance(&env);

        FeeUpdateEvent {
            admin,
            fee_bps: pending.new_fee_bps,
        }
        .publish(&env);
        Ok(())
    }

    /// Cancel a pending fee change proposal before it is applied.
    ///
    /// Emits a [`FeeChangeCancelEvent`]. The current `Config.fee_bps` is unchanged.
    ///
    /// # Errors
    /// - [`PredifiError::ContractPaused`]     – contract is paused.
    /// - [`PredifiError::Unauthorized`]       – caller lacks Admin role (0).
    /// - [`PredifiError::NoFeeChangePending`] – no proposal is queued.
    ///
    /// PRE:  a [`PendingFeeChange`] exists.
    /// POST: the pending proposal is removed; `Config.fee_bps` is unmodified.
    pub fn cancel_fee_proposal(env: Env, admin: Address) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "cancel_fee_proposal")?;

        if !env.storage().instance().has(&DataKey::PendingFeeBps) {
            return Err(PredifiError::NoFeeChangePending);
        }

        env.storage().instance().remove(&DataKey::PendingFeeBps);
        Self::extend_instance(&env);

        FeeChangeCancelEvent { admin }.publish(&env);
        Ok(())
    }

    /// Return the currently pending fee change proposal, or `None` if no proposal
    /// is queued.
    ///
    /// Clients should poll this before calling [`Self::apply_fee_bps`] to confirm
    /// a proposal exists and to read the `effective_at` timestamp.
    pub fn get_pending_fee_change(env: Env) -> Option<PendingFeeChange> {
        env.storage().instance().get(&DataKey::PendingFeeBps)
    }

    /// Set maximum predictions per user. Caller must have Admin role (0).
    /// PRE: admin has role 0
    /// POST: Config.max_predictions_per_user >= 0 (0 = no limit)
    pub fn set_max_predictions_per_user(
        env: Env,
        admin: Address,
        limit: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_max_predictions_per_user")?;
        let mut config = Self::get_config(&env);
        config.max_predictions_per_user = limit;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        MaxPredictionsUpdateEvent { admin, limit }.publish(&env);
        Ok(())
    }

    /// Set the cooldown in seconds between consecutive predictions from the same address.
    pub fn set_prediction_cooldown(
        env: Env,
        admin: Address,
        cooldown_seconds: u64,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_prediction_cooldown")?;

        let mut config = Self::get_config(&env);
        config.prediction_cooldown_seconds = cooldown_seconds;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        PredictionCooldownUpdateEvent {
            admin,
            cooldown_seconds,
        }
        .publish(&env);
        Ok(())
    }

    /// Set resolution delay in seconds. Caller must have Admin role (0).
    pub fn set_resolution_delay(env: Env, admin: Address, delay: u64) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_resolution_delay")?;
        if delay > MAX_RESOLUTION_DELAY {
            return Err(PredifiError::InvalidData);
        }
        let mut config = Self::get_config(&env);
        config.resolution_delay = delay;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        ResolutionDelayUpdateEvent { admin, delay }.publish(&env);
        Ok(())
    }

    /// Set claim window in seconds. Caller must have Admin role (0).
    ///
    /// The claim window defines how long after pool resolution users can claim winnings.
    /// Must be between MIN_CLAIM_WINDOW (1 day) and MAX_CLAIM_WINDOW (365 days).
    ///
    /// # Arguments
    /// * `admin` - Address with Admin role (0).
    /// * `claim_window_seconds` - Claim window duration in seconds.
    ///
    /// # Errors
    /// * `PredifiError::InvalidData` if claim_window_seconds is outside allowed range
    /// * `PredifiError::Unauthorized` if caller doesn't have Admin role
    pub fn set_claim_window(
        env: Env,
        admin: Address,
        claim_window_seconds: u64,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_claim_window")?;

        if !(MIN_CLAIM_WINDOW..=MAX_CLAIM_WINDOW).contains(&claim_window_seconds) {
            return Err(PredifiError::InvalidData);
        }

        let mut config = Self::get_config(&env);
        config.claim_window_seconds = claim_window_seconds;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        ClaimWindowUpdateEvent {
            admin,
            claim_window_seconds,
        }
        .publish(&env);

        Ok(())
    }

    /// Set minimum pool duration in seconds. Caller must have Admin role (0).
    pub fn set_min_pool_duration(
        env: Env,
        admin: Address,
        duration: u64,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
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
        Self::require_not_paused(&env)?;
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

    /// Add a token to the allowed betting whitelist. Caller must have Admin role (0).
    pub fn add_token_to_whitelist(
        env: Env,
        admin: Address,
        token: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
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
        Self::require_not_paused(&env)?;
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

    /// Batch add multiple tokens to the allowed betting whitelist in a single
    /// transaction. Caller must have Admin role (0).
    ///
    /// Reduces per-token transaction overhead when onboarding several tokens
    /// at once. Skips tokens that are already whitelisted.
    ///
    /// # Errors
    /// * `Unauthorized` - If caller does not hold the Admin role.
    /// * `InvalidData` - If `tokens` is empty or exceeds `MAX_BATCH_SIZE` (100).
    pub fn batch_add_tokens_to_whitelist(
        env: Env,
        admin: Address,
        tokens: Vec<Address>,
    ) -> Result<u32, PredifiError> {
        const MAX_BATCH_SIZE: u32 = 100;

        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "batch_add_tokens_to_whitelist")?;

        let batch_size = tokens.len();
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(PredifiError::InvalidData);
        }

        let whitelist_key = DataKey::TokenWhitelist;
        let mut whitelist: Vec<Address> = env
            .storage()
            .persistent()
            .get(&whitelist_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut added_count: u32 = 0;
        for token in tokens.iter() {
            let key = DataKey::TokenWl(token.clone());
            let already_whitelisted = env.storage().persistent().has(&key);

            if !already_whitelisted {
                env.storage().persistent().set(&key, &true);
                Self::extend_persistent(&env, &key);

                if !whitelist.contains(&token) {
                    whitelist.push_back(token.clone());
                }

                added_count += 1;

                TokenWhitelistAddedEvent {
                    admin: admin.clone(),
                    token: token.clone(),
                }
                .publish(&env);
            }
        }

        if added_count > 0 {
            env.storage().persistent().set(&whitelist_key, &whitelist);
            Self::extend_persistent(&env, &whitelist_key);
        }

        Ok(added_count)
    }

    /// Batch remove multiple tokens from the allowed betting whitelist in a
    /// single transaction. Caller must have Admin role (0).
    ///
    /// # Errors
    /// * `Unauthorized` - If caller does not hold the Admin role.
    /// * `InvalidData` - If `tokens` is empty or exceeds `MAX_BATCH_SIZE` (100).
    pub fn batch_remove_tokens_whitelist(
        env: Env,
        admin: Address,
        tokens: Vec<Address>,
    ) -> Result<u32, PredifiError> {
        const MAX_BATCH_SIZE: u32 = 100;

        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "batch_remove_tokens_whitelist")?;

        let batch_size = tokens.len();
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(PredifiError::InvalidData);
        }

        let whitelist_key = DataKey::TokenWhitelist;
        let mut whitelist: Vec<Address> = env
            .storage()
            .persistent()
            .get(&whitelist_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut removed_count: u32 = 0;
        for token in tokens.iter() {
            let key = DataKey::TokenWl(token.clone());
            let was_whitelisted = env.storage().persistent().has(&key);

            if was_whitelisted {
                env.storage().persistent().remove(&key);
                removed_count += 1;

                TokenWhitelistRemovedEvent {
                    admin: admin.clone(),
                    token: token.clone(),
                }
                .publish(&env);
            }
        }

        if removed_count > 0 {
            let mut new_whitelist = Vec::new(&env);
            for t in whitelist.iter() {
                if !tokens.contains(&t) {
                    new_whitelist.push_back(t);
                }
            }
            whitelist = new_whitelist;

            env.storage().persistent().set(&whitelist_key, &whitelist);
            Self::extend_persistent(&env, &whitelist_key);
        }

        Ok(removed_count)
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

        let old_version: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or(0u32);
        let new_version = old_version + 1;

        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        env.storage()
            .instance()
            .set(&DataKey::Version, &new_version);
        Self::extend_instance(&env);

        UpgradeEvent {
            admin: admin.clone(),
            new_wasm_hash,
        }
        .publish(&env);

        ContractUpgradedEvent {
            old_version,
            new_version,
            upgraded_by: admin,
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
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "migrate_state")?;

        // v2 migration: Add any state migration logic here.
        // Use Self::validate_pool_invariants(&pool) to ensure pool data consistency
        // during migrations.

        Ok(())
    }

    /// Returns true if the given token is on the allowed betting whitelist.
    pub fn is_token_allowed(env: Env, token: Address) -> bool {
        Self::is_token_whitelisted(&env, &token)
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

    /// Returns the full fee configuration as a [`FeeConfig`]: base protocol fee,
    /// referral cut, treasury, dynamic fee tiers, any pending fee change and the
    /// fee-change timelock. Read-only; requires the contract to be initialised.
    pub fn get_fee_config(env: Env) -> FeeConfig {
        let config = Self::get_config(&env);
        let pending = Self::get_pending_fee_change(env.clone());
        FeeConfig {
            fee_bps: config.fee_bps,
            referral_cut_bps: Self::read_referral_cut_bps(&env),
            treasury: config.treasury,
            tiers: Self::get_fee_tiers(env.clone()),
            pending_fee_bps: pending.clone().map(|p| p.new_fee_bps),
            pending_fee_effective_at: pending.map(|p| p.effective_at),
            fee_change_timelock_seconds: FEE_CHANGE_TIMELOCK_SECONDS,
        }
    }

    /// Returns the current cooldown in seconds between consecutive predictions from the same address.
    pub fn get_prediction_cooldown(env: Env) -> u64 {
        Self::get_config(&env).prediction_cooldown_seconds
    }

    /// Return an aggregated metadata view of contract config and protocol state.
    pub fn get_contract_info(env: Env) -> ContractInfo {
        let config = Self::get_config(&env);
        // If the access-control contract cannot be reached, fall back to the
        // contract's own address so callers still receive a valid response.
        let current_admin = Self::get_access_control_admin(&env, &config.access_control)
            .unwrap_or_else(|_| env.current_contract_address());

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
            prediction_cooldown_seconds: config.prediction_cooldown_seconds,
        }
    }

    /// Add a user to a private pool's whitelist. Only callable by pool creator.
    pub fn add_to_whitelist(
        env: Env,
        creator: Address,
        pool_id: u64,
        user: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
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
            added_by: creator,
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
        Self::require_not_paused(&env)?;
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
            removed_by: creator,
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

    /// Batch add multiple users to a private pool's whitelist.
    ///
    /// This optimized function reduces storage operations by processing
    /// multiple users in a single transaction, improving gas efficiency.
    ///
    /// # Arguments
    /// * `creator` - The pool creator (must match pool.creator)
    /// * `pool_id` - The pool identifier
    /// * `users` - Vector of addresses to add to the whitelist
    ///
    /// # Returns
    /// Result indicating success or a PredifiError
    ///
    /// # Errors
    /// * `Unauthorized` - If caller is not the pool creator
    /// * `PoolNotFound` - If the pool doesn't exist
    /// * `InvalidPoolState` - If pool is not private
    /// * `InvalidData` - If users vector is empty or exceeds max batch size
    pub fn batch_add_to_whitelist(
        env: Env,
        creator: Address,
        pool_id: u64,
        users: Vec<Address>,
    ) -> Result<u32, PredifiError> {
        const MAX_BATCH_SIZE: u32 = 100;

        Self::require_not_paused(&env)?;
        creator.require_auth();

        // Validate batch size
        let batch_size = users.len();
        if batch_size == 0 {
            return Err(PredifiError::InvalidData);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(PredifiError::InvalidData);
        }

        // Load and validate pool
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .ok_or(PredifiError::PoolNotFound)?;
        Self::extend_persistent(&env, &pool_key);

        // Authorization check
        if pool.creator != creator {
            return Err(PredifiError::Unauthorized);
        }

        // Pool must be private
        if !pool.private {
            return Err(PredifiError::InvalidPoolState);
        }

        // Batch process: add all users to whitelist
        let mut added_count: u32 = 0;
        let timestamp = env.ledger().timestamp();

        for user in users.iter() {
            let whitelist_key = DataKey::Whitelist(pool_id, user.clone());

            // Only add if not already whitelisted
            let already_whitelisted = env
                .storage()
                .persistent()
                .get(&whitelist_key)
                .unwrap_or(false);

            if !already_whitelisted {
                env.storage().persistent().set(&whitelist_key, &true);
                Self::extend_persistent(&env, &whitelist_key);
                added_count += 1;

                // Emit event for each user
                AddedToWhitelistEvent {
                    pool_id,
                    user,
                    added_by: creator.clone(),
                    timestamp,
                }
                .publish(&env);
            }
        }

        Ok(added_count)
    }

    /// Batch remove multiple users from a private pool's whitelist.
    ///
    /// This optimized function reduces storage operations by processing
    /// multiple users in a single transaction, improving gas efficiency.
    ///
    /// # Arguments
    /// * `creator` - The pool creator (must match pool.creator)
    /// * `pool_id` - The pool identifier
    /// * `users` - Vector of addresses to remove from the whitelist
    ///
    /// # Returns
    /// Result indicating success or a PredifiError
    ///
    /// # Errors
    /// * `Unauthorized` - If caller is not the pool creator
    /// * `PoolNotFound` - If the pool doesn't exist
    /// * `InvalidPoolState` - If pool is not private
    /// * `InvalidData` - If users vector is empty or exceeds max batch size
    pub fn batch_remove_from_whitelist(
        env: Env,
        creator: Address,
        pool_id: u64,
        users: Vec<Address>,
    ) -> Result<u32, PredifiError> {
        const MAX_BATCH_SIZE: u32 = 100;

        Self::require_not_paused(&env)?;
        creator.require_auth();

        // Validate batch size
        let batch_size = users.len();
        if batch_size == 0 {
            return Err(PredifiError::InvalidData);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(PredifiError::InvalidData);
        }

        // Load and validate pool
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .ok_or(PredifiError::PoolNotFound)?;
        Self::extend_persistent(&env, &pool_key);

        // Authorization check
        if pool.creator != creator {
            return Err(PredifiError::Unauthorized);
        }

        // Pool must be private
        if !pool.private {
            return Err(PredifiError::InvalidPoolState);
        }

        // Batch process: remove all users from whitelist
        let mut removed_count: u32 = 0;
        let timestamp = env.ledger().timestamp();

        for user in users.iter() {
            let whitelist_key = DataKey::Whitelist(pool_id, user.clone());

            // Only remove if currently whitelisted
            if env.storage().persistent().has(&whitelist_key) {
                env.storage().persistent().remove(&whitelist_key);
                removed_count += 1;

                // Emit event for each user
                RemovedFromWhitelistEvent {
                    pool_id,
                    user,
                    removed_by: creator.clone(),
                    timestamp,
                }
                .publish(&env);
            }
        }

        Ok(removed_count)
    }

    /// Batch check whitelist status for multiple users.
    ///
    /// This optimized function checks whitelist status for multiple users
    /// in a single call, reducing RPC overhead for frontends.
    ///
    /// # Arguments
    /// * `pool_id` - The pool identifier
    /// * `users` - Vector of addresses to check
    ///
    /// # Returns
    /// Vector of boolean values indicating whitelist status for each user
    ///
    /// # Errors
    /// * `InvalidData` - If users vector is empty or exceeds max batch size
    pub fn batch_check_whitelist(
        env: Env,
        pool_id: u64,
        users: Vec<Address>,
    ) -> Result<Vec<bool>, PredifiError> {
        const MAX_BATCH_SIZE: u32 = 200;

        // Validate batch size
        let batch_size = users.len();
        if batch_size == 0 {
            return Err(PredifiError::InvalidData);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(PredifiError::InvalidData);
        }

        // Build result vector
        let mut results = Vec::new(&env);

        for user in users.iter() {
            let whitelist_key = DataKey::Whitelist(pool_id, user);
            let is_whitelisted = env
                .storage()
                .persistent()
                .get(&whitelist_key)
                .unwrap_or(false);

            // Extend TTL for accessed keys
            if env.storage().persistent().has(&whitelist_key) {
                Self::extend_persistent(&env, &whitelist_key);
            }

            results.push_back(is_whitelisted);
        }

        Ok(results)
    }

    pub fn set_fee_tiers(
        env: Env,
        admin: Address,
        tiers: Vec<FeeTier>,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_fee_tiers")?;

        for i in 0..tiers.len() {
            if let Some(tier) = tiers.get(i) {
                if tier.fee_bps > 10_000 {
                    return Err(PredifiError::InvalidFeeBps);
                }
                if i > 0 {
                    if let Some(prev) = tiers.get(i - 1) {
                        if tier.stake_threshold <= prev.stake_threshold {
                            return Err(PredifiError::InvalidFeeBps);
                        }
                    }
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

    // ── Issue #1125: Storage TTL renewal helper ───────────────────────────────

    /// Renew (bump) the storage TTL for all persistent entries associated with
    /// a pool, keeping them alive for another full `BUMP_AMOUNT` ledger period.
    ///
    /// This is a permissionless, read-only-effect helper that any party
    /// (keeper bot, front-end, pool participant) can call at any time to
    /// prevent pool data from expiring on-chain. It does **not** alter any
    /// pool state — it is a pure TTL maintenance operation.
    ///
    /// Entries renewed:
    /// - `Pool(pool_id)` — core pool struct
    /// - `OutStakes(pool_id)` — batch outcome stakes (if present)
    ///
    /// # Arguments
    /// * `pool_id` - The ID of the pool whose TTLs should be renewed.
    ///
    /// # Errors
    /// * `PoolNotFound` — no pool exists for `pool_id`.
    ///
    /// # Events
    /// Emits `StorageTtlRenewedEvent` so off-chain monitors can track
    /// renewal activity and verify keeper health.
    pub fn renew_storage_ttl(env: Env, pool_id: u64) -> Result<(), PredifiError> {
        let pool_key = DataKey::Pool(pool_id);

        // Verify the pool exists before attempting any TTL extension.
        if !env.storage().persistent().has(&pool_key) {
            return Err(PredifiError::PoolNotFound);
        }

        // Bump the pool struct entry.
        Self::bump_ttl(&env, &pool_key);

        // Bump batch outcome stakes entry if present.
        let stakes_key = DataKey::OutStakes(pool_id);
        if env.storage().persistent().has(&stakes_key) {
            Self::extend_persistent(&env, &stakes_key);
        }

        StorageTtlRenewedEvent {
            pool_id,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    // ── Issue #1137: Contract metadata getter ─────────────────────────────────

    /// Return a comprehensive metadata snapshot of the contract's current
    /// configuration and operational state.
    ///
    /// This is designed as a single-call alternative to individually querying
    /// `get_contract_info`, `get_fees`, `get_fee_tiers`, `get_oracle_config`,
    /// `get_active_pools_count`, and `get_referral_volume_threshold`.
    /// Front-ends and tooling can bootstrap their state from one RPC call.
    ///
    /// # Returns
    /// A `ContractMetadata` struct containing all protocol parameters.
    pub fn get_contract_metadata(env: Env) -> ContractMetadata {
        let config = Self::get_config(&env);
        let current_admin = Self::get_access_control_admin(&env, &config.access_control)
            .unwrap_or_else(|_| env.current_contract_address());

        let total_pools: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PoolIdCtr)
            .unwrap_or(0u64);

        let active_pools_count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::ActivePoolCtr)
            .unwrap_or(0u32);

        let fee_tiers = Self::get_fee_tiers(env.clone());
        let fee_tiers_count = fee_tiers.len();

        let oracle_initialized = env.storage().persistent().has(&DataKey::OracleConfig);

        let referral_min_volume: i128 = env
            .storage()
            .instance()
            .get(&DataKey::ReferralMinVolumeBps)
            .unwrap_or(0i128);

        ContractMetadata {
            version: env
                .storage()
                .instance()
                .get(&DataKey::Version)
                .unwrap_or(0u32),
            version_string: Symbol::new(&env, "0_0_0"),
            current_admin,
            is_paused: Self::is_paused(&env),
            total_pools,
            active_pools_count,
            fee_bps: config.fee_bps,
            referral_cut_bps: Self::read_referral_cut_bps(&env),
            referral_min_volume,
            treasury: config.treasury,
            access_control: config.access_control,
            resolution_delay: config.resolution_delay,
            min_pool_duration: config.min_pool_duration,
            min_stake: config.min_stake,
            max_predictions_per_user: config.max_predictions_per_user,
            prediction_cooldown_seconds: config.prediction_cooldown_seconds,
            fee_tiers_count,
            oracle_initialized,
        }
    }
}

// ── Domain types & events (moved from lib.rs, issue #1970) ─────────────────

/// Global protocol configuration.
///
/// Contains system-wide settings that control protocol behavior.
/// These settings can be updated by admin with appropriate governance.
///
/// # Invariants
/// - `fee_bps` must be <= 10,000 (100%) (INV-6)
/// - `max_predictions_per_user` must be >= 0 (0 = no limit)
/// - `referral_bps` must be <= 10,000 (100%)
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
    /// Minimum cooldown in seconds between consecutive predictions from the same address.
    pub prediction_cooldown_seconds: u64,
    /// Referral reward rate in basis points (1 bp = 0.01%). Valid range: 0-10,000.
    /// Represents the share of the protocol fee paid to referrers.
    /// Default: 500 (5%). Can be raised to 1000 (10%) for referral seasons.
    pub referral_bps: u32,
    /// Claim window duration in seconds after pool resolution.
    /// Users must claim winnings within this time period after resolution.
    /// Default: 2,592,000 seconds (30 days). Range: 86,400-31,536,000 (1-365 days).
    pub claim_window_seconds: u64,
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

/// Complete fee configuration returned by [`PredifiContract::get_fee_config`].
///
/// Lets a frontend show the fee a user will pay before staking without
/// hard-coding or inferring on-chain values.
///
/// - `fee_bps`: base protocol fee (bps) applied when no tier matches.
/// - `referral_cut_bps`: share of the protocol fee paid to referrers (bps).
/// - `treasury`: address receiving protocol fees.
/// - `tiers`: dynamic fee tiers ordered by ascending `stake_threshold`; the
///   highest tier whose threshold is <= a pool's total stake overrides `fee_bps`.
///   Empty when no tiers are configured.
/// - `pending_fee_bps` / `pending_fee_effective_at`: queued base-fee change and the
///   timestamp it becomes appliable, or `None` when no proposal is queued.
/// - `fee_change_timelock_seconds`: delay between proposing and applying a fee change.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub fee_bps: u32,
    pub referral_cut_bps: u32,
    pub treasury: Address,
    pub tiers: Vec<FeeTier>,
    pub pending_fee_bps: Option<u32>,
    pub pending_fee_effective_at: Option<u64>,
    pub fee_change_timelock_seconds: u64,
}

/// Snapshot of a pending protocol fee change awaiting timelock expiry.
///
/// Created when an admin calls [`PredifiContract::set_fee_bps`] and persisted in
/// instance storage until [`PredifiContract::apply_fee_bps`] commits the change
/// or [`PredifiContract::cancel_fee_proposal`] discards it.
///
/// Only one proposal may exist at a time; a second `set_fee_bps` call while this
/// record is present returns [`PredifiError::FeeChangePending`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingFeeChange {
    /// The proposed new protocol fee in basis points (1 bp = 0.01%).
    pub new_fee_bps: u32,
    /// Unix timestamp (seconds) at or after which `apply_fee_bps` may execute.
    pub effective_at: u64,
    /// The admin address that submitted this proposal.
    pub proposed_by: Address,
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
    /// Minimum cooldown in seconds between consecutive predictions from the same address.
    pub prediction_cooldown_seconds: u64,
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

/// Extended contract metadata for frontend/tooling consumption.
///
/// Combines all protocol configuration, version information, and operational
/// parameters into a single queryable structure via `get_contract_metadata()`.
/// Unlike `ContractInfo`, this also exposes oracle configuration, fee tiers
/// count, active pool count, and the referral volume threshold.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMetadata {
    /// Contract version stored in instance storage.
    pub version: u32,
    /// Human-readable semantic version string (e.g. `"0_0_0"`).
    pub version_string: Symbol,
    /// Admin address resolved from the access-control contract.
    pub current_admin: Address,
    /// Whether the contract is currently paused.
    pub is_paused: bool,
    /// Total number of pools ever created (monotonically increasing counter).
    pub total_pools: u64,
    /// Number of currently active (unresolved, uncanceled) pools.
    pub active_pools_count: u32,
    /// Protocol fee in basis points (1 bp = 0.01%).
    pub fee_bps: u32,
    /// Referral fee cut in basis points (share of protocol fee paid to referrer).
    pub referral_cut_bps: u32,
    /// Minimum referred volume (base token units) a referrer must accumulate
    /// in a pool before receiving a referral reward. 0 = no minimum.
    pub referral_min_volume: i128,
    /// Treasury address that receives protocol fees.
    pub treasury: Address,
    /// Access-control contract address.
    pub access_control: Address,
    /// Global resolution delay in seconds.
    pub resolution_delay: u64,
    /// Minimum pool duration in seconds.
    pub min_pool_duration: u64,
    /// Global minimum stake amount (base token units).
    pub min_stake: i128,
    /// Maximum predictions allowed per user per pool (0 = no limit).
    pub max_predictions_per_user: u32,
    /// Cooldown in seconds between consecutive predictions from the same address.
    pub prediction_cooldown_seconds: u64,
    /// Number of dynamic fee tiers currently configured.
    pub fee_tiers_count: u32,
    /// Whether oracle/price-feed resolution has been initialized.
    pub oracle_initialized: bool,
}

#[contractevent(topics = ["init"])]
#[contracttype(export = false)]
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
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseEvent {
    pub admin: Address,
}

#[contractevent(topics = ["unpause"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnpauseEvent {
    pub admin: Address,
}

#[contractevent(topics = ["fee_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeUpdateEvent {
    pub admin: Address,
    pub fee_bps: u32,
}

/// Emitted when an admin queues a fee change proposal via `set_fee_bps`.
/// The change does not take effect until `apply_fee_bps` is called at or after
/// `effective_at`.
#[contractevent(topics = ["fee_change_proposed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeChangeProposeEvent {
    /// Admin that submitted the proposal.
    pub admin: Address,
    /// The proposed new fee in basis points.
    pub new_fee_bps: u32,
    /// Unix timestamp at or after which the change may be applied.
    pub effective_at: u64,
}

/// Emitted when an admin cancels a pending fee change proposal via
/// `cancel_fee_proposal` before it has been applied.
#[contractevent(topics = ["fee_change_canceled"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeChangeCancelEvent {
    /// Admin that cancelled the proposal.
    pub admin: Address,
}

#[contractevent(topics = ["max_predictions_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxPredictionsUpdateEvent {
    pub admin: Address,
    pub limit: u32,
}

#[contractevent(topics = ["prediction_cooldown_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionCooldownUpdateEvent {
    pub admin: Address,
    pub cooldown_seconds: u64,
}

#[contractevent(topics = ["fee_tiers_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeTiersUpdateEvent {
    pub admin: Address,
    pub tiers_count: u32,
}

#[contractevent(topics = ["treasury_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryUpdateEvent {
    pub admin: Address,
    pub treasury: Address,
}

#[contractevent(topics = ["resolution_delay_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionDelayUpdateEvent {
    pub admin: Address,
    pub delay: u64,
}

#[contractevent(topics = ["min_pool_duration_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinPoolDurationUpdateEvent {
    pub admin: Address,
    pub duration: u64,
}

#[contractevent(topics = ["min_stake_update"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinStakeUpdateEvent {
    pub admin: Address,
    pub min_stake: i128,
}

#[contractevent(topics = ["token_whitelist_added"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenWhitelistAddedEvent {
    pub admin: Address,
    pub token: Address,
}

#[contractevent(topics = ["token_whitelist_removed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenWhitelistRemovedEvent {
    pub admin: Address,
    pub token: Address,
}

#[contractevent(topics = ["added_to_whitelist"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddedToWhitelistEvent {
    pub pool_id: u64,
    pub user: Address,
    pub added_by: Address,
    pub timestamp: u64,
}

#[contractevent(topics = ["removed_from_whitelist"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemovedFromWhitelistEvent {
    pub pool_id: u64,
    pub user: Address,
    pub removed_by: Address,
    pub timestamp: u64,
}

#[contractevent(topics = ["upgrade"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeEvent {
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
}

#[contractevent(topics = ["contract_upgraded"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUpgradedEvent {
    pub old_version: u32,
    pub new_version: u32,
    pub upgraded_by: Address,
}

/// Emitted when `renew_storage_ttl` is called to bump TTLs for pool storage
/// entries, keeping them alive for another full BUMP_AMOUNT period.
#[contractevent(topics = ["storage_ttl_renewed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTtlRenewedEvent {
    /// The pool whose storage TTLs were renewed.
    pub pool_id: u64,
    /// Ledger timestamp when the renewal was triggered.
    pub timestamp: u64,
}

