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
    assert_eq!(result, Err(Ok(PrediFiError::Unauthorized)));
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
    assert_eq!(result, Err(Ok(PrediFiError::Unauthorized)));
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

// ─── Role documentation tests ────────────────────────────────────────────────
// These tests verify that each role (Admin=0, Operator=1, Moderator=2,
// Oracle=3, User=4) can be assigned, detected, and revoked correctly, and
// that the numeric discriminants match the documented values.

/// Admin role (value 0) is automatically assigned on init and is detectable
/// via both `has_role` and `is_admin`.
#[test]
fn test_admin_role_value_and_assignment() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    // Admin role (0) is set automatically during init
    assert!(client.has_role(&admin, &Role::Admin));
    assert!(client.is_admin(&admin));

    // Numeric discriminant must be 0
    assert_eq!(Role::Admin as u32, 0);
}

/// Operator role (value 1) can be granted by admin and checked with has_role.
/// This role maps to resolve_pool / cancel_pool / set_stake_limits in predifi-contract.
#[test]
fn test_operator_role_value_and_assignment() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    client.init(&admin);

    // Numeric discriminant must be 1
    assert_eq!(Role::Operator as u32, 1);

    assert!(!client.has_role(&operator, &Role::Operator));
    client.assign_role(&admin, &operator, &Role::Operator);
    assert!(client.has_role(&operator, &Role::Operator));

    // Operator does not gain Admin privileges
    assert!(!client.has_role(&operator, &Role::Admin));
    assert!(!client.is_admin(&operator));
}

/// Moderator role (value 2) can be assigned and revoked independently.
#[test]
fn test_moderator_role_value_and_assignment() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let moderator = Address::generate(&env);
    client.init(&admin);

    // Numeric discriminant must be 2
    assert_eq!(Role::Moderator as u32, 2);

    client.assign_role(&admin, &moderator, &Role::Moderator);
    assert!(client.has_role(&moderator, &Role::Moderator));

    client.revoke_role(&admin, &moderator, &Role::Moderator);
    assert!(!client.has_role(&moderator, &Role::Moderator));
}

/// Oracle role (value 3) can be assigned and is independent of Operator.
/// This role maps to oracle_resolve (OracleCallback) in predifi-contract.
#[test]
fn test_oracle_role_value_and_assignment() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.init(&admin);

    // Numeric discriminant must be 3
    assert_eq!(Role::Oracle as u32, 3);

    client.assign_role(&admin, &oracle, &Role::Oracle);
    assert!(client.has_role(&oracle, &Role::Oracle));

    // Oracle does not gain Operator privileges
    assert!(!client.has_role(&oracle, &Role::Operator));
}

/// User role (value 4) can be assigned and revoked.
#[test]
fn test_user_role_value_and_assignment() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.init(&admin);

    // Numeric discriminant must be 4
    assert_eq!(Role::User as u32, 4);

    client.assign_role(&admin, &user, &Role::User);
    assert!(client.has_role(&user, &Role::User));

    client.revoke_role(&admin, &user, &Role::User);
    assert!(!client.has_role(&user, &Role::User));
}

/// Roles are independent: assigning one role does not grant any other role.
#[test]
fn test_roles_are_independent() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.init(&admin);

    client.assign_role(&admin, &user, &Role::Operator);

    // Only Operator is set; all other roles must be absent
    assert!(client.has_role(&user, &Role::Operator));
    assert!(!client.has_role(&user, &Role::Admin));
    assert!(!client.has_role(&user, &Role::Moderator));
    assert!(!client.has_role(&user, &Role::Oracle));
    assert!(!client.has_role(&user, &Role::User));
}

/// A single address can hold multiple roles simultaneously.
#[test]
fn test_multiple_roles_on_same_address() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let multi = Address::generate(&env);
    client.init(&admin);

    client.assign_role(&admin, &multi, &Role::Operator);
    client.assign_role(&admin, &multi, &Role::Oracle);
    client.assign_role(&admin, &multi, &Role::Moderator);

    assert!(client.has_role(&multi, &Role::Operator));
    assert!(client.has_role(&multi, &Role::Oracle));
    assert!(client.has_role(&multi, &Role::Moderator));
    assert!(!client.has_role(&multi, &Role::Admin));
}

/// `has_any_role` returns true when the user holds at least one of the queried roles.
/// Covers the Operator+Oracle combination used by predifi-contract resolution checks.
#[test]
fn test_has_any_role_operator_oracle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.init(&admin);

    client.assign_role(&admin, &oracle, &Role::Oracle);

    let mut roles = soroban_sdk::Vec::new(&env);
    roles.push_back(Role::Operator);
    roles.push_back(Role::Oracle);

    // Oracle role satisfies the check even though Operator is absent
    assert!(client.has_any_role(&oracle, &roles));
}

/// After `transfer_admin`, the new admin holds Admin role (0) and the old
/// admin loses it — verifying the documented admin-transfer behaviour.
#[test]
fn test_admin_transfer_updates_role_storage() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    client.init(&admin1);

    client.transfer_admin(&admin1, &admin2);

    assert!(client.has_role(&admin2, &Role::Admin));
    assert!(client.is_admin(&admin2));
    assert!(!client.has_role(&admin1, &Role::Admin));
    assert!(!client.is_admin(&admin1));
}

/// `revoke_all_roles` removes every role (0-4) from the target address.
#[test]
fn test_revoke_all_roles_clears_all_five_roles() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.init(&admin);

    for role in [Role::Operator, Role::Moderator, Role::Oracle, Role::User] {
        client.assign_role(&admin, &user, &role);
    }

    client.revoke_all_roles(&admin, &user);

    assert!(!client.has_role(&user, &Role::Admin));
    assert!(!client.has_role(&user, &Role::Operator));
    assert!(!client.has_role(&user, &Role::Moderator));
    assert!(!client.has_role(&user, &Role::Oracle));
    assert!(!client.has_role(&user, &Role::User));
}

