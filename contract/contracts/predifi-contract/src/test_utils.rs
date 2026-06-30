use soroban_sdk::{token, Address, Env};

pub struct TokenTestContext {
    pub token_address: Address,
    pub token: token::Client<'static>,
    pub admin: token::StellarAssetClient<'static>,
}

impl TokenTestContext {
    pub fn deploy(env: &Env, admin: &Address) -> Self {
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_address = token_contract.address();
        let token = token::Client::new(env, &token_address);
        let token_admin = token::StellarAssetClient::new(env, &token_address);

        Self {
            token_address,
            token,
            admin: token_admin,
        }
    }

    pub fn mint(&self, to: &Address, amount: i128) {
        self.admin.mint(to, &amount);
    }
}

use crate::{DataKey, MarketState, Pool};

/// Transition an existing pool to Disputed state for testing.
///
/// This helper loads a pool from storage, sets its state to MarketState::Disputed,
/// and saves it back. Useful for testing operations that should be blocked on disputed pools.
///
/// # Arguments
/// * `env` - The test environment
/// * `pool_id` - The ID of the pool to transition
///
/// # Panics
/// Panics if the pool does not exist in storage.
pub fn transition_pool_to_disputed(env: &Env, pool_id: u64) {
    let pool_key = DataKey::Pool(pool_id);
    let mut pool: Pool = env
        .storage()
        .persistent()
        .get(&pool_key)
        .expect("Pool not found");
    
    pool.state = MarketState::Disputed;
    env.storage().persistent().set(&pool_key, &pool);
}

/// Create a new pool and immediately transition it to Disputed state.
///
/// This is a convenience helper for tests that need a disputed pool.
/// It creates a pool using the provided parameters and then transitions it to Disputed.
///
/// # Arguments
/// * `env` - The test environment
/// * `pool_id` - The ID of an existing pool to make disputed
///
/// # Returns
/// The pool_id of the created disputed pool.
pub fn create_disputed_pool(env: &Env, pool_id: u64) -> u64 {
    transition_pool_to_disputed(env, pool_id);
    pool_id
}
