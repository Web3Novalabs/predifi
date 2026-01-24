#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

#[test]
fn test_initialization() {
    let env = Env::default();
    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_double_initialization() {
    let env = Env::default();
    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);
    let result = client.try_init(&admin);
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_role_assignment() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    client.init(&admin);

    client.assign_role(&admin, &user, &Role::Operator);
    assert!(client.has_role(&user, &Role::Operator));
}

#[test]
fn test_role_revocation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    client.init(&admin);

    client.assign_role(&admin, &user, &Role::Operator);
    assert!(client.has_role(&user, &Role::Operator));

    client.revoke_role(&admin, &user, &Role::Operator);
    assert!(!client.has_role(&user, &Role::Operator));
}

#[test]
fn test_role_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    client.init(&admin);

    client.assign_role(&admin, &user1, &Role::Operator);
    assert!(client.has_role(&user1, &Role::Operator));

    client.transfer_role(&admin, &user1, &user2, &Role::Operator);
    assert!(!client.has_role(&user1, &Role::Operator));
    assert!(client.has_role(&user2, &Role::Operator));
}

#[test]
fn test_admin_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.init(&admin1);
    assert_eq!(client.get_admin(), admin1);

    client.transfer_admin(&admin1, &admin2);
    assert_eq!(client.get_admin(), admin2);
    assert!(client.has_role(&admin2, &Role::Admin));
    assert!(!client.has_role(&admin1, &Role::Admin));
}

#[test]
fn test_unauthorized_assignment() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let user = Address::generate(&env);

    client.init(&admin);

    // non_admin tries to assign a role
    let result = client.try_assign_role(&non_admin, &user, &Role::Operator);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_get_pools() {
    let env = Env::default();
    let contract_id = env.register_contract(None, AccessControl);
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    // Test with valid parameters
    let result = client.get_pools(&0, &50, &None, &None);
    assert_eq!(result.len(), 0); // Empty result as expected
}
