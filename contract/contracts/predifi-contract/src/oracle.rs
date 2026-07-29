//! Oracle domain: oracle registration, price feeds, price conditions and
//! oracle-driven pool resolution.

use soroban_sdk::{contractimpl, log, Address, Env, String, Symbol, Vec};

use crate::{
    DataKey, MarketState, OracleCallback, OracleInitEvent, OracleResolvedEvent,
    OracleWhitelistAddedEvent, OracleWhitelistRemovedEvent, Pool, PoolResolvedDiagEvent,
    PoolResolvedEvent, PredifiContract, PredifiContractArgs, PredifiContractClient, PredifiError,
    PriceConditionSetEvent, PriceFeedUpdatedEvent, PriceFeedsCleanedEvent, ResolutionConflictEvent,
    ResolutionVoteCastEvent, MAX_PRICE_CONDITION_MATCH_STEPS, MAX_TOLERANCE, UNRESOLVED_OUTCOME,
};

#[contractimpl]
impl PredifiContract {
    /// Add an oracle address to the trusted oracle whitelist. Caller must have Admin role (0).
    pub fn add_oracle(
        env: Env,
        admin: Address,
        oracle_address: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "add_oracle")?;

        let key = DataKey::OracleWl(oracle_address.clone());
        env.storage().persistent().set(&key, &true);
        Self::extend_persistent(&env, &key);

        let list_key = DataKey::OracleWhitelist;
        let mut whitelist: Vec<Address> = env
            .storage()
            .persistent()
            .get(&list_key)
            .unwrap_or_else(|| Vec::new(&env));

        if !whitelist.contains(&oracle_address) {
            whitelist.push_back(oracle_address.clone());
            env.storage().persistent().set(&list_key, &whitelist);
            Self::extend_persistent(&env, &list_key);
        }

        OracleWhitelistAddedEvent {
            admin,
            oracle: oracle_address,
        }
        .publish(&env);

        Ok(())
    }

    /// Remove an oracle address from the trusted oracle whitelist. Caller must have Admin role (0).
    pub fn remove_oracle(
        env: Env,
        admin: Address,
        oracle_address: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "remove_oracle")?;

        let key = DataKey::OracleWl(oracle_address.clone());
        env.storage().persistent().remove(&key);

        let list_key = DataKey::OracleWhitelist;
        let whitelist: Vec<Address> = env
            .storage()
            .persistent()
            .get(&list_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut new_whitelist = Vec::new(&env);
        for oracle in whitelist.iter() {
            if oracle.clone() != oracle_address {
                new_whitelist.push_back(oracle);
            }
        }

        env.storage().persistent().set(&list_key, &new_whitelist);
        Self::extend_persistent(&env, &list_key);

        OracleWhitelistRemovedEvent {
            admin,
            oracle: oracle_address,
        }
        .publish(&env);

        Ok(())
    }

    /// Initialize the oracle configuration. Only callable by Admin (role 0).
    ///
    /// # Errors
    /// - `InvalidData`   – `max_price_age` is 0 (every feed would be immediately stale).
    /// - `InvalidFeeBps` – `min_confidence_ratio` exceeds 10 000 bps (100 %).
    pub fn init_oracle(
        env: Env,
        admin: Address,
        pyth_contract: Address,
        max_price_age: u64,
        min_confidence_ratio: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "init_oracle")?;

        if max_price_age == 0 {
            return Err(PredifiError::InvalidData);
        }
        if min_confidence_ratio > 10_000 {
            return Err(PredifiError::InvalidFeeBps);
        }

        env.storage().persistent().set(
            &DataKey::OracleConfig,
            &(pyth_contract.clone(), max_price_age, min_confidence_ratio),
        );
        Self::extend_persistent(&env, &DataKey::OracleConfig);

        OracleInitEvent {
            admin,
            pyth_contract,
            max_price_age,
            min_confidence_ratio,
        }
        .publish(&env);

        Ok(())
    }

    /// Return the current oracle configuration, if initialised.
    pub fn get_oracle_config(env: Env) -> Option<(Address, u64, u32)> {
        env.storage()
            .persistent()
            .get::<DataKey, (Address, u64, u32)>(&DataKey::OracleConfig)
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
        Self::require_not_paused(&env)?;
        operator.require_auth();
        Self::require_role(&env, &operator, 1)?; // Role Operator

        if target_price <= 0 {
            return Err(PredifiError::InvalidTargetPrice);
        }

        Self::validate_price_condition_match_params(operator_type, tolerance_bps)?;

        let pool_key = DataKey::Pool(pool_id);
        if !env.storage().persistent().has(&pool_key) {
            return Err(PredifiError::PoolNotFound);
        }

        let condition_key = DataKey::PriceCondition(pool_id);
        env.storage().persistent().set(
            &condition_key,
            &(
                feed_pair.clone(),
                target_price,
                operator_type,
                tolerance_bps,
            ),
        );
        Self::extend_persistent(&env, &condition_key);

        // Issue #1142: emit event for consistency with other setter functions.
        PriceConditionSetEvent {
            pool_id,
            feed_pair,
            target_price,
            operator: operator_type,
            tolerance_bps,
        }
        .publish(&env);

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
        Self::require_not_paused(&env)?;
        oracle.require_auth();

        if !Self::is_oracle_whitelisted(&env, &oracle) {
            return Err(PredifiError::Unauthorized);
        }

        // Security: reject prices with a future or current timestamp — the
        // timestamp must be strictly in the past to prevent oracle manipulation
        // via pre-dated or same-ledger price injections.
        if timestamp >= env.ledger().timestamp() {
            return Err(PredifiError::InvalidData);
        }

        // expires_at must be after the timestamp
        if expires_at <= timestamp {
            return Err(PredifiError::InvalidData);
        }

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
            list.push_back(feed_pair.clone());
            env.storage()
                .persistent()
                .set(&DataKey::PriceFeedList, &list);
        }

        // Emit event so off-chain monitors and indexers can track price updates.
        PriceFeedUpdatedEvent {
            oracle,
            feed_pair,
            price,
            confidence,
            timestamp,
            expires_at,
        }
        .publish(&env);

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
                env.storage().persistent().remove(&DataKey::PriceFeed(pair));
                removed += 1;
            } else {
                remaining.push_back(pair);
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::PriceFeedList, &remaining);

        PriceFeedsCleanedEvent {
            feeds_removed: removed,
            timestamp: current_time,
        }
        .publish(&env);

        removed
    }

    /// Load the current price and expiry timestamp for the configured feed.
    ///
    /// This intentionally preserves the previous missing-feed behavior:
    /// callers panic with "Feed not found" when the feed has not been updated.
    fn load_price_feed_for_resolution(env: &Env, feed_pair: Symbol) -> (i128, u64, u64) {
        let (price, _confidence, timestamp, expires_at): (i128, i128, u64, u64) = env
            .storage()
            .persistent()
            .get(&DataKey::PriceFeed(feed_pair))
            .expect("Feed not found");

        (price, timestamp, expires_at)
    }

    fn require_fresh_price_feed(
        env: &Env,
        timestamp: u64,
        expires_at: u64,
    ) -> Result<(), PredifiError> {
        let current_time = env.ledger().timestamp();

        // Check if price has expired
        if current_time > expires_at {
            return Err(PredifiError::PriceDataInvalid);
        }

        // Check if price is within the oracle's max_price_age limit
        let (_pyth_contract, max_price_age, _min_confidence_ratio) =
            Self::get_oracle_config(env.clone()).ok_or(PredifiError::OracleNotInitialized)?;

        if current_time > timestamp.saturating_add(max_price_age) {
            return Err(PredifiError::PriceDataInvalid);
        }

        Ok(())
    }

    fn load_price_resolution_condition(
        env: &Env,
        pool_id: u64,
    ) -> Result<(Symbol, i128, u32, u32), PredifiError> {
        let condition_key = DataKey::PriceCondition(pool_id);
        env.storage()
            .persistent()
            .get(&condition_key)
            .ok_or(PredifiError::PriceConditionNotSet)
    }

    fn validate_price_condition_match_params(
        comparison_op: u32,
        tolerance_bps: u32,
    ) -> Result<(), PredifiError> {
        if comparison_op > 2 || tolerance_bps > MAX_TOLERANCE {
            return Err(PredifiError::InvalidData);
        }

        Ok(())
    }

    fn price_tolerance_amount(
        target_price: i128,
        tolerance_bps: u32,
    ) -> Result<i128, PredifiError> {
        target_price
            .checked_mul(tolerance_bps as i128)
            .and_then(|amount| amount.checked_div(MAX_TOLERANCE as i128))
            .ok_or(PredifiError::ArithmeticError)
    }

    /// Match a stored price condition in a fixed, explicitly bounded number of checks.
    ///
    /// Operators: 0=Equal, 1=Greater, 2=Less. Outcome: 0=No, 1=Yes.
    fn price_resolution_outcome(
        price: i128,
        target_price: i128,
        comparison_op: u32,
        tolerance_bps: u32,
    ) -> Result<u32, PredifiError> {
        Self::validate_price_condition_match_params(comparison_op, tolerance_bps)?;

        let tolerance_amount = Self::price_tolerance_amount(target_price, tolerance_bps)?;
        let lower_bound = target_price
            .checked_sub(tolerance_amount)
            .ok_or(PredifiError::ArithmeticError)?;
        let upper_bound = target_price
            .checked_add(tolerance_amount)
            .ok_or(PredifiError::ArithmeticError)?;

        let mut steps = 0u32;
        let condition_met = match comparison_op {
            0 => {
                steps += 2;
                price >= lower_bound && price <= upper_bound
            }
            1 => {
                steps += 1;
                price > upper_bound
            }
            2 => {
                steps += 1;
                price < lower_bound
            }
            _ => return Err(PredifiError::InvalidData),
        };

        steps += 2;
        if steps > MAX_PRICE_CONDITION_MATCH_STEPS {
            return Err(PredifiError::RateLimitOrSuspiciousActivity);
        }

        Ok(if condition_met { 1 } else { 0 })
    }

    /// Load the pool and validate all non-outcome preconditions for price resolution.
    fn load_resolvable_price_pool(
        env: &Env,
        pool_id: u64,
    ) -> Result<(DataKey, Pool), PredifiError> {
        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        Self::validate_pool_invariants(&pool);

        if pool.state != MarketState::Active {
            return Err(PredifiError::InvalidPoolState);
        }

        let current_time = env.ledger().timestamp();
        let config = Self::get_config(env);

        if current_time < pool.end_time.saturating_add(config.resolution_delay) {
            return Err(PredifiError::ResolutionDelayNotMet);
        }

        Ok((pool_key, pool))
    }

    fn validate_price_resolution_outcome(
        env: &Env,
        pool_id: u64,
        outcome: u32,
        options_count: u32,
    ) -> Result<(), PredifiError> {
        if outcome >= options_count {
            log!(
                env,
                "resolve_pool_from_price rejected: outcome is out of bounds",
                pool_id,
                outcome,
                options_count
            );
            return Err(PredifiError::InvalidOutcome);
        }

        if outcome == UNRESOLVED_OUTCOME {
            log!(
                env,
                "resolve_pool_from_price rejected: outcome cannot be sentinel value",
                pool_id,
                outcome
            );
            return Err(PredifiError::InvalidOutcome);
        }

        Ok(())
    }

    fn persist_price_resolution(
        env: &Env,
        pool_key: &DataKey,
        pool_id: u64,
        mut pool: Pool,
        outcome: u32,
    ) {
        pool.state = MarketState::Resolved;
        pool.outcome = outcome;
        pool.fee_bps = Self::calculate_dynamic_fee(env, &pool);
        pool.resolution_timestamp = Some(env.ledger().timestamp()); // Record resolution time

        env.storage().persistent().set(pool_key, &pool);
        Self::bump_ttl(env, pool_key);

        PoolResolvedEvent {
            pool_id,
            operator: env.current_contract_address(),
            outcome,
        }
        .publish(env);
    }

    /// Automatically resolve a pool based on its configured price condition.
    /// Anyone can trigger this once the pool's end time and resolution delay have passed.
    pub fn resolve_pool_from_price(env: Env, pool_id: u64) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;

        let (feed_pair, target_price, comparison_op, tolerance_bps) =
            Self::load_price_resolution_condition(&env, pool_id)?;
        let (price, timestamp, expires_at) = Self::load_price_feed_for_resolution(&env, feed_pair);
        Self::require_fresh_price_feed(&env, timestamp, expires_at)?;

        let outcome =
            Self::price_resolution_outcome(price, target_price, comparison_op, tolerance_bps)?;
        let (pool_key, pool) = Self::load_resolvable_price_pool(&env, pool_id)?;
        Self::validate_price_resolution_outcome(&env, pool_id, outcome, pool.options_count)?;
        Self::persist_price_resolution(&env, &pool_key, pool_id, pool, outcome);

        Ok(())
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
        Self::require_not_paused(&env)?;
        oracle.require_auth();

        Self::require_oracle_role_for_resolution(&env, &oracle, pool_id)?;

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

        // Validate: outcome cannot be the sentinel value
        if outcome == UNRESOLVED_OUTCOME {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidOutcome);
        }

        // --- Multi-oracle Voting Logic ---

        let vote_key = DataKey::ResVote(pool_id, oracle.clone());
        if env.storage().temporary().has(&vote_key) {
            return Err(PredifiError::OracleAlreadyVoted);
        }

        // Record the oracle's vote in temporary storage
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

        // Detect conflicts: if there are ANY votes for a different outcome
        if new_total_votes > new_outcome_votes {
            // A conflict exists. Find at least one other voted outcome for the event.
            for i in 0..pool.options_count {
                if i == outcome {
                    continue;
                }
                let other_key = DataKey::ResVoteCt(pool_id, i);
                if env.storage().temporary().has(&other_key) {
                    ResolutionConflictEvent {
                        pool_id,
                        oracle: oracle.clone(),
                        outcome,
                        existing_outcome: i,
                    }
                    .publish(&env);
                    return Err(PredifiError::ResolutionConflict);
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

        // Emit vote-cast event for oracle votes as well
        ResolutionVoteCastEvent {
            pool_id,
            voter: oracle.clone(),
            outcome,
            vote_count: new_outcome_votes,
            required_resolutions: pool.required_resolutions,
        }
        .publish(&env);

        // Check if the required threshold has been met
        if new_outcome_votes >= pool.required_resolutions {
            pool.state = MarketState::Resolved;
            pool.outcome = outcome;
            pool.fee_bps = Self::calculate_dynamic_fee(&env, &pool);
            pool.resolution_timestamp = Some(env.ledger().timestamp()); // Record resolution time

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
