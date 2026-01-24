#![cfg(test)]
#![allow(deprecated)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Env};

#[test]
fn test_claim_winnings() {
    let env = Env::default();
    env.mock_all_auths();

    // Register contract
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    // Setup Token
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone()); // Revert to v1
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract); // Client for minting
    let token_address = token_contract;

    // Setup Users
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // Mint tokens to users
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    // Init contract
    client.init();

    // Create Pool
    let pool_id = client.create_pool(&100, &token_address);

    // Place Predictions
    // User 1 bets 100 on Outcome 1
    client.place_prediction(&user1, &pool_id, &100, &1);

    // User 2 bets 100 on Outcome 2
    client.place_prediction(&user2, &pool_id, &100, &2);

    // Check balances (contract should have 200)
    assert_eq!(token.balance(&contract_id), 200);

    // Resolve Pool - Outcome 1 wins
    client.resolve_pool(&pool_id, &1);

    // User 1 Claims
    let winnings = client.claim_winnings(&user1, &pool_id);

    // Total pool is 200. Winning stake is 100. User 1 stake is 100.
    // Share = (100 / 100) * 200 = 200.
    assert_eq!(winnings, 200);
    assert_eq!(token.balance(&user1), 1100); // Initial 1000 - 100 bet + 200 winnings

    // User 2 Clams (Expect 0 or failure)
    let winnings2 = client.claim_winnings(&user2, &pool_id);
    assert_eq!(winnings2, 0);
    assert_eq!(token.balance(&user2), 900); // Initial 1000 - 100 bet
}

#[test]
#[should_panic(expected = "Already claimed")]
fn test_double_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin); // v1
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    client.init();
    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);
    client.resolve_pool(&pool_id, &1);

    client.claim_winnings(&user1, &pool_id);
    client.claim_winnings(&user1, &pool_id); // Should panic
}

#[test]
#[should_panic(expected = "Pool not resolved")]
fn test_claim_unresolved() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin); // v1
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    client.init();
    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);

    // Do NOT resolve
    client.claim_winnings(&user1, &pool_id);
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

    client.init();

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
