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
    assert!(result.is_err()); // init panics with an error code, which results in an InvokeError

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
    assert_eq!(result, Err(Ok(PrediFiError::Unauthorized.into())));

}
#[test]
fn test_is_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);

    client.init(&admin);

    // Check that admin is recognized as admin
    assert!(client.is_admin(&admin));

    // Check that non-admin is not admin
    assert!(!client.is_admin(&non_admin));
}

#[test]
fn test_revoke_all_roles() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    client.init(&admin);

    // Assign multiple roles to user
    client.assign_role(&admin, &user, &Role::Operator);
    client.assign_role(&admin, &user, &Role::Moderator);

    assert!(client.has_role(&user, &Role::Operator));
    assert!(client.has_role(&user, &Role::Moderator));

    // Revoke all roles at once
    client.revoke_all_roles(&admin, &user);

    // Verify all roles are removed
    assert!(!client.has_role(&user, &Role::Operator));
    assert!(!client.has_role(&user, &Role::Moderator));
    assert!(!client.has_role(&user, &Role::Admin));
}

#[test]
fn test_revoke_all_roles_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let user = Address::generate(&env);

    client.init(&admin);
    client.assign_role(&admin, &user, &Role::Operator);

    // Non-admin tries to revoke all roles
    let result = client.try_revoke_all_roles(&non_admin, &user);
    assert_eq!(result, Err(Ok(PrediFiError::Unauthorized.into())));

}

#[test]
fn test_has_any_role() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    client.init(&admin);

    // User has no roles yet
    let mut empty_roles = soroban_sdk::Vec::new(&env);
    empty_roles.push_back(Role::Operator);
    empty_roles.push_back(Role::Moderator);
    assert!(!client.has_any_role(&user, &empty_roles));

    // Assign Operator role to user
    client.assign_role(&admin, &user, &Role::Operator);

    // Now user should have any role (Operator is in the list)
    assert!(client.has_any_role(&user, &empty_roles));

    // Create a list with roles user doesn't have
    let mut other_roles = soroban_sdk::Vec::new(&env);
    other_roles.push_back(Role::Admin);
    other_roles.push_back(Role::Moderator);

    // User still has Operator (not in this list), so should return false
    assert!(!client.has_any_role(&user, &other_roles));

    // Add Admin to the list - still false because user is not admin
    let mut mixed_roles = soroban_sdk::Vec::new(&env);
    mixed_roles.push_back(Role::Operator);
    mixed_roles.push_back(Role::Admin);

    // Now should be true because user has Operator
    assert!(client.has_any_role(&user, &mixed_roles));
}
