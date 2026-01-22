#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, vec, Env, String, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Pool(u64),
}

#[derive(Clone)]
#[contracttype]
pub struct Pool {
    pub pool_id: u64,
    pub name: String,
    pub total_liquidity: i128,
    pub token_a: String,
    pub token_b: String,
    pub fee_rate: u32,
    pub is_active: bool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    PoolNotFound = 1,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn hello(env: Env, to: String) -> Vec<String> {
        vec![&env, String::from_str(&env, "Hello"), to]
    }

    pub fn create_pool(
        env: Env,
        pool_id: u64,
        name: String,
        token_a: String,
        token_b: String,
        fee_rate: u32,
    ) {
        let pool = Pool {
            pool_id,
            name,
            total_liquidity: 0,
            token_a,
            token_b,
            fee_rate,
            is_active: true,
        };
        env.storage().persistent().set(&DataKey::Pool(pool_id), &pool);
    }

    pub fn get_pool(env: Env, pool_id: u64) -> Result<Pool, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Pool(pool_id))
            .ok_or(Error::PoolNotFound)
    }
}

mod test;
