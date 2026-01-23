#![cfg(test)]

use super::*;
use soroban_sdk::{vec, Env, String};

#[test]
fn test() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let words = client.hello(&String::from_str(&env, "Dev"));
    assert_eq!(
        words,
        vec![
            &env,
            String::from_str(&env, "Hello"),
            String::from_str(&env, "Dev"),
        ]
    );
}

#[test]
fn test_get_pool() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let pool_id = 1u64;
    let name = String::from_str(&env, "ETH/USDC");
    let token_a = String::from_str(&env, "ETH");
    let token_b = String::from_str(&env, "USDC");
    let fee_rate = 30u32;
    let end_time = 100u64;

    client.create_pool(&pool_id, &name, &token_a, &token_b, &fee_rate, &end_time);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.pool_id, pool_id);
    assert_eq!(pool.name, name);
    assert_eq!(pool.token_a, token_a);
    assert_eq!(pool.token_b, token_b);
    assert_eq!(pool.fee_rate, fee_rate);
    assert_eq!(pool.is_active, true);
    assert_eq!(pool.total_liquidity, 0);
    assert_eq!(pool.status, PoolStatus::Active);
    assert_eq!(pool.end_time, end_time);
}

#[test]
#[should_panic(expected = "PoolNotFound")]
fn test_get_pool_not_found() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let _ = client.try_get_pool(&999u64).unwrap();
}
