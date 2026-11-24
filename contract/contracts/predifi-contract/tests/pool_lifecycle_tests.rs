//! Pool lifecycle integration tests for `predifi-contract`.
//!
//! Exercises the full `create_pool -> close_staking -> mark_pool_ready ->
//! resolve_pool` flow through the real [`PredifiContractClient`] entry points
//! and asserts the resulting `MarketState` transitions plus the typed errors
//! returned on invalid ordering.

#![cfg(test)]

mod common;

use common::{event_count, setup, test_env, ROLE_OPERATOR};
use predifi_contract::{MarketState, PoolConfig, PredifiError, UNRESOLVED_OUTCOME};
use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env, String};

/// A minimal valid pool configuration with exactly two outcomes.
fn config(env: &Env, required_resolutions: u32) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "Lifecycle pool"),
        metadata_url: String::from_str(env, ""),
        min_stake: 1,
        max_stake: 0,
        min_total_stake: 1,
        max_total_stake: 0,
        initial_liquidity: 0,
        required_resolutions,
        private: false,
        whitelist_key: None,
        outcome_descriptions: vec![
            env,
            String::from_str(env, "Yes"),
            String::from_str(env, "No"),
        ],
    }
}

fn create_pool(env: &Env, h: &common::Harness<'_>, end_time: u64) -> u64 {
    h.client.create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config(env, 1),
    )
}

#[test]
fn create_pool_initial_state_is_active() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    let pool = h.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Active);
    assert_eq!(pool.outcome, UNRESOLVED_OUTCOME);
    assert_eq!(pool.options_count, 2);
    assert_eq!(pool.total_stake, 0);
}

#[test]
fn close_staking_before_end_time_is_rejected() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    let result = h.client.try_close_staking(&pool_id);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::StakingStillOpen);
}

#[test]
fn close_staking_after_end_time_succeeds_and_emits_event() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);
    h.client.close_staking(&pool_id);

    assert_eq!(event_count(&env, "staking_closed"), 1);
}

#[test]
fn close_staking_is_idempotent() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);

    // First call closes staking and emits the event once.
    h.client.close_staking(&pool_id);
    assert_eq!(event_count(&env, "staking_closed"), 1);

    // The second call is idempotent: it succeeds but emits no further event.
    h.client.close_staking(&pool_id);
    assert_eq!(event_count(&env, "staking_closed"), 0);
}

#[test]
fn mark_pool_ready_requires_staking_closed() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);
    let result = h.client.try_mark_pool_ready(&pool_id);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::StakingStillOpen);
}

#[test]
fn mark_pool_ready_after_delay_emits_event() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);
    h.client.close_staking(&pool_id);
    h.client.mark_pool_ready(&pool_id);

    assert_eq!(event_count(&env, "pool_ready"), 1);
}

#[test]
fn resolve_pool_requires_operator_role() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);
    let stranger = Address::generate(&env);

    let result = h.client.try_resolve_pool(&stranger, &pool_id, &1u32);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::Unauthorized);
}

#[test]
fn resolve_pool_out_of_range_outcome_is_rejected() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);
    let result = h.client.try_resolve_pool(&h.operator, &pool_id, &2u32);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::InvalidOutcome);
}

#[test]
fn resolve_pool_transitions_to_resolved() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);
    h.client.close_staking(&pool_id);
    h.client.mark_pool_ready(&pool_id);
    h.client.resolve_pool(&h.operator, &pool_id, &1u32);

    // Events reflect the most recent invocation (`resolve_pool`), so they must
    // be inspected before any further client call.
    assert_eq!(event_count(&env, "pool_resolved"), 1);

    let pool = h.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 1);
}

#[test]
fn resolving_an_already_resolved_pool_is_rejected() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);
    h.client.resolve_pool(&h.operator, &pool_id, &0u32);

    let result = h.client.try_resolve_pool(&h.operator, &pool_id, &1u32);
    assert_eq!(
        result.unwrap_err().unwrap(),
        PredifiError::InvalidPoolState
    );
}

#[test]
fn duplicate_vote_is_rejected_when_quorum_not_yet_reached() {
    let env = test_env();
    let h = setup(&env);

    // Grant a second operator so `required_resolutions = 2` is permitted.
    let second_operator = Address::generate(&env);
    h.ac.grant_role(&second_operator, &ROLE_OPERATOR);

    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = h.client.create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config(&env, 2),
    );

    h.set_time(end_time + 1);
    // First vote is recorded but quorum (2) is not yet reached.
    h.client.resolve_pool(&h.operator, &pool_id, &1u32);

    let pool = h.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Active);

    // The same operator may not vote twice.
    let result = h.client.try_resolve_pool(&h.operator, &pool_id, &1u32);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::OracleAlreadyVoted);
}

#[test]
fn full_lifecycle_sequence() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    assert_eq!(h.client.get_pool(&pool_id).state, MarketState::Active);

    h.set_time(end_time + 1);
    h.client.close_staking(&pool_id);
    h.client.mark_pool_ready(&pool_id);
    h.client.resolve_pool(&h.operator, &pool_id, &0u32);

    let pool = h.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 0);
}
