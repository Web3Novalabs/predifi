#![cfg(test)]
#![allow(deprecated)]

use super::*;
use predifi_errors::PrediFiError;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Env,
};

#[test]
fn test_claim_winnings() {
    let env = Env::default();
    env.mock_all_auths();

    // Register contract
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    // Setup Token
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    // Setup Users
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // Mint tokens to users
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    // Init contract with treasury and zero fees for this test
    let treasury = Address::generate(&env);
    let fee_bps = 0u32;
    client.init(&treasury, &fee_bps);

    // Create Pool with end_time = 100
    let pool_id = client.create_pool(&100, &token_address);

    // Place Predictions
    // User 1 bets 100 on Outcome 1
    client.place_prediction(&user1, &pool_id, &100, &1);

    // User 2 bets 100 on Outcome 2
    client.place_prediction(&user2, &pool_id, &100, &2);

    // Check balances (contract should have 200)
    assert_eq!(token.balance(&contract_id), 200);

    // Advance time past pool end_time to allow resolution
    env.ledger().set_timestamp(101);

    // Resolve Pool - Outcome 1 wins
    client.resolve_pool(&pool_id, &1);

    // User 1 Claims
    let winnings = client.claim_winnings(&user1, &pool_id);

    // Total pool is 200. Winning stake is 100. User 1 stake is 100.
    // Share = (100 / 100) * 200 = 200.
    assert_eq!(winnings, 200);
    assert_eq!(token.balance(&user1), 1100); // Initial 1000 - 100 bet + 200 winnings

    // User 2 Claims (Expect error - they lost)
    let result = client.try_claim_winnings(&user2, &pool_id);
    assert_eq!(result, Err(Ok(PrediFiError::NotAWinner)));
    assert_eq!(token.balance(&user2), 900); // Initial 1000 - 100 bet
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
    token_admin_client.mint(&user1, &1000);

    let treasury = Address::generate(&env);
    let fee_bps = 0u32;
    client.init(&treasury, &fee_bps);

    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);

    // Advance time past pool end_time to allow resolution
    env.ledger().set_timestamp(101);

    client.resolve_pool(&pool_id, &1);

    // First claim should succeed
    client.claim_winnings(&user1, &pool_id);

    // Second claim should fail with AlreadyClaimed error
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
    token_admin_client.mint(&user1, &1000);

    let treasury = Address::generate(&env);
    let fee_bps = 0u32;
    client.init(&treasury, &fee_bps);

    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);

    // Do NOT resolve - should fail with PoolNotResolved error
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
    token_admin_client.mint(&user, &1000);

    let treasury = Address::generate(&env);
    let fee_bps = 0u32;
    client.init(&treasury, &fee_bps);

    // Create 3 pools and place predictions
    let pool0 = client.create_pool(&100, &token_address);
    let pool1 = client.create_pool(&200, &token_address);
    let pool2 = client.create_pool(&300, &token_address);

    client.place_prediction(&user, &pool0, &10, &1);
    client.place_prediction(&user, &pool1, &20, &2);
    client.place_prediction(&user, &pool2, &30, &1);

    // Test pagination: All 3
    let all = client.get_user_predictions(&user, &0, &10);
    assert_eq!(all.len(), 3);
    assert_eq!(all.get(0).unwrap().pool_id, pool0);
    assert_eq!(all.get(1).unwrap().pool_id, pool1);
    assert_eq!(all.get(2).unwrap().pool_id, pool2);
    assert_eq!(all.get(0).unwrap().amount, 10);
    assert_eq!(all.get(0).unwrap().user_outcome, 1);

    // Test pagination: Limit 2
    let first_two = client.get_user_predictions(&user, &0, &2);
    assert_eq!(first_two.len(), 2);
    assert_eq!(first_two.get(0).unwrap().pool_id, pool0);
    assert_eq!(first_two.get(1).unwrap().pool_id, pool1);

    // Test pagination: Offset 1, Limit 2
    let last_two = client.get_user_predictions(&user, &1, &2);
    assert_eq!(last_two.len(), 2);
    assert_eq!(last_two.get(0).unwrap().pool_id, pool1);
    assert_eq!(last_two.get(1).unwrap().pool_id, pool2);

    // Test pagination: Offset 2, Limit 1
    let last_one = client.get_user_predictions(&user, &2, &1);
    assert_eq!(last_one.len(), 1);
    assert_eq!(last_one.get(0).unwrap().pool_id, pool2);

    // Test pagination: Out of bounds
    let empty = client.get_user_predictions(&user, &3, &1);
    assert_eq!(empty.len(), 0);
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
    let treasury = Address::generate(&env);

    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    // Init with 1% fee (100 basis points)
    let fee_bps = 100u32;
    client.init(&treasury, &fee_bps);

    let pool_id = client.create_pool(&100, &token_address);

    client.place_prediction(&user1, &pool_id, &100, &1);
    client.place_prediction(&user2, &pool_id, &100, &2);

    // Advance time and resolve
    env.ledger().set_timestamp(101);
    client.resolve_pool(&pool_id, &1);

    // User 1 claims
    let winnings = client.claim_winnings(&user1, &pool_id);

    // Total pool: 200
    // Gross winnings: 200 (user won all)
    // Fee: 200 * 100 / 10000 = 2
    // User's share of fee: (200/200) * 2 = 2
    // Net winnings: 200 - 2 = 198
    assert_eq!(winnings, 198);

    // Check treasury received fee
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

    let treasury = Address::generate(&env);
    client.init(&treasury, &0);

    let pool_id = client.create_pool(&100, &token_address);

    // Try to resolve before end time (should fail with PoolNotExpired)
    env.ledger().set_timestamp(50);
    let result = client.try_resolve_pool(&pool_id, &1);
    assert_eq!(result, Err(Ok(PrediFiError::PoolNotExpired)));

    // Resolve at exactly end time
    env.ledger().set_timestamp(100);
    client.resolve_pool(&pool_id, &1);

    // Try to resolve again (should fail with PoolAlreadyResolved)
    let result = client.try_resolve_pool(&pool_id, &1);
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

    let treasury = Address::generate(&env);
    client.init(&treasury, &0);

    // Set current time
    env.ledger().set_timestamp(100);

    // Try to create pool with end_time in the past (should fail)
    let result = client.try_create_pool(&50, &token_address);
    assert_eq!(result, Err(Ok(PrediFiError::EndTimeMustBeFuture)));

    // Try to create pool with end_time equal to current time (should fail)
    let result = client.try_create_pool(&100, &token_address);
    assert_eq!(result, Err(Ok(PrediFiError::EndTimeMustBeFuture)));

    // Create pool with end_time in the future (should succeed)
    let result = client.try_create_pool(&200, &token_address);
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
    token_admin_client.mint(&user, &1000);

    let treasury = Address::generate(&env);
    client.init(&treasury, &0);

    let pool_id = client.create_pool(&100, &token_address);

    // Try to place prediction with zero amount (should fail)
    let result = client.try_place_prediction(&user, &pool_id, &0, &1);
    assert_eq!(result, Err(Ok(PrediFiError::InvalidPredictionAmount)));

    // Try to place prediction with negative amount (should fail)
    let result = client.try_place_prediction(&user, &pool_id, &-10, &1);
    assert_eq!(result, Err(Ok(PrediFiError::InvalidPredictionAmount)));

    // Place valid prediction
    client.place_prediction(&user, &pool_id, &100, &1);

    // Try to place another prediction on same pool (should fail)
    let result = client.try_place_prediction(&user, &pool_id, &50, &2);
    assert_eq!(result, Err(Ok(PrediFiError::PredictionAlreadyExists)));

    // Advance time past end_time
    env.ledger().set_timestamp(101);

    let user2 = Address::generate(&env);
    token_admin_client.mint(&user2, &1000);

    // Try to place prediction after pool ended (should fail)
    let result = client.try_place_prediction(&user2, &pool_id, &100, &1);
    assert_eq!(result, Err(Ok(PrediFiError::PredictionTooLate)));
}
