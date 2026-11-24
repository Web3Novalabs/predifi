//! Tests for issues #2035 and #2036.
//!
//! * **#2035**: state-changing pool entry points return
//!   `PredifiError::PoolNotFound` (instead of panicking) for unknown pool IDs.
//!   There is one `try_` test per entry point.
//! * **#2036**: non-panicking pool getters (`find_pool`, `pool_exists`, and the
//!   Rust helper `PredifiContract::try_get_pool`) for existing and missing
//!   pool IDs.

#![cfg(test)]

use crate::test::setup;
use crate::{PoolConfig, PredifiContract, PredifiError};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, String};

/// A pool ID that is never created in these tests.
const MISSING_POOL_ID: u64 = 999_999;

const ROLE_MODERATOR: u32 = 2;

fn create_test_pool(
    env: &Env,
    client: &crate::PredifiContractClient<'_>,
    creator: &Address,
    token_address: &Address,
) -> u64 {
    client.create_pool(
        creator,
        &100_000u64,
        token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(env, "Getter test pool"),
            metadata_url: String::from_str(env, "ipfs://getter-test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                env,
                String::from_str(env, "Outcome 0"),
                String::from_str(env, "Outcome 1"),
            ],
        },
    )
}

// ── #2035: PoolNotFound from pool management entry points ──────────────────

#[test]
fn test_increase_max_total_stake_returns_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, creator) = setup(&env);

    let result = client.try_increase_max_total_stake(&creator, &MISSING_POOL_ID, &1_000i128);
    assert_eq!(result, Err(Ok(PredifiError::PoolNotFound)));
}

#[test]
fn test_update_pool_description_returns_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, creator) = setup(&env);

    let result = client.try_update_pool_description(
        &creator,
        &MISSING_POOL_ID,
        &String::from_str(&env, "new description"),
    );
    assert_eq!(result, Err(Ok(PredifiError::PoolNotFound)));
}

#[test]
fn test_resolve_pool_returns_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, operator, _) = setup(&env);

    let result = client.try_resolve_pool(&operator, &MISSING_POOL_ID, &0u32);
    assert_eq!(result, Err(Ok(PredifiError::PoolNotFound)));
}

#[test]
fn test_mark_pool_ready_returns_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, _) = setup(&env);

    let result = client.try_mark_pool_ready(&MISSING_POOL_ID);
    assert_eq!(result, Err(Ok(PredifiError::PoolNotFound)));
}

#[test]
fn test_cancel_pool_returns_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let result = client.try_cancel_pool(
        &operator,
        &MISSING_POOL_ID,
        &String::from_str(&env, "does not exist"),
    );
    assert_eq!(result, Err(Ok(PredifiError::PoolNotFound)));

    // The reentrancy guard must have been released: cancelling a real pool
    // afterwards still succeeds.
    let pool_id = create_test_pool(&env, &client, &creator, &token_address);
    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, "cleanup"));
}

#[test]
fn test_set_stake_limits_returns_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, operator, _) = setup(&env);

    let result = client.try_set_stake_limits(&operator, &MISSING_POOL_ID, &1i128, &100i128);
    assert_eq!(result, Err(Ok(PredifiError::PoolNotFound)));
}

#[test]
fn test_flag_disputed_pool_returns_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _, _, _, _, _, _) = setup(&env);

    let moderator = Address::generate(&env);
    ac_client.grant_role(&moderator, &ROLE_MODERATOR);

    let result = client.try_flag_disputed_pool(
        &moderator,
        &MISSING_POOL_ID,
        &String::from_str(&env, "does not exist"),
    );
    assert_eq!(result, Err(Ok(PredifiError::PoolNotFound)));
}

// ── #2036: non-panicking pool getters ──────────────────────────────────────

#[test]
fn test_find_pool_and_pool_exists_for_existing_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = create_test_pool(&env, &client, &creator, &token_address);

    assert!(client.pool_exists(&pool_id));

    let found = client.find_pool(&pool_id);
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found, client.get_pool(&pool_id));
    assert_eq!(found.creator, creator);
    assert_eq!(found.options_count, 2);
}

#[test]
fn test_find_pool_and_pool_exists_for_missing_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, _) = setup(&env);

    // Neither call panics for an unknown ID.
    assert!(!client.pool_exists(&MISSING_POOL_ID));
    assert!(client.find_pool(&MISSING_POOL_ID).is_none());

    // The existing panicking getter is unchanged.
    assert!(client.try_get_pool(&MISSING_POOL_ID).is_err());
}

#[test]
fn test_try_get_pool_helper_for_existing_and_missing_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = create_test_pool(&env, &client, &creator, &token_address);
    let expected = client.get_pool(&pool_id);

    env.as_contract(&client.address, || {
        let found = PredifiContract::try_get_pool(&env, pool_id);
        assert_eq!(found, Some(expected.clone()));

        let missing = PredifiContract::try_get_pool(&env, MISSING_POOL_ID);
        assert!(missing.is_none());
    });
}
