#![cfg(test)]
#![allow(deprecated)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Env, Symbol};

// Dummy access control contract for testing
mod dummy_access_control {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct DummyAccessControl;

    #[contractimpl]
    impl DummyAccessControl {
        pub fn grant_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().set(&key, &true);
        }

        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }
    }
}

// Role constants
const ROLE_ADMIN: u32 = 0;
const ROLE_OPERATOR: u32 = 1;

#[test]
fn test_claim_winnings() {
    let env = Env::default();
    env.mock_all_auths();

    // Register dummy access control contract
    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);

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
    let operator = Address::generate(&env);

    // Mint tokens to users
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    // Grant operator role
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    // Init contract
    client.init(&ac_id, &user1, &100u32);

    // Create Pool
    let pool_id = client.create_pool(&100, &token_address);

    // Place Predictions
    client.place_prediction(&user1, &pool_id, &100, &1);
    client.place_prediction(&user2, &pool_id, &100, &2);

    // Check balances (contract should have 200)
    assert_eq!(token.balance(&contract_id), 200);

    // Resolve Pool - Outcome 1 wins
    client.resolve_pool(&operator, &pool_id, &1u32);

    // User 1 Claims
    let winnings = client.claim_winnings(&user1, &pool_id);
    // Total pool is 200. Winning stake is 100. User 1 stake is 100.
    // Share = (100 / 100) * 200 = 200.
    assert_eq!(winnings, 200);
    assert_eq!(token.balance(&user1), 1100); // Initial 1000 - 100 bet + 200 winnings

    // User 2 Claims (Expect 0)
    let winnings2 = client.claim_winnings(&user2, &pool_id);
    assert_eq!(winnings2, 0);
    assert_eq!(token.balance(&user2), 900); // Initial 1000 - 100 bet
}

#[test]
#[should_panic(expected = "Unauthorized: missing required role")]
fn test_unauthorized_set_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let not_admin = Address::generate(&env); // No role granted
    let treasury = Address::generate(&env);

    client.init(&ac_id, &treasury, &100u32);
    client.set_fee_bps(&not_admin, &999u32); // Should panic
}

#[test]
#[should_panic(expected = "Unauthorized: missing required role")]
fn test_unauthorized_set_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let not_admin = Address::generate(&env); // No role granted
    let treasury = Address::generate(&env);

    client.init(&ac_id, &treasury, &100u32);

    let new_treasury = Address::generate(&env);
    client.set_treasury(&not_admin, &new_treasury); // Should panic
}

#[test]
#[should_panic(expected = "Unauthorized: missing required role")]
fn test_unauthorized_resolve_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let treasury = Address::generate(&env);
    client.init(&ac_id, &treasury, &100u32);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;

    let pool_id = client.create_pool(&100, &token_address);

    let not_operator = Address::generate(&env); // No role granted

    client.resolve_pool(&not_operator, &pool_id, &1u32); // Should panic
}

#[test]
#[should_panic(expected = "Already claimed")]
fn test_double_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user1 = Address::generate(&env);
    let operator = Address::generate(&env);

    token_admin_client.mint(&user1, &1000);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    client.init(&ac_id, &user1, &100u32);

    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);
    client.resolve_pool(&operator, &pool_id, &1u32);

    client.claim_winnings(&user1, &pool_id);
    client.claim_winnings(&user1, &pool_id); // Should panic
}

#[test]
#[should_panic(expected = "Pool not resolved")]
fn test_claim_unresolved() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    client.init(&ac_id, &user1, &100u32);

    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);

    // Do NOT resolve
    client.claim_winnings(&user1, &pool_id); // Should panic
}

#[test]
fn test_get_user_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_address = token_contract;
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    client.init(&ac_id, &user, &100u32);

    // Create 3 pools and place predictions
    let pool0 = client.create_pool(&100, &token_address);
    let pool1 = client.create_pool(&200, &token_address);
    let pool2 = client.create_pool(&300, &token_address);

    client.place_prediction(&user, &pool0, &10, &1);
    client.place_prediction(&user, &pool1, &20, &2);
    client.place_prediction(&user, &pool2, &30, &1);

    // Test pagination: Offset 0, Limit 2
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
