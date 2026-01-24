#![cfg(test)]
#![allow(deprecated)]

use super::*;
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
    // Resolve Pool - Outcome 1 wins
    // Move time forward to end_time
    env.ledger().set_timestamp(100);
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
    env.ledger().set_timestamp(100);
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
fn test_resolution_window() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;

    client.init();
    let end_time = 1000;
    let pool_id = client.create_pool(&end_time, &token_address);

    // 1. Too Early
    env.ledger().set_timestamp(end_time - 1);
    let result = client.try_resolve_pool(&pool_id, &1);
    assert_eq!(result, Err(Ok(Error::ResolutionTooEarly)));

    // 2. Too Late
    let resolution_window = 7 * 24 * 60 * 60;
    env.ledger().set_timestamp(end_time + resolution_window + 1);
    let result = client.try_resolve_pool(&pool_id, &1);
    assert_eq!(result, Err(Ok(Error::ResolutionTooLate)));

    // 3. Just Right (Start of window)
    env.ledger().set_timestamp(end_time);
    let result = client.try_resolve_pool(&pool_id, &1);
    assert!(result.is_ok());

    // Create another pool for end of window test
    let pool_id_2 = client.create_pool(&end_time, &token_address);
    // 4. Just Right (End of window)
    env.ledger().set_timestamp(end_time + resolution_window);
    let result = client.try_resolve_pool(&pool_id_2, &1);
    assert!(result.is_ok());
}
