//! High-volume prediction stress tests for `predifi-contract`.
//!
//! These tests drive the real [`PredifiContractClient`] with many participants
//! and predictions, then assert on the aggregate pool state (`PoolStats`) to
//! catch regressions in stake accounting, participant counting and per-outcome
//! bookkeeping under load.

#![cfg(test)]

mod common;

use common::{setup, test_env};
use predifi_contract::{MarketState, PoolConfig};
use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env, String};

/// A minimal valid pool configuration with two outcomes.
fn config(env: &Env) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "Stress pool"),
        metadata_url: String::from_str(env, ""),
        min_stake: 1,
        max_stake: 0,
        min_total_stake: 1,
        max_total_stake: 0,
        initial_liquidity: 0,
        required_resolutions: 1,
        private: false,
        whitelist_key: None,
        outcome_descriptions: vec![
            env,
            String::from_str(env, "Up"),
            String::from_str(env, "Down"),
        ],
    }
}

/// Create a fresh two-outcome pool and return its id.
fn create_pool(env: &Env, h: &common::Harness<'_>) -> u64 {
    let end_time = env.ledger().timestamp() + 10_000;
    h.client.create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config(env),
    )
}

#[test]
fn many_distinct_predictions_accumulate_stake_and_participants() {
    let env = test_env();
    let h = setup(&env);
    let pool_id = create_pool(&env, &h);

    let count = 200u32;
    for i in 0..count {
        let user = Address::generate(&env);
        h.mint(&user, 1_000);
        h.client
            .place_prediction(&user, &pool_id, &100, &(i % 2), &None, &None);
    }

    let stats = h.client.get_pool_stats(&pool_id);
    assert_eq!(stats.total_stake, (count as i128) * 100);
    assert_eq!(stats.participants_count, count);
    // 100 users on each outcome, 100 tokens each.
    assert_eq!(stats.stakes_per_outcome.get(0).unwrap(), 100 * 100);
    assert_eq!(stats.stakes_per_outcome.get(1).unwrap(), 100 * 100);
    assert_eq!(h.client.get_pool_participants_count(&pool_id), count);
}

#[test]
fn single_user_can_place_many_predictions() {
    let env = test_env();
    let h = setup(&env);
    let pool_id = create_pool(&env, &h);

    let user = Address::generate(&env);
    h.mint(&user, 100_000);

    let count = 100u32;
    for _ in 0..count {
        h.client
            .place_prediction(&user, &pool_id, &50, &0u32, &None, &None);
    }

    let stats = h.client.get_pool_stats(&pool_id);
    assert_eq!(stats.total_stake, (count as i128) * 50);
    // A single unique address, regardless of prediction count.
    assert_eq!(stats.participants_count, 1);
}

#[test]
fn stake_accounting_is_exact_across_outcomes() {
    let env = test_env();
    let h = setup(&env);
    let pool_id = create_pool(&env, &h);

    let mut expected = [0i128, 0i128];
    for i in 0..50u32 {
        let user = Address::generate(&env);
        let outcome = i % 2;
        let amount = 10 + (i as i128);
        h.mint(&user, amount);
        h.client
            .place_prediction(&user, &pool_id, &amount, &outcome, &None, &None);
        expected[outcome as usize] += amount;
    }

    let stats = h.client.get_pool_stats(&pool_id);
    assert_eq!(stats.total_stake, expected[0] + expected[1]);
    assert_eq!(stats.stakes_per_outcome.get(0).unwrap(), expected[0]);
    assert_eq!(stats.stakes_per_outcome.get(1).unwrap(), expected[1]);
}

#[test]
fn high_volume_pool_resolves_and_leaves_active_index() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 10_000;
    let pool_id = h.client.create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config(&env),
    );

    for i in 0..100u32 {
        let user = Address::generate(&env);
        h.mint(&user, 1_000);
        h.client
            .place_prediction(&user, &pool_id, &100, &(i % 2), &None, &None);
    }

    assert_eq!(h.client.get_active_pools_count(), 1);

    h.set_time(end_time + 1);
    h.client.resolve_pool(&h.operator, &pool_id, &0u32);

    let pool = h.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    // Resolved pools are removed from the active index.
    assert_eq!(h.client.get_active_pools_count(), 0);
}

#[test]
fn predictions_from_many_users_are_independently_tracked() {
    let env = test_env();
    let h = setup(&env);
    let pool_id = create_pool(&env, &h);

    let a = Address::generate(&env);
    let b = Address::generate(&env);
    h.mint(&a, 1_000);
    h.mint(&b, 1_000);

    h.client
        .place_prediction(&a, &pool_id, &100, &0u32, &None, &None);
    h.client
        .place_prediction(&b, &pool_id, &250, &1u32, &None, &None);

    // Second prediction from `a` accumulates on top of the first.
    h.client
        .place_prediction(&a, &pool_id, &150, &0u32, &None, &None);

    let stats = h.client.get_pool_stats(&pool_id);
    assert_eq!(stats.total_stake, 500);
    assert_eq!(stats.stakes_per_outcome.get(0).unwrap(), 250);
    assert_eq!(stats.stakes_per_outcome.get(1).unwrap(), 250);
    assert_eq!(stats.participants_count, 2);
}
