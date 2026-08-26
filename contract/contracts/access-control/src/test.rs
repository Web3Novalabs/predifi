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
fn test_propose_and_accept_admin_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.init(&admin1);
    client.propose_new_admin(&admin1, &admin2);

    assert_eq!(client.get_admin(), admin1);
    assert_eq!(client.get_proposed_admin(), Some(admin2.clone()));

    client.accept_admin_role(&admin2);

    assert_eq!(client.get_admin(), admin2.clone());
    assert_eq!(client.get_proposed_admin(), None);
    assert!(client.has_role(&admin2, &Role::Admin));
    assert!(!client.has_role(&admin1, &Role::Admin));
}

#[test]
fn test_only_proposed_admin_can_accept_admin_role() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let random_user = Address::generate(&env);

    client.init(&admin1);
    client.propose_new_admin(&admin1, &admin2);

    let result = client.try_accept_admin_role(&random_user);
    assert_eq!(result, Err(Ok(PrediFiError::Unauthorized)));

    assert_eq!(client.get_admin(), admin1);
    assert_eq!(client.get_proposed_admin(), Some(admin2));
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

/// Revoking a role that was never assigned returns `InsufficientPermissions`.
#[test]
fn test_revoke_unassigned_role_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.init(&admin);

    let result = client.try_revoke_role(&admin, &user, &Role::Operator);
    assert_eq!(result, Err(Ok(PrediFiError::InsufficientPermissions)));
}

/// Transferring a role that the `from` address does not hold returns
/// `InsufficientPermissions`.
#[test]
fn test_transfer_role_from_address_without_role_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    client.init(&admin);

    let result = client.try_transfer_role(&admin, &from, &to, &Role::Oracle);
    assert_eq!(result, Err(Ok(PrediFiError::InsufficientPermissions)));
}

// ─── get_operator_count tests ─────────────────────────────────────────────────

/// get_operator_count returns 0 when no operators have been assigned.
#[test]
fn test_get_operator_count_initially_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    assert_eq!(client.get_operator_count(), 0);
}

/// Assigning the Operator role increments the count.
#[test]
fn test_get_operator_count_increments_on_assign() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let op1 = Address::generate(&env);
    let op2 = Address::generate(&env);
    let op3 = Address::generate(&env);
    client.init(&admin);

    assert_eq!(client.get_operator_count(), 0);

    client.assign_role(&admin, &op1, &Role::Operator);
    assert_eq!(client.get_operator_count(), 1);

    client.assign_role(&admin, &op2, &Role::Operator);
    assert_eq!(client.get_operator_count(), 2);

    client.assign_role(&admin, &op3, &Role::Operator);
    assert_eq!(client.get_operator_count(), 3);
}

/// Assigning a non-Operator role does not affect the operator count.
#[test]
fn test_get_operator_count_unaffected_by_other_roles() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.init(&admin);

    client.assign_role(&admin, &user, &Role::Oracle);
    client.assign_role(&admin, &user, &Role::Moderator);
    client.assign_role(&admin, &user, &Role::User);

    assert_eq!(client.get_operator_count(), 0);
}

/// Revoking the Operator role decrements the count.
#[test]
fn test_get_operator_count_decrements_on_revoke() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let op1 = Address::generate(&env);
    let op2 = Address::generate(&env);
    client.init(&admin);

    client.assign_role(&admin, &op1, &Role::Operator);
    client.assign_role(&admin, &op2, &Role::Operator);
    assert_eq!(client.get_operator_count(), 2);

    client.revoke_role(&admin, &op1, &Role::Operator);
    assert_eq!(client.get_operator_count(), 1);

    client.revoke_role(&admin, &op2, &Role::Operator);
    assert_eq!(client.get_operator_count(), 0);
}

/// revoke_all_roles decrements operator count when the user held the Operator role.
#[test]
fn test_get_operator_count_decrements_on_revoke_all() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let op = Address::generate(&env);
    client.init(&admin);

    client.assign_role(&admin, &op, &Role::Operator);
    client.assign_role(&admin, &op, &Role::Oracle);
    assert_eq!(client.get_operator_count(), 1);

    client.revoke_all_roles(&admin, &op);
    assert_eq!(client.get_operator_count(), 0);
}

/// Assigning the Operator role to the same address twice does not double-count.
#[test]
fn test_get_operator_count_no_double_count_on_reassign() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let op = Address::generate(&env);
    client.init(&admin);

    client.assign_role(&admin, &op, &Role::Operator);
    assert_eq!(client.get_operator_count(), 1);

    // Assigning again to the same address should not increment
    client.assign_role(&admin, &op, &Role::Operator);
    assert_eq!(client.get_operator_count(), 1);
}

/// transfer_role moves the Operator role without changing the count.
#[test]
fn test_get_operator_count_stable_on_transfer_role() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let op_from = Address::generate(&env);
    let op_to = Address::generate(&env);
    client.init(&admin);

    client.assign_role(&admin, &op_from, &Role::Operator);
    assert_eq!(client.get_operator_count(), 1);

    client.transfer_role(&admin, &op_from, &op_to, &Role::Operator);
    // Count should remain 1: one operator removed, one added
    assert_eq!(client.get_operator_count(), 1);
    assert!(!client.has_role(&op_from, &Role::Operator));
    assert!(client.has_role(&op_to, &Role::Operator));
}

// ─── Admin removal tests ──────────────────────────────────────────────────────
//
// These tests verify that an admin can be successfully removed from the
// Admin role through the various available mechanisms, and that the
// removal is correctly reflected in both `has_role` and `is_admin`.

/// The admin can revoke the Admin role from a secondary address that was
/// explicitly granted Admin via `assign_role`.
///
/// Flow: init → assign Admin to user → revoke Admin from user
/// Expected: user loses Admin role; original admin is unaffected.
#[test]
fn test_admin_can_revoke_admin_role_from_another_address() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let second_admin = Address::generate(&env);
    client.init(&admin);

    // Grant Admin role to a second address.
    client.assign_role(&admin, &second_admin, &Role::Admin);
    assert!(client.has_role(&second_admin, &Role::Admin));
    assert!(client.is_admin(&second_admin) || client.has_role(&second_admin, &Role::Admin));

    // Remove the Admin role from the second address.
    client.revoke_role(&admin, &second_admin, &Role::Admin);

    // The second address must no longer hold the Admin role.
    assert!(
        !client.has_role(&second_admin, &Role::Admin),
        "Admin role should be removed from second_admin after revoke_role"
    );

    // The original admin must be unaffected.
    assert!(
        client.has_role(&admin, &Role::Admin),
        "Original admin should still hold the Admin role"
    );
    assert!(
        client.is_admin(&admin),
        "Original admin should still be the contract admin"
    );
}

/// `revoke_all_roles` removes the Admin role from a target address that
/// holds it, along with any other roles that address may have.
///
/// Flow: init → assign Admin + Operator to user → revoke_all_roles
/// Expected: user loses both roles; original admin is unaffected.
#[test]
fn test_revoke_all_roles_removes_admin_role() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let target = Address::generate(&env);
    client.init(&admin);

    // Give the target both Admin and Operator roles.
    client.assign_role(&admin, &target, &Role::Admin);
    client.assign_role(&admin, &target, &Role::Operator);
    assert!(client.has_role(&target, &Role::Admin));
    assert!(client.has_role(&target, &Role::Operator));

    // Strip all roles from the target.
    client.revoke_all_roles(&admin, &target);

    assert!(
        !client.has_role(&target, &Role::Admin),
        "Admin role should be removed by revoke_all_roles"
    );
    assert!(
        !client.has_role(&target, &Role::Operator),
        "Operator role should also be removed by revoke_all_roles"
    );

    // The original admin must be unaffected.
    assert!(
        client.has_role(&admin, &Role::Admin),
        "Original admin should still hold the Admin role"
    );
}

/// After `transfer_admin`, the old admin's Admin role is removed from
/// persistent storage and `is_admin` returns false for them.
///
/// This is the primary "admin removal" path: transferring admin rights
/// atomically removes the role from the previous holder.
#[test]
fn test_transfer_admin_removes_admin_role_from_old_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let old_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.init(&old_admin);

    // Confirm initial state.
    assert!(client.has_role(&old_admin, &Role::Admin));
    assert!(client.is_admin(&old_admin));
    assert!(!client.has_role(&new_admin, &Role::Admin));

    client.transfer_admin(&old_admin, &new_admin);

    // Old admin must have lost the Admin role entirely.
    assert!(
        !client.has_role(&old_admin, &Role::Admin),
        "Old admin should no longer hold the Admin role after transfer_admin"
    );
    assert!(
        !client.is_admin(&old_admin),
        "is_admin should return false for the old admin after transfer_admin"
    );

    // New admin must hold the Admin role.
    assert!(
        client.has_role(&new_admin, &Role::Admin),
        "New admin should hold the Admin role after transfer_admin"
    );
    assert!(
        client.is_admin(&new_admin),
        "is_admin should return true for the new admin after transfer_admin"
    );
}

/// After the two-step propose/accept flow, the proposing admin's Admin role
/// is removed and the accepting address becomes the sole admin.
#[test]
fn test_accept_admin_role_removes_admin_role_from_proposer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let old_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.init(&old_admin);

    client.propose_new_admin(&old_admin, &new_admin);
    client.accept_admin_role(&new_admin);

    // Old admin must have lost the Admin role.
    assert!(
        !client.has_role(&old_admin, &Role::Admin),
        "Old admin should no longer hold the Admin role after accept_admin_role"
    );
    assert!(
        !client.is_admin(&old_admin),
        "is_admin should return false for the old admin after accept_admin_role"
    );

    // New admin must hold the Admin role.
    assert!(
        client.has_role(&new_admin, &Role::Admin),
        "New admin should hold the Admin role after accept_admin_role"
    );
    assert!(
        client.is_admin(&new_admin),
        "is_admin should return true for the new admin after accept_admin_role"
    );
}

/// A non-admin cannot remove the Admin role from any address.
/// `revoke_role` must return `Unauthorized` when called by a non-admin.
#[test]
fn test_non_admin_cannot_revoke_admin_role() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    client.init(&admin);

    // Attacker attempts to strip the admin of their Admin role.
    let result = client.try_revoke_role(&attacker, &admin, &Role::Admin);
    assert_eq!(
        result,
        Err(Ok(PrediFiError::Unauthorized)),
        "Non-admin must not be able to revoke the Admin role"
    );

    // Admin role must be intact.
    assert!(
        client.has_role(&admin, &Role::Admin),
        "Admin role must remain after an unauthorized revoke attempt"
    );
    assert!(
        client.is_admin(&admin),
        "is_admin must still return true after an unauthorized revoke attempt"
    );
}

/// Revoking the Admin role from an address that never held it returns
/// `InsufficientPermissions` — consistent with how other roles behave.
#[test]
fn test_revoke_admin_role_from_address_without_it_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.init(&admin);

    // `user` was never granted the Admin role.
    let result = client.try_revoke_role(&admin, &user, &Role::Admin);
    assert_eq!(
        result,
        Err(Ok(PrediFiError::InsufficientPermissions)),
        "Revoking Admin role from an address that never held it should return InsufficientPermissions"
    );
}

/// After the Admin role is removed from a secondary address, that address
/// can no longer perform admin-only operations such as assigning roles.
#[test]
fn test_removed_admin_cannot_assign_roles() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let ex_admin = Address::generate(&env);
    let victim = Address::generate(&env);
    client.init(&admin);

    // Grant Admin role to ex_admin, then immediately revoke it.
    client.assign_role(&admin, &ex_admin, &Role::Admin);
    assert!(client.has_role(&ex_admin, &Role::Admin));

    client.revoke_role(&admin, &ex_admin, &Role::Admin);
    assert!(!client.has_role(&ex_admin, &Role::Admin));

    // ex_admin must no longer be able to assign roles.
    let result = client.try_assign_role(&ex_admin, &victim, &Role::Operator);
    assert_eq!(
        result,
        Err(Ok(PrediFiError::Unauthorized)),
        "An address whose Admin role was revoked must not be able to assign roles"
    );
}
