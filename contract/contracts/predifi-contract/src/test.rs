#![cfg(test)]
#![allow(deprecated)]

use super::*;
use predifi_errors::PrediFiError;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Symbol,
};

/// Helper to generate consistent metadata for tests
fn get_metadata(env: &Env) -> (Symbol, u32) {
    (Symbol::new(env, "general"), 2)
}

#[test]
fn test_claim_winnings() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let creator = Address::generate(&env);
    let (category, options) = get_metadata(&env);

    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    let treasury = Address::generate(&env);
    client.init(&treasury, &0u32);

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles to test users
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Operator); // Add Oracle role for resolve_pool
    access_control_client.assign_role(&creator, &user1, &access_control::Role::Moderator);
    access_control_client.assign_role(&creator, &user2, &access_control::Role::Moderator);
    // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    // Updated with 6 arguments (including min_stake = 0)
    let pool_id = client.create_pool(&creator, &100, &token_address, &category, &options, &0);

    client.place_prediction(&user1, &user1, &pool_id, &100, &1);
    client.place_prediction(&user2, &user2, &pool_id, &100, &2);

    assert_eq!(token.balance(&contract_id), 200);
    env.ledger().set_timestamp(101);
    client.resolve_pool(&creator, &pool_id, &1);

    let winnings = client.claim_winnings(&user1, &pool_id);
    assert_eq!(winnings, 200);
    assert_eq!(token.balance(&user1), 1100);

    let result = client.try_claim_winnings(&user2, &pool_id);
    assert_eq!(result, Err(Ok(PrediFiError::NotAWinner)));
}

#[test]
fn test_double_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user1 = Address::generate(&env);
    let creator = Address::generate(&env);
    let (category, options) = get_metadata(&env);
    token_admin_client.mint(&user1, &1000);

    let treasury = Address::generate(&env);
    client.init(&treasury, &0u32);

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles to test users
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Operator); // Add Oracle role for resolve_pool
    access_control_client.assign_role(&creator, &user1, &access_control::Role::Moderator);
    // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    let pool_id = client.create_pool(&creator, &100, &token_address, &category, &options, &0);
    client.place_prediction(&user1, &user1, &pool_id, &100, &1);

    env.ledger().set_timestamp(101);
    client.resolve_pool(&creator, &pool_id, &1);

    client.claim_winnings(&user1, &pool_id);

    let result = client.try_claim_winnings(&user1, &pool_id);
    assert_eq!(result, Err(Ok(PrediFiError::AlreadyClaimed)));
}

#[test]
fn test_claim_unresolved() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user1 = Address::generate(&env);
    let creator = Address::generate(&env);
    let (category, options) = get_metadata(&env);
    token_admin_client.mint(&user1, &1000);

    let treasury = Address::generate(&env);
    client.init(&treasury, &0u32);

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles to test users
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &user1, &access_control::Role::Moderator);
    // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    let pool_id = client.create_pool(&creator, &100, &token_address, &category, &options, &0);
    client.place_prediction(&user1, &user1, &pool_id, &100, &1);

    let result = client.try_claim_winnings(&user1, &pool_id);
    assert_eq!(result, Err(Ok(PrediFiError::PoolNotResolved)));
}

#[test]
fn test_get_user_predictions() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user = Address::generate(&env);
    let creator = Address::generate(&env);
    let (category, options) = get_metadata(&env);
    token_admin_client.mint(&user, &1000);

    let treasury = Address::generate(&env);
    client.init(&treasury, &0u32);

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles to test users
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &user, &access_control::Role::Moderator);
    // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    let pool0 = client.create_pool(&creator, &100, &token_address, &category, &options, &0);
    let pool1 = client.create_pool(&creator, &200, &token_address, &category, &options, &0);
    let pool2 = client.create_pool(&creator, &300, &token_address, &category, &options, &0);

    client.place_prediction(&user, &user, &pool0, &10, &1);
    client.place_prediction(&user, &user, &pool1, &20, &2);
    client.place_prediction(&user, &user, &pool2, &30, &1);

    let all = client.get_user_predictions(&user, &0, &10);
    assert_eq!(all.len(), 3);
}

#[test]
fn test_claim_with_fees() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let creator = Address::generate(&env);
    let (category, options) = get_metadata(&env);
    let treasury = Address::generate(&env);

    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    let fee_bps = 100u32; // 1%
    client.init(&treasury, &fee_bps);

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles to test users
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Operator); // Add Oracle role for resolve_pool
    access_control_client.assign_role(&creator, &user1, &access_control::Role::Moderator);
    access_control_client.assign_role(&creator, &user2, &access_control::Role::Moderator);
    // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    let pool_id = client.create_pool(&creator, &100, &token_address, &category, &options, &0);

    client.place_prediction(&user1, &user1, &pool_id, &100, &1);
    client.place_prediction(&user2, &user2, &pool_id, &100, &2);

    env.ledger().set_timestamp(101);
    client.resolve_pool(&creator, &pool_id, &1);

    let winnings = client.claim_winnings(&user1, &pool_id);
    assert_eq!(winnings, 198);
    assert_eq!(token.balance(&treasury), 2);
}

#[test]
fn test_resolve_pool_validation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;

    let creator = Address::generate(&env);
    let (category, options) = get_metadata(&env);
    let treasury = Address::generate(&env);
    client.init(&treasury, &0);

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles to test users
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Operator); // Add Oracle role for resolve_pool
                                                                                            // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    let pool_id = client.create_pool(&creator, &100, &token_address, &category, &options, &0);

    env.ledger().set_timestamp(50);
    let result = client.try_resolve_pool(&creator, &pool_id, &1);
    assert_eq!(result, Err(Ok(PrediFiError::PoolNotExpired)));

    env.ledger().set_timestamp(100);
    client.resolve_pool(&creator, &pool_id, &1);

    let result = client.try_resolve_pool(&creator, &pool_id, &1);
    assert_eq!(result, Err(Ok(PrediFiError::PoolAlreadyResolved)));
}

#[test]
fn test_create_pool_validation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;

    let creator = Address::generate(&env);
    let (category, _) = get_metadata(&env);
    let treasury = Address::generate(&env);
    client.init(&treasury, &0);

    env.ledger().set_timestamp(100);

    // Past end_time
    let result = client.try_create_pool(&creator, &50, &token_address, &category, &2, &0);
    assert_eq!(result, Err(Ok(PrediFiError::EndTimeMustBeFuture)));

    // Invalid options count
    let result = client.try_create_pool(&creator, &200, &token_address, &category, &1, &0);
    assert_eq!(result, Err(Ok(PrediFiError::InvalidOptionsCount)));

    // Valid
    let result = client.try_create_pool(&creator, &200, &token_address, &category, &2, &0);
    assert!(result.is_ok());
}

#[test]
fn test_place_prediction_validation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user = Address::generate(&env);
    let creator = Address::generate(&env);
    let (category, options) = get_metadata(&env);
    token_admin_client.mint(&user, &1000);

    let treasury = Address::generate(&env);
    client.init(&treasury, &0);

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles to test users
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &user, &access_control::Role::Moderator);
    // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    let pool_id = client.create_pool(&creator, &100, &token_address, &category, &options, &0);

    let result = client.try_place_prediction(&user, &user, &pool_id, &0, &1);
    assert_eq!(result, Err(Ok(PrediFiError::InvalidPredictionAmount)));

    client.place_prediction(&user, &user, &pool_id, &100, &1);

    let result = client.try_place_prediction(&user, &user, &pool_id, &50, &2);
    assert_eq!(result, Err(Ok(PrediFiError::PredictionAlreadyExists)));

    env.ledger().set_timestamp(101);
    let result = client.try_place_prediction(&user, &user, &pool_id, &100, &1);
    assert_eq!(result, Err(Ok(PrediFiError::PredictionTooLate)));
}

#[test]
fn test_place_prediction_min_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user = Address::generate(&env);
    let creator = Address::generate(&env);
    let (category, options) = get_metadata(&env);
    token_admin_client.mint(&user, &1000);

    let treasury = Address::generate(&env);
    client.init(&treasury, &0);

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles to test users
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Operator);
    access_control_client.assign_role(&creator, &user, &access_control::Role::Moderator);
    // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    // Create pool with min_stake = 100
    let pool_id = client.create_pool(&creator, &200, &token_address, &category, &options, &100);

    // Try to place prediction with amount 50 (should fail)
    let result = client.try_place_prediction(&user, &user, &pool_id, &50, &1);
    assert_eq!(result, Err(Ok(PrediFiError::MinStakeNotMet)));

    // Place prediction with amount 100 (should succeed)
    let result = client.try_place_prediction(&user, &user, &pool_id, &100, &1);
    assert!(result.is_ok());
}

#[test]
fn test_zero_bet_resolution() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);
    let (category, options) = get_metadata(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&Address::generate(&env), &100); // 1% fee

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Operator);
    // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    let pool_id = client.create_pool(&creator, &100, &token, &category, &options, &0);

    env.ledger().set_timestamp(101);
    // Should succeed even with 0 bets
    let result = client.try_resolve_pool(&creator, &pool_id, &1);
    assert!(result.is_ok());
}

#[test]
fn test_resolution_window_expiry() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);
    let (category, options) = get_metadata(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&Address::generate(&env), &0);

    // Initialize access control contract
    let access_control_contract_id = env.register(access_control::AccessControl, ());
    let access_control_client =
        access_control::AccessControlClient::new(&env, &access_control_contract_id);
    access_control_client.init(&creator);
    // Assign roles
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Admin);
    access_control_client.assign_role(&creator, &creator, &access_control::Role::Operator);
    // Set access control contract in PrediFi
    client.set_access_control(&access_control_contract_id);

    let pool_id = client.create_pool(&creator, &100, &token, &category, &options, &0);

    // Advance time past 7 days (100 + 604800 + 1)
    let seven_days_in_seconds = 7 * 24 * 60 * 60;
    env.ledger().set_timestamp(100 + seven_days_in_seconds + 1);

    let result = client.try_resolve_pool(&creator, &pool_id, &1);
    assert_eq!(result, Err(Ok(PrediFiError::ResolutionWindowExpired)));
}

#[test]
#[should_panic] // Soroban panics on auth failure in tests if not mocked correctly
fn test_unauthorized_pool_creation() {
    let env = Env::default();
    // env.mock_all_auths(); // WE DO NOT CALL THIS to test real auth failure

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);
    let (category, options) = get_metadata(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&Address::generate(&env), &0);

    // Attempt to create pool without auth
    client.create_pool(&creator, &200, &token, &category, &options, &0);
}
