#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Symbol};

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
pub enum DataKey {
    Pool(u64),
    Prediction(Address, u64), // User, PoolId
    PoolIdCounter,
    HasClaimed(Address, u64), // User, PoolId
    OutcomeStake(u64, u32),   // PoolId, Outcome -> Total stake for this outcome
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
    pub fn init(env: Env) {
        if !env.storage().instance().has(&DataKey::PoolIdCounter) {
            env.storage().instance().set(&DataKey::PoolIdCounter, &0u64);
        }
    }

    pub fn create_pool(env: Env, end_time: u64, token: Address) -> u64 {
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
        };
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);
        env.storage()
            .instance()
            .set(&DataKey::PoolIdCounter, &(pool_id + 1));
        pool_id
    }

    pub fn resolve_pool(env: Env, pool_id: u64, outcome: u32) {
        let mut pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .unwrap();
        pool.resolved = true;
        pool.outcome = outcome;
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);
    }

    pub fn place_prediction(env: Env, user: Address, pool_id: u64, amount: i128, outcome: u32) {
        user.require_auth();
        let mut pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .unwrap();

        // Transfer tokens to contract
        // Transfer tokens to contract
        let token_client = token::Client::new(&env, &pool.token);
        let contract_addr = env.current_contract_address();
        token_client.transfer(&user, &contract_addr, &amount);

        // Record prediction
        let prediction = Prediction { amount, outcome };
        env.storage()
            .instance()
            .set(&DataKey::Prediction(user.clone(), pool_id), &prediction);

        // Update total pool stake
        pool.total_stake += amount;
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);

        // Update stake specific to this outcome
        let outcome_key = DataKey::OutcomeStake(pool_id, outcome);
        let current_outcome_stake: i128 = env.storage().instance().get(&outcome_key).unwrap_or(0);
        env.storage()
            .instance()
            .set(&outcome_key, &(current_outcome_stake + amount));
    }

    #[allow(deprecated)]
    pub fn claim_winnings(env: Env, user: Address, pool_id: u64) -> i128 {
        user.require_auth();

        // 1. Validate pool exists and is resolved
        let pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .expect("Pool not found");
        if !pool.resolved {
            panic!("Pool not resolved");
        }

        // 2. Prevent double claiming
        if env
            .storage()
            .instance()
            .has(&DataKey::HasClaimed(user.clone(), pool_id))
        {
            panic!("Already claimed");
        }

        // 3. Get user prediction
        let prediction: Prediction = env
            .storage()
            .instance()
            .get(&DataKey::Prediction(user.clone(), pool_id))
            .expect("No prediction found");

        // 4. Check if user won
        if prediction.outcome != pool.outcome {
            // User lost. No winnings.
            // We could revert or return 0. Returning 0 is safer but revert is clearer for "claim" action.
            // Let's return 0 to denote "nothing to claim" without erroring if they just call it?
            // But usually "claim" implies entitlement. Let's return 0 for now but mark as claimed to prevent re-entrancy attacks or spam?
            // Actually if they lost, they have nothing to claim.
            return 0;
        }

        // 5. Calculate winnings
        // Share = (User Stake / Total Winning Stake) * Total Pool Stake
        // Share = (User Stake / Total Winning Stake) * Total Pool Stake
        let outcome_key = DataKey::OutcomeStake(pool_id, pool.outcome);
        let winning_outcome_stake: i128 = env
            .storage()
            .instance()
            .get(&outcome_key)
            .unwrap_or(0);

        if winning_outcome_stake == 0 {
            // Should not happen if prediction.outcome == pool.outcome and user has stake
            panic!("Critical error: winning stake is 0");
        }

        let winnings = (prediction.amount * pool.total_stake) / winning_outcome_stake;

        // 6. Transfer winnings
        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&env.current_contract_address(), &user, &winnings);

        // 7. Update claim status
        env.storage()
            .instance()
            .set(&DataKey::HasClaimed(user.clone(), pool_id), &true);

        // 8. Emit event
        env.events()
            .publish((Symbol::new(&env, "claim"), user, pool_id), winnings);

        winnings
    }
}

mod test;
