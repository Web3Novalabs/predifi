//! Pause / unpause integration tests for `predifi-contract`.
//!
//! Exercises the contract-level pause flag through the real
//! [`PredifiContractClient`] entry points and asserts that state-changing
//! operations are rejected with the typed `ContractPaused` error while the
//! contract is paused.

#![cfg(test)]

mod common;

use common::{event_count, setup, test_env};
use predifi_contract::{PredifiError, PoolConfig};
use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env, String};

/// A minimal but valid pool configuration accepted by `create_pool`.
fn valid_config(env: &Env) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "Pause test pool"),
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
            String::from_str(env, "Yes"),
            String::from_str(env, "No"),
        ],
    }
}

#[test]
fn pause_sets_flag_and_unpause_clears_it() {
    let env = test_env();
    let h = setup(&env);

    assert!(!h.client.is_contract_paused());

    h.client.pause(&h.admin);
    assert!(h.client.is_contract_paused());

    h.client.unpause(&h.admin);
    assert!(!h.client.is_contract_paused());
}

#[test]
fn pause_emits_alert_and_pause_events() {
    let env = test_env();
    let h = setup(&env);

    h.client.pause(&h.admin);

    assert_eq!(event_count(&env, "contract_paused_alert"), 1);
    assert_eq!(event_count(&env, "pause"), 1);
}

#[test]
fn unpause_emits_unpause_event() {
    let env = test_env();
    let h = setup(&env);

    h.client.pause(&h.admin);
    h.client.unpause(&h.admin);

    assert_eq!(event_count(&env, "unpause"), 1);
}

#[test]
fn pause_blocks_create_pool() {
    let env = test_env();
    let h = setup(&env);

    h.client.pause(&h.admin);

    let end_time = env.ledger().timestamp() + 3_600;
    let result = h.client.try_create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &valid_config(&env),
    );

    assert_eq!(result.unwrap_err().unwrap(), PredifiError::ContractPaused);
}

#[test]
fn unpause_restores_create_pool() {
    let env = test_env();
    let h = setup(&env);

    h.client.pause(&h.admin);
    h.client.unpause(&h.admin);

    let end_time = env.ledger().timestamp() + 3_600;
    let pool_id = h.client.create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &valid_config(&env),
    );

    assert_eq!(pool_id, 0);
}

#[test]
fn double_pause_is_rejected() {
    let env = test_env();
    let h = setup(&env);

    h.client.pause(&h.admin);

    assert!(h.client.try_pause(&h.admin).is_err());
    assert!(h.client.is_contract_paused());
}

#[test]
fn unpause_when_not_paused_is_rejected() {
    let env = test_env();
    let h = setup(&env);

    assert!(h.client.try_unpause(&h.admin).is_err());
    assert!(!h.client.is_contract_paused());
}

#[test]
fn non_admin_cannot_pause() {
    let env = test_env();
    let h = setup(&env);

    let stranger = Address::generate(&env);

    assert!(h.client.try_pause(&stranger).is_err());
    assert!(!h.client.is_contract_paused());
}

#[test]
fn non_admin_cannot_unpause() {
    let env = test_env();
    let h = setup(&env);

    h.client.pause(&h.admin);
    let stranger = Address::generate(&env);

    assert!(h.client.try_unpause(&stranger).is_err());
    assert!(h.client.is_contract_paused());
}
