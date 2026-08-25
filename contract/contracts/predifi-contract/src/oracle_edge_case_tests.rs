//! Edge-case and boundary tests for `add_oracle` / `remove_oracle` (Issue #1321).

#![cfg(test)]

use crate::test::ROLE_ADMIN;
use crate::{DataKey, PredifiError};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, String};

#[test]
fn test_add_oracle_idempotent_no_duplicate_in_list() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let oracle = Address::generate(&env);

    // Add the same oracle twice — both calls must succeed (idempotent).
    client.add_oracle(&admin, &oracle);
    client.add_oracle(&admin, &oracle);
    // A third add still succeeds without error.
    client.add_oracle(&admin, &oracle);

    let whitelist: soroban_sdk::Vec<Address> = env
        .storage()
        .persistent()
        .get(&DataKey::OracleWhitelist)
        .unwrap();
    assert_eq!(whitelist.len(), 1);
    assert_eq!(whitelist.get(0), Some(oracle));
}

#[test]
fn test_remove_oracle_while_emergency_cancel_vote_is_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _token, _token_admin, _treasury, operator, creator) =
        crate::test::setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    let operator2 = Address::generate(&env);
    ac_client.grant_role(&operator2, &crate::test::ROLE_OPERATOR);
    let oracle = Address::generate(&env);
    client.add_oracle(&admin, &oracle);

    let pool_id = client.create_pool(
        &creator,
        &7_200u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &crate::PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "oracle removal vote"),
            metadata_url: String::from_str(&env, "ipfs://oracle-removal"),
            min_stake: 1,
            max_stake: 0,
            max_total_stake: 0,
            min_total_stake: 0,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "No"),
                String::from_str(&env, "Yes"),
            ],
        },
    );
    client.emergency_cancel_pool(&operator, &pool_id, &String::from_str(&env, "pending"));
    assert_eq!(client.get_emergency_cancel_approvals(&pool_id).len(), 1);

    client.remove_oracle(&admin, &oracle);
    assert_eq!(client.get_emergency_cancel_approvals(&pool_id).len(), 1);
    assert_eq!(
        client.try_update_price_feed(
            &oracle,
            &symbol_short!("ETHUSD"),
            &3_000i128,
            &1i128,
            &0u64,
            &60u64,
        ),
        Err(Ok(PredifiError::Unauthorized))
    );
}

#[test]
fn test_add_maximum_supported_oracle_list_and_remove_last() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // No protocol oracle-count cap exists; exercise a practical list boundary.
    let mut oracles = soroban_sdk::Vec::new(&env);
    for _ in 0..64 {
        oracles.push_back(Address::generate(&env));
    }
    for oracle in oracles.iter() {
        client.add_oracle(&admin, &oracle);
    }
    let stored: soroban_sdk::Vec<Address> = env
        .storage()
        .persistent()
        .get(&DataKey::OracleWhitelist)
        .unwrap();
    assert_eq!(stored.len(), 64);

    for oracle in oracles.iter() {
        client.remove_oracle(&admin, &oracle);
    }
    let stored: soroban_sdk::Vec<Address> = env
        .storage()
        .persistent()
        .get(&DataKey::OracleWhitelist)
        .unwrap();
    assert_eq!(stored.len(), 0);
}

#[test]
fn test_add_and_remove_single_oracle() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let oracle = Address::generate(&env);

    client.add_oracle(&admin, &oracle);
    // Removing the only oracle should succeed — empty list is valid.
    client.remove_oracle(&admin, &oracle);
    // Re-adding after removal must also succeed.
    client.add_oracle(&admin, &oracle);
}

#[test]
fn test_add_multiple_oracles_and_remove_middle() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let oracle3 = Address::generate(&env);

    client.add_oracle(&admin, &oracle1);
    client.add_oracle(&admin, &oracle2);
    client.add_oracle(&admin, &oracle3);

    // Remove the middle oracle; remaining two should still be manageable.
    client.remove_oracle(&admin, &oracle2);
    client.remove_oracle(&admin, &oracle1);
    client.remove_oracle(&admin, &oracle3);
}

#[test]
fn test_remove_non_existent_oracle_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let never_added = Address::generate(&env);
    // Removing an oracle that was never added must not panic.
    client.remove_oracle(&admin, &never_added);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_add_oracle_non_admin_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);

    // Random address with no role — must be rejected with Unauthorized (#10).
    let non_admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.add_oracle(&non_admin, &oracle);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_oracle_role_cannot_manage_whitelist() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let oracle_role_holder = Address::generate(&env);
    ac_client.grant_role(&oracle_role_holder, &3u32);

    client.add_oracle(&oracle_role_holder, &Address::generate(&env));
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_remove_oracle_non_admin_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    let oracle = Address::generate(&env);
    client.add_oracle(&admin, &oracle);

    // A non-admin attempting removal must be rejected.
    let non_admin = Address::generate(&env);
    client.remove_oracle(&non_admin, &oracle);
}
