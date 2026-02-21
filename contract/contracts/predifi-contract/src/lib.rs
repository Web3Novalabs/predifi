#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env,
    IntoVal, String, Symbol, Vec,
};

const DAY_IN_LEDGERS: u32 = 17280;
const BUMP_THRESHOLD: u32 = 14 * DAY_IN_LEDGERS;
const BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PredifiError {
    Unauthorized = 10,
    PoolNotResolved = 22,
    AlreadyClaimed = 60,
    PoolCanceled = 70,
}

#[contracttype]
#[derive(Clone)]
pub struct Pool {
    pub end_time: u64,
    pub resolved: bool,
    pub canceled: bool,
    pub outcome: u32,
    pub token: Address,
    pub total_stake: i128,
    /// A short human-readable description of the event being predicted.
    pub description: String,
    /// A URL (e.g. IPFS CIDv1) pointing to extended metadata for this pool.
    pub metadata_url: String,
}

#[contracttype]
#[derive(Clone)]
pub struct Config {
    pub fee_bps: u32,
    pub treasury: Address,
    pub access_control: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct UserPredictionDetail {
    pub pool_id: u64,
    pub amount: i128,
    pub user_outcome: u32,
    pub pool_end_time: u64,
    pub pool_resolved: bool,
    pub pool_outcome: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Pool(u64),
    Prediction(Address, u64),
    PoolIdCounter,
    HasClaimed(Address, u64),
    OutcomeStake(u64, u32),
    UserPredictionCount(Address),
    UserPredictionIndex(Address, u32),
    Config,
    Paused,
}

#[contracttype]
#[derive(Clone)]
pub struct Prediction {
    pub amount: i128,
    pub outcome: u32,
}

// ── Events ───────────────────────────────────────────────────────────────────

#[contractevent(topics = ["init"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitEvent {
    pub access_control: Address,
    pub treasury: Address,
    pub fee_bps: u32,
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

#[contractevent(topics = ["treasury_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryUpdateEvent {
    pub admin: Address,
    pub treasury: Address,
}

#[contractevent(topics = ["pool_created"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolCreatedEvent {
    pub pool_id: u64,
    pub end_time: u64,
    pub token: Address,
    /// Metadata URL included so off-chain indexers can immediately fetch context.
    pub metadata_url: String,
}

#[contractevent(topics = ["pool_resolved"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolResolvedEvent {
    pub pool_id: u64,
    pub operator: Address,
    pub outcome: u32,
}

#[contractevent(topics = ["pool_canceled"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolCanceledEvent {
    pub pool_id: u64,
    pub caller: Address,
    pub reason: String,
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

// ─────────────────────────────────────────────────────────────────────────────

#[contract]
pub struct PredifiContract;

#[contractimpl]
impl PredifiContract {
    // ── Private helpers ───────────────────────────────────────────────────────

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

    fn has_role(env: &Env, contract: &Address, user: &Address, role: u32) -> bool {
        env.invoke_contract(
            contract,
            &Symbol::new(env, "has_role"),
            soroban_sdk::vec![env, user.into_val(env), role.into_val(env)],
        )
    }

    fn require_role(env: &Env, user: &Address, role: u32) -> Result<(), PredifiError> {
        let config = Self::get_config(env);
        if !Self::has_role(env, &config.access_control, user, role) {
            return Err(PredifiError::Unauthorized);
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

    // ── Public interface ──────────────────────────────────────────────────────

    /// Initialize the contract. Idempotent — safe to call multiple times.
    pub fn init(env: Env, access_control: Address, treasury: Address, fee_bps: u32) {
        if !env.storage().instance().has(&DataKey::Config) {
            let config = Config {
                fee_bps,
                treasury: treasury.clone(),
                access_control: access_control.clone(),
            };
            env.storage().instance().set(&DataKey::Config, &config);
            env.storage().instance().set(&DataKey::PoolIdCounter, &0u64);
            Self::extend_instance(&env);

            InitEvent {
                access_control,
                treasury,
                fee_bps,
            }
            .publish(&env);
        }
    }

    /// Pause the contract. Only callable by Admin (role 0).
    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        Self::require_role(&env, &admin, 0)
            .unwrap_or_else(|_| panic!("Unauthorized: missing required role"));
        env.storage().instance().set(&DataKey::Paused, &true);
        Self::extend_instance(&env);

        PauseEvent { admin }.publish(&env);
    }

    /// Unpause the contract. Only callable by Admin (role 0).
    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        Self::require_role(&env, &admin, 0)
            .unwrap_or_else(|_| panic!("Unauthorized: missing required role"));
        env.storage().instance().set(&DataKey::Paused, &false);
        Self::extend_instance(&env);

        UnpauseEvent { admin }.publish(&env);
    }

    /// Set fee in basis points. Caller must have Admin role (0).
    pub fn set_fee_bps(env: Env, admin: Address, fee_bps: u32) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_role(&env, &admin, 0)?;
        assert!(fee_bps <= 10_000, "fee_bps exceeds 10000");
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
        Self::require_role(&env, &admin, 0)?;
        let mut config = Self::get_config(&env);
        config.treasury = treasury.clone();
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        TreasuryUpdateEvent { admin, treasury }.publish(&env);
        Ok(())
    }

    /// Create a new prediction pool. Returns the new pool ID.
    ///
    /// # Arguments
    /// * `end_time`     - Unix timestamp after which no more predictions are accepted.
    /// * `token`        - The Stellar token contract address used for staking.
    /// * `description`  - Short human-readable description of the event (max 256 bytes).
    /// * `metadata_url` - URL pointing to extended metadata, e.g. an IPFS link (max 512 bytes).
    pub fn create_pool(
        env: Env,
        end_time: u64,
        token: Address,
        description: String,
        metadata_url: String,
    ) -> u64 {
        Self::require_not_paused(&env);
        assert!(
            end_time > env.ledger().timestamp(),
            "end_time must be in the future"
        );
        assert!(description.len() <= 256, "description exceeds 256 bytes");
        assert!(metadata_url.len() <= 512, "metadata_url exceeds 512 bytes");

        let pool_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PoolIdCounter)
            .unwrap_or(0);
        Self::extend_instance(&env);

        let pool = Pool {
            end_time,
            resolved: false,
            canceled: false,
            outcome: 0,
            token: token.clone(),
            total_stake: 0,
            description,
            metadata_url: metadata_url.clone(),
        };

        let pool_key = DataKey::Pool(pool_id);
        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        env.storage()
            .instance()
            .set(&DataKey::PoolIdCounter, &(pool_id + 1));
        Self::extend_instance(&env);

        PoolCreatedEvent {
            pool_id,
            end_time,
            token,
            metadata_url,
        }
        .publish(&env);

        pool_id
    }

    /// Resolve a pool with a winning outcome. Caller must have Operator role (1).
    /// Cannot resolve a canceled pool.
    pub fn resolve_pool(
        env: Env,
        operator: Address,
        pool_id: u64,
        outcome: u32,
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

        assert!(!pool.resolved, "Pool already resolved");
        assert!(!pool.canceled, "Cannot resolve a canceled pool");

        pool.resolved = true;
        pool.outcome = outcome;

        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        PoolResolvedEvent {
            pool_id,
            operator,
            outcome,
        }
        .publish(&env);
        Ok(())
    }

    /// Cancel a pool, freezing all betting and enabling refund process.
    /// Only callable by Admin (role 0) - can cancel any pool for any reason.
    ///
    /// # Arguments
    /// * `caller`  - The address requesting the cancellation (must be admin).
    /// * `pool_id` - The ID of the pool to cancel.
    /// * `reason`  - A short description of why the pool is being canceled.
    ///
    /// # Errors
    /// - `Unauthorized` if caller is not admin.
    /// - `PoolNotResolved` error (code 22) is returned if trying to cancel an already resolved pool.
    pub fn cancel_pool(
        env: Env,
        caller: Address,
        pool_id: u64,
        reason: String,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        caller.require_auth();

        // Check authorization: caller must be admin (role 0)
        Self::require_role(&env, &caller, 0)?;

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        // Ensure resolved pools cannot be canceled
        if pool.resolved {
            return Err(PredifiError::PoolNotResolved);
        }

        // Prevent double cancellation
        assert!(!pool.canceled, "Pool already canceled");

        // Mark pool as canceled
        pool.canceled = true;
        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        PoolCanceledEvent {
            pool_id,
            caller,
            reason,
        }
        .publish(&env);

        Ok(())
    }

    /// Place a prediction on a pool. Cannot predict on canceled pools.
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn place_prediction(env: Env, user: Address, pool_id: u64, amount: i128, outcome: u32) {
        Self::require_not_paused(&env);
        user.require_auth();
        assert!(amount > 0, "amount must be positive");

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        assert!(!pool.resolved, "Pool already resolved");
        assert!(!pool.canceled, "Cannot place prediction on canceled pool");
        assert!(env.ledger().timestamp() < pool.end_time, "Pool has ended");

        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let pred_key = DataKey::Prediction(user.clone(), pool_id);
        env.storage()
            .persistent()
            .set(&pred_key, &Prediction { amount, outcome });
        Self::extend_persistent(&env, &pred_key);

        pool.total_stake = pool.total_stake.checked_add(amount).expect("overflow");
        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        let outcome_key = DataKey::OutcomeStake(pool_id, outcome);
        let current_stake: i128 = env.storage().persistent().get(&outcome_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&outcome_key, &(current_stake + amount));
        Self::extend_persistent(&env, &outcome_key);

        let count_key = DataKey::UserPredictionCount(user.clone());
        let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);

        let index_key = DataKey::UserPredictionIndex(user.clone(), count);
        env.storage().persistent().set(&index_key, &pool_id);
        Self::extend_persistent(&env, &index_key);

        env.storage().persistent().set(&count_key, &(count + 1));
        Self::extend_persistent(&env, &count_key);

        PredictionPlacedEvent {
            pool_id,
            user,
            amount,
            outcome,
        }
        .publish(&env);
    }

    /// Claim winnings from a resolved pool. Returns the amount paid out (0 for losers).
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn claim_winnings(env: Env, user: Address, pool_id: u64) -> Result<i128, PredifiError> {
        Self::require_not_paused(&env);
        user.require_auth();

        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        if !pool.resolved {
            return Err(PredifiError::PoolNotResolved);
        }

        let claimed_key = DataKey::HasClaimed(user.clone(), pool_id);
        if env.storage().persistent().has(&claimed_key) {
            return Err(PredifiError::AlreadyClaimed);
        }

        // Mark as claimed immediately to prevent re-entrancy
        env.storage().persistent().set(&claimed_key, &true);
        Self::extend_persistent(&env, &claimed_key);

        let pred_key = DataKey::Prediction(user.clone(), pool_id);
        let prediction: Option<Prediction> = env.storage().persistent().get(&pred_key);

        if env.storage().persistent().has(&pred_key) {
            Self::extend_persistent(&env, &pred_key);
        }

        let prediction = match prediction {
            Some(p) => p,
            None => return Ok(0),
        };

        if prediction.outcome != pool.outcome {
            return Ok(0);
        }

        let outcome_key = DataKey::OutcomeStake(pool_id, pool.outcome);
        let winning_stake: i128 = env.storage().persistent().get(&outcome_key).unwrap_or(0);
        if env.storage().persistent().has(&outcome_key) {
            Self::extend_persistent(&env, &outcome_key);
        }

        if winning_stake == 0 {
            return Ok(0);
        }

        let winnings = prediction
            .amount
            .checked_mul(pool.total_stake)
            .expect("overflow")
            .checked_div(winning_stake)
            .expect("division by zero");

        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&env.current_contract_address(), &user, &winnings);

        WinningsClaimedEvent {
            pool_id,
            user,
            amount: winnings,
        }
        .publish(&env);

        Ok(winnings)
    }

    /// Get a paginated list of a user's predictions.
    pub fn get_user_predictions(
        env: Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<UserPredictionDetail> {
        let count_key = DataKey::UserPredictionCount(user.clone());
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
            let index_key = DataKey::UserPredictionIndex(user.clone(), i);
            let pool_id: u64 = env
                .storage()
                .persistent()
                .get(&index_key)
                .expect("index not found");
            Self::extend_persistent(&env, &index_key);

            let pred_key = DataKey::Prediction(user.clone(), pool_id);
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
                pool_resolved: pool.resolved,
                pool_outcome: pool.outcome,
            });
        }

        results
    }
}

mod test;
