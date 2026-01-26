#![no_std]
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, token, Address, Env};
// Event structs for contract events
#[contractevent]
pub struct SetFeeBpsEvent {
    pub new_fee_bps: u32,
}

#[contractevent]
pub struct SetTreasuryEvent {
    pub new_treasury: Address,
}

#[contractevent]
pub struct FeeCollectedEvent {
    pub pool_id: u64,
    pub fee: i128,
}

#[contractevent]
pub struct FeeDistributedEvent {
    pub pool_id: u64,
    pub fee: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct Pool {
    pub end_time: u64,
    pub resolved: bool,
    pub outcome: u32,
    pub token: Address,
    pub total_stake: i128,
    pub cancelled: bool,
    pub admin: Address,
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
    Prediction(Address, u64), // User, PoolId
    PoolIdCounter,
    HasClaimed(Address, u64), // User, PoolId
    OutcomeStake(u64, u32),   // PoolId, Outcome -> Total stake for this outcome
    UserPredictionCount(Address),
    UserPredictionIndex(Address, u32), // User, Index -> PoolId
    PoolUserCount(u64),                // PoolId -> number of unique users
    PoolUserIndex(u64, u32),           // PoolId, index -> Address
    Admin,
    FeeBps,                            // Fee in basis points (1/100 of a percent)
    Treasury,                          // Protocol treasury address
    CollectedFees(u64),                // PoolId -> Collected fee amount
}

#[contracttype]
#[derive(Clone)]
pub struct Prediction {
    pub amount: i128,
    pub outcome: u32,
}

#[contract]
pub struct PredifiContract;

impl PredifiContract {
    fn get_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("admin not set")
    }

    fn require_admin(env: &Env) -> Address {
        let admin = Self::get_admin(env);
        admin.require_auth();
        admin
    }

    fn guard_pool_not_final(pool: &Pool) {
        if pool.cancelled || pool.resolved {
            panic!("Pool already finalized");
        }
    }
}

#[contractimpl]
impl PredifiContract {

    pub fn init(env: Env, treasury: Address, fee_bps: u32) {
        // Only set if not already initialized
        if !env.storage().instance().has(&DataKey::PoolIdCounter) {
            env.storage().instance().set(&DataKey::PoolIdCounter, &0u64);
        }
        if !env.storage().instance().has(&DataKey::FeeBps) {
            env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
        }
        if !env.storage().instance().has(&DataKey::Treasury) {
            env.storage().instance().set(&DataKey::Treasury, &treasury);
        }
    }

    // Set fee (basis points, e.g. 100 = 1%)
    pub fn set_fee_bps(env: Env, fee_bps: u32) {
        // Add access control as needed
        env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
        SetFeeBpsEvent {
            new_fee_bps: fee_bps,
        }
        .publish(&env);
    }

    pub fn get_fee_bps(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0)
    }

    pub fn set_treasury(env: Env, treasury: Address) {
        // Add access control as needed
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        SetTreasuryEvent {
            new_treasury: treasury.clone(),
        }
        .publish(&env);
    }

    pub fn get_treasury(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Treasury)
            .expect("Treasury not set")
    }

    pub fn create_pool(env: Env, end_time: u64, token: Address) -> u64 {
        let admin = Self::require_admin(&env);

        let pool_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PoolIdCounter)
            .unwrap_or(0);
        let pool = Pool {
            end_time,
            resolved: false,
            outcome: 0,
            total_stake: 0,
            token,
            cancelled: false,
            admin,
        };
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);
        env.storage()
            .instance()
            .set(&DataKey::PoolIdCounter, &(pool_id + 1));
        // No fee collected at creation, but could emit event if needed
        pool_id
    }

    pub fn resolve_pool(env: Env, pool_id: u64, outcome: u32) {
        let _admin = Self::require_admin(&env);

        let mut pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .unwrap();

        Self::guard_pool_not_final(&pool);

        pool.resolved = true;
        pool.outcome = outcome;
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);

        // Calculate and store fee for this pool
        let fee_bps: u32 = env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0);
        if fee_bps > 0 && pool.total_stake > 0 {
            let fee = (pool.total_stake * (fee_bps as i128)) / 10_000;
            env.storage()
                .instance()
                .set(&DataKey::CollectedFees(pool_id), &fee);
            FeeCollectedEvent { pool_id, fee }.publish(&env);
        }
    }

    pub fn place_prediction(env: Env, user: Address, pool_id: u64, amount: i128, outcome: u32) {
        user.require_auth();
        let mut pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .unwrap();

        Self::guard_pool_not_final(&pool);

        let token_client = token::Client::new(&env, &pool.token);
        let contract_addr = env.current_contract_address();
        token_client.transfer(&user, &contract_addr, &amount);

        let mut prediction: Prediction = env
            .storage()
            .instance()
            .get(&DataKey::Prediction(user.clone(), pool_id))
            .unwrap_or(Prediction { amount: 0, outcome });

        if prediction.amount == 0 {
            let pool_user_count: u32 = env
                .storage()
                .instance()
                .get(&DataKey::PoolUserCount(pool_id))
                .unwrap_or(0);
            env.storage()
                .instance()
                .set(&DataKey::PoolUserIndex(pool_id, pool_user_count), &user);
            env.storage()
                .instance()
                .set(&DataKey::PoolUserCount(pool_id), &(pool_user_count + 1));
        } else if prediction.outcome != outcome {
            panic!("Cannot change prediction outcome");
        }

        prediction.amount += amount;
        env.storage()
            .instance()
            .set(&DataKey::Prediction(user.clone(), pool_id), &prediction);

        pool.total_stake += amount;
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);

        let outcome_key = DataKey::OutcomeStake(pool_id, outcome);
        let current_outcome_stake: i128 = env.storage().instance().get(&outcome_key).unwrap_or(0);
        env.storage()
            .instance()
            .set(&outcome_key, &(current_outcome_stake + amount));

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

    #[allow(deprecated)]
    pub fn claim_winnings(env: Env, user: Address, pool_id: u64) -> i128 {
        user.require_auth();

        let pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .expect("Pool not found");

        if pool.cancelled {
            panic!("Pool cancelled");
        }
        if !pool.resolved {
            panic!("Pool not resolved");
        }

        if env
            .storage()
            .instance()
            .has(&DataKey::HasClaimed(user.clone(), pool_id))
        {
            panic!("Already claimed");
        }

        let prediction: Prediction = env
            .storage()
            .instance()
            .get(&DataKey::Prediction(user.clone(), pool_id))
            .expect("No prediction found");

        if prediction.outcome != pool.outcome {
            return 0;
        }

        let outcome_key = DataKey::OutcomeStake(pool_id, pool.outcome);
        let winning_outcome_stake: i128 = env.storage().instance().get(&outcome_key).unwrap_or(0);

        if winning_outcome_stake == 0 {
            panic!("Critical error: winning stake is 0");
        }

        let gross_winnings = (prediction.amount * pool.total_stake) / winning_outcome_stake;
        // Deduct fee proportionally from winnings
        let fee_bps: u32 = env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0);
        let mut fee_share = 0i128;
        if fee_bps > 0 && pool.total_stake > 0 {
            let total_fee: i128 = env
                .storage()
                .instance()
                .get(&DataKey::CollectedFees(pool_id))
                .unwrap_or(0);
            // User's share of fee = (user's gross winnings / total pool stake) * total_fee
            fee_share = (gross_winnings * total_fee) / pool.total_stake;
        }
        let net_winnings = gross_winnings - fee_share;


        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&env.current_contract_address(), &user, &net_winnings);

        env.storage()
            .instance()
            .set(&DataKey::HasClaimed(user.clone(), pool_id), &true);

        env.events().publish(
            (Symbol::new(&env, "claim"), user.clone(), pool_id),
            winnings,
        );
        // 8. Emit event (still using legacy for claim, or add #[contractevent] if needed)
        // env.events().publish((Symbol::new(&env, "claim"), user.clone(), pool_id), net_winnings);

        // 9. On first claim, transfer fee to treasury
        if fee_bps > 0 && pool.total_stake > 0 {
            let total_fee: i128 = env
                .storage()
                .instance()
                .get(&DataKey::CollectedFees(pool_id))
                .unwrap_or(0);
            if total_fee > 0 {
                // Use HasClaimed for a special address to mark fee paid
                let marker_addr = env.current_contract_address();
                let fee_paid_key = DataKey::HasClaimed(marker_addr, pool_id);
                if !env.storage().instance().has(&fee_paid_key) {
                    let treasury: Address = env
                        .storage()
                        .instance()
                        .get(&DataKey::Treasury)
                        .expect("Treasury not set");
                    token_client.transfer(&env.current_contract_address(), &treasury, &total_fee);
                    env.storage().instance().set(&fee_paid_key, &true);
                    FeeDistributedEvent {
                        pool_id,
                        fee: total_fee,
                    }
                    .publish(&env);
                }
            }
        }

        net_winnings
    }

    #[allow(deprecated)]
    pub fn cancel_pool(env: Env, pool_id: u64) {
        let admin = Self::require_admin(&env);

        let mut pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .expect("Pool not found");

        Self::guard_pool_not_final(&pool);

        pool.cancelled = true;
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);

        let pool_user_count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PoolUserCount(pool_id))
            .unwrap_or(0);

        let token_client = token::Client::new(&env, &pool.token);
        let contract_addr = env.current_contract_address();

        for i in 0..pool_user_count {
            let user: Address = env
                .storage()
                .instance()
                .get(&DataKey::PoolUserIndex(pool_id, i))
                .expect("User index missing");

            let prediction: Option<Prediction> = env
                .storage()
                .instance()
                .get(&DataKey::Prediction(user.clone(), pool_id));

            if let Some(pred) = prediction {
                if pred.amount > 0 {
                    token_client.transfer(&contract_addr, &user, &pred.amount);

                    env.storage().instance().set(
                        &DataKey::Prediction(user.clone(), pool_id),
                        &Prediction {
                            amount: 0,
                            outcome: pred.outcome,
                        },
                    );
                }
            }
        }

        pool.total_stake = 0;
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);

        let topic = (Symbol::new(&env, "pool_cancel"), pool_id);
        let data = (admin, env.ledger().timestamp());
        env.events().publish(topic, data);
    }

    pub fn get_user_predictions(
        env: Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<UserPredictionDetail> {
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::UserPredictionCount(user.clone()))
            .unwrap_or(0);

        let mut results = soroban_sdk::Vec::new(&env);
        if offset >= count {
            return results;
        }

        let end = core::cmp::min(offset + limit, count);

        for i in offset..end {
            let pool_id: u64 = env
                .storage()
                .instance()
                .get(&DataKey::UserPredictionIndex(user.clone(), i))
                .unwrap();

            let prediction: Prediction = env
                .storage()
                .instance()
                .get(&DataKey::Prediction(user.clone(), pool_id))
                .unwrap();

            let pool: Pool = env
                .storage()
                .instance()
                .get(&DataKey::Pool(pool_id))
                .unwrap();

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

// stellar contract build
