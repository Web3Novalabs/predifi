//! Boundary & Edge Case Tests for Issue #1893 — `update_pool_description`
//!
//! Coverage:
//! - Empty string is rejected with `InvalidAmount`
//! - The 256-byte maximum length is accepted; 257 bytes is rejected
//! - Special/ASCII characters are handled correctly
//! - Non-creator, non-admin callers are rejected with `Unauthorized`
//! - Updates are locked once a prediction has been placed
//! - Multiple sequential updates by the creator succeed

#![cfg(test)]

extern crate std;

use crate::{test::setup, PredifiContractClient, PredifiError, PoolConfig};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, String};

/// Create an active, single-participant-free pool owned by `creator`.
fn create_active_pool(
    env: &Env,
    client: &PredifiContractClient<'_>,
    token_address: &Address,
    creator: &Address,
) -> u64 {
    client.create_pool(
        creator,
        &100_000u64,
        token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(env, "Original description"),
            metadata_url: String::from_str(env, "ipfs://desc-boundary-test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                env,
                String::from_str(env, "Yes"),
                String::from_str(env, "No"),
            ],
        },
    )
}

#[test]
fn test_update_with_empty_string_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);
    let pool_id = create_active_pool(&env, &client, &token_address, &creator);

    let result = client.try_update_pool_description(
        &creator,
        &pool_id,
        &String::from_str(&env, ""),
    );
    assert_eq!(result, Err(Ok(PredifiError::InvalidAmount)));
}

#[test]
fn test_update_with_max_length_string_is_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);
    let pool_id = create_active_pool(&env, &client, &token_address, &creator);

    // Exactly 256 bytes is the documented inclusive maximum.
    let max_desc = String::from_str(&env, &"a".repeat(256));
    let result = client.try_update_pool_description(&creator, &pool_id, &max_desc);
    assert_eq!(result, Ok(Ok(())));

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.description.len(), 256);
}

#[test]
fn test_update_over_max_length_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);
    let pool_id = create_active_pool(&env, &client, &token_address, &creator);

    // 257 bytes exceeds the maximum and must be rejected without mutating state.
    let too_long = String::from_str(&env, &"a".repeat(257));
    let result = client.try_update_pool_description(&creator, &pool_id, &too_long);
    assert_eq!(result, Err(Ok(PredifiError::InvalidAmount)));

    let pool = client.get_pool(&pool_id);
    assert_eq!(
        pool.description,
        String::from_str(&env, "Original description")
    );
}

#[test]
fn test_update_with_special_characters() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);
    let pool_id = create_active_pool(&env, &client, &token_address, &creator);

    let special = String::from_str(&env, "Test!@#$%^&*()_+-=[]{}|;:',.<>?/~`");
    let result = client.try_update_pool_description(&creator, &pool_id, &special);
    assert_eq!(result, Ok(Ok(())));

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.description, special);
}

#[test]
fn test_non_creator_cannot_update() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);
    let pool_id = create_active_pool(&env, &client, &token_address, &creator);

    let stranger = Address::generate(&env);
    let result = client.try_update_pool_description(
        &stranger,
        &pool_id,
        &String::from_str(&env, "Unauthorized update"),
    );
    assert_eq!(result, Err(Ok(PredifiError::Unauthorized)));

    let pool = client.get_pool(&pool_id);
    assert_eq!(
        pool.description,
        String::from_str(&env, "Original description")
    );
}

#[test]
fn test_update_locked_after_prediction() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);
    let pool_id = create_active_pool(&env, &client, &token_address, &creator);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1_000_000i128);
    client.place_prediction(&user, &pool_id, &100i128, &0u32, &None, &None);

    let result = client.try_update_pool_description(
        &creator,
        &pool_id,
        &String::from_str(&env, "Attempted change after start"),
    );
    assert_eq!(result, Err(Ok(PredifiError::InvalidPoolState)));
}

#[test]
fn test_multiple_sequential_updates() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);
    let pool_id = create_active_pool(&env, &client, &token_address, &creator);

    let desc1 = String::from_str(&env, "First description");
    let desc2 = String::from_str(&env, "Second description");

    assert_eq!(
        client.try_update_pool_description(&creator, &pool_id, &desc1),
        Ok(Ok(()))
    );
    assert_eq!(
        client.try_update_pool_description(&creator, &pool_id, &desc2),
        Ok(Ok(()))
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.description, desc2);
}
