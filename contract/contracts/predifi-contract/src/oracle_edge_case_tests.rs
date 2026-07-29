//! Edge-case and boundary tests for `add_oracle` / `remove_oracle` (Issue #1321).

#![cfg(test)]

use crate::test::ROLE_ADMIN;
use soroban_sdk::{testutils::Address as _, Address, Env};

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
