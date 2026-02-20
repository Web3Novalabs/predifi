#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, IntoVal, Symbol, Vec,
};

#[contracttype]
#[derive(Clone)]
pub struct Pool {
    pub end_time: u64,
    pub resolved: bool,
    pub outcome: u32,
    pub token: Address,
    pub total_stake: i128,
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

#[contract]
pub struct PredifiContract;

#[contractimpl]
impl PredifiContract {
    // ── Private helpers ───────────────────────────────────────────────────────

    fn has_role(env: &Env, contract: &Address, user: &Address, role: u32) -> bool {
        env.invoke_contract(
            contract,
            &Symbol::new(env, "has_role"),
            soroban_sdk::vec![env, user.into_val(env), role.into_val(env)],
        )
    }

    fn require_role(env: &Env, user: &Address, role: u32) {
        let config = Self::get_config(env);
        if !Self::has_role(env, &config.access_control, user, role) {
            panic!("Unauthorized: missing required role");
        }
    }

    fn get_config(env: &Env) -> Config {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .expect("Config not set")
    }

    fn is_paused(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
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
                treasury,
                access_control,
            };
            env.storage().instance().set(&DataKey::Config, &config);
            env.storage().instance().set(&DataKey::PoolIdCounter, &0u64);
        }
    }

    /// Pause the contract. Only callable by Admin (role 0).
    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        Self::require_role(&env, &admin, 0);
        env.storage().instance().set(&DataKey::Paused, &true);
    }

    /// Unpause the contract. Only callable by Admin (role 0).
    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        Self::require_role(&env, &admin, 0);
        env.storage().instance().set(&DataKey::Paused, &false);
    }

    /// Set fee in basis points. Caller must have Admin role (0).
    pub fn set_fee_bps(env: Env, admin: Address, fee_bps: u32) {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_role(&env, &admin, 0);
        assert!(fee_bps <= 10_000, "fee_bps exceeds 10000");
        let mut config = Self::get_config(&env);
        config.fee_bps = fee_bps;
        env.storage().instance().set(&DataKey::Config, &config);
    }

    /// Set treasury address. Caller must have Admin role (0).
    pub fn set_treasury(env: Env, admin: Address, treasury: Address) {
        Self::require_not_paused(&env);
        admin.require_auth();
        Self::require_role(&env, &admin, 0);
        let mut config = Self::get_config(&env);
        config.treasury = treasury;
        env.storage().instance().set(&DataKey::Config, &config);
    }

    /// Create a new prediction pool. Returns the new pool ID.
    pub fn create_pool(env: Env, end_time: u64, token: Address) -> u64 {
        Self::require_not_paused(&env);
        assert!(
            end_time > env.ledger().timestamp(),
            "end_time must be in the future"
        );

        let pool_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PoolIdCounter)
            .unwrap_or(0);

        let pool = Pool {
            end_time,
            resolved: false,
            outcome: 0,
            token,
            total_stake: 0,
        };

        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);
        env.storage()
            .instance()
            .set(&DataKey::PoolIdCounter, &(pool_id + 1));

        pool_id
    }

    /// Resolve a pool with a winning outcome. Caller must have Operator role (1).
    pub fn resolve_pool(env: Env, operator: Address, pool_id: u64, outcome: u32) {
        Self::require_not_paused(&env);
        operator.require_auth();
        Self::require_role(&env, &operator, 1);

        let mut pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .expect("Pool not found");

        assert!(!pool.resolved, "Pool already resolved");

        pool.resolved = true;
        pool.outcome = outcome;

        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);
    }

    /// Place a prediction on a pool.
    pub fn place_prediction(env: Env, user: Address, pool_id: u64, amount: i128, outcome: u32) {
        Self::require_not_paused(&env);
        user.require_auth();
        assert!(amount > 0, "amount must be positive");

        let mut pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .expect("Pool not found");

        assert!(!pool.resolved, "Pool already resolved");
        assert!(env.ledger().timestamp() < pool.end_time, "Pool has ended");

        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&user, env.current_contract_address(), &amount);

        env.storage().instance().set(
            &DataKey::Prediction(user.clone(), pool_id),
            &Prediction { amount, outcome },
        );

        pool.total_stake = pool.total_stake.checked_add(amount).expect("overflow");
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);

        let outcome_key = DataKey::OutcomeStake(pool_id, outcome);
        let current_stake: i128 = env.storage().instance().get(&outcome_key).unwrap_or(0);
        env.storage()
            .instance()
            .set(&outcome_key, &(current_stake + amount));

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::UserPredictionCount(user.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::UserPredictionIndex(user.clone(), count), &pool_id);
        env.storage()
            .instance()
            .set(&DataKey::UserPredictionCount(user.clone()), &(count + 1));
    }

    /// Claim winnings from a resolved pool. Returns the amount paid out (0 for losers).
    pub fn claim_winnings(env: Env, user: Address, pool_id: u64) -> i128 {
        Self::require_not_paused(&env);
        user.require_auth();

        let pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .expect("Pool not found");

        assert!(pool.resolved, "Pool not resolved");
        assert!(
            !env.storage()
                .instance()
                .has(&DataKey::HasClaimed(user.clone(), pool_id)),
            "Already claimed"
        );

        // Mark as claimed immediately to prevent re-entrancy
        env.storage()
            .instance()
            .set(&DataKey::HasClaimed(user.clone(), pool_id), &true);

        let prediction: Option<Prediction> = env
            .storage()
            .instance()
            .get(&DataKey::Prediction(user.clone(), pool_id));

        let prediction = match prediction {
            Some(p) => p,
            None => return 0,
        };

        if prediction.outcome != pool.outcome {
            return 0;
        }

        let winning_stake: i128 = env
            .storage()
            .instance()
            .get(&DataKey::OutcomeStake(pool_id, pool.outcome))
            .unwrap_or(0);

        if winning_stake == 0 {
            return 0;
        }

        let winnings = prediction
            .amount
            .checked_mul(pool.total_stake)
            .expect("overflow")
            .checked_div(winning_stake)
            .expect("division by zero");

        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&env.current_contract_address(), &user, &winnings);

        winnings
    }

    /// Get a paginated list of a user's predictions.
    pub fn get_user_predictions(
        env: Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<UserPredictionDetail> {
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::UserPredictionCount(user.clone()))
            .unwrap_or(0);

        let mut results = Vec::new(&env);

        if offset >= count || limit == 0 {
            return results;
        }

        let end = core::cmp::min(offset.saturating_add(limit), count);

        for i in offset..end {
            let pool_id: u64 = env
                .storage()
                .instance()
                .get(&DataKey::UserPredictionIndex(user.clone(), i))
                .expect("index not found");

            let prediction: Prediction = env
                .storage()
                .instance()
                .get(&DataKey::Prediction(user.clone(), pool_id))
                .expect("prediction not found");

            let pool: Pool = env
                .storage()
                .instance()
                .get(&DataKey::Pool(pool_id))
                .expect("pool not found");

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
