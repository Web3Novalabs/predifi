#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    token, Address, Env,
};

// Helper function to create a test token
fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
    (
        token::Client::new(env, &contract_address.address()),
        token::StellarAssetClient::new(env, &contract_address.address()),
    )
}

// Helper function to setup the contract
fn setup_contract<'a>(
    env: &Env,
) -> (
    PredictionMarketClient<'a>,
    token::Client<'a>,
    token::StellarAssetClient<'a>,
    Address,
    Address,
) {
    let admin = Address::generate(env);
    let treasury = Address::generate(env);
    let token_admin = Address::generate(env);

    let contract_id = env.register(PredictionMarket, ());
    let client = PredictionMarketClient::new(env, &contract_id);

    let (token, token_admin_client) = create_token_contract(env, &token_admin);

    (client, token, token_admin_client, admin, treasury)
}

#[test]
fn test_initialize() {
    let env = Env::default();
    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);

    env.mock_all_auths();

    // Initialize with 2% fee (200 basis points)
    client.initialize(&admin, &treasury, &200);

    // Verify initialization
    assert_eq!(client.get_protocol_fee(), 200);
    assert_eq!(client.get_treasury(), treasury);
    assert_eq!(client.get_total_fees_collected(), 0);
}

#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_initialize_twice_fails() {
    let env = Env::default();
    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);

    env.mock_all_auths();

    client.initialize(&admin, &treasury, &200);
    // Try to initialize again - should panic
    client.initialize(&admin, &treasury, &300);
}

#[test]
#[should_panic(expected = "Fee exceeds maximum allowed")]
fn test_initialize_with_excessive_fee() {
    let env = Env::default();
    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);

    env.mock_all_auths();

    // Try to initialize with 15% fee (1500 bps), max is 10% (1000 bps)
    client.initialize(&admin, &treasury, &1500);
}

#[test]
fn test_update_protocol_fee() {
    let env = Env::default();
    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);

    env.mock_all_auths();

    client.initialize(&admin, &treasury, &200);

    // Update fee to 3%
    client.update_protocol_fee(&300);

    assert_eq!(client.get_protocol_fee(), 300);
}

#[test]
#[should_panic(expected = "Fee exceeds maximum allowed")]
fn test_update_protocol_fee_exceeds_max() {
    let env = Env::default();
    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);

    env.mock_all_auths();

    client.initialize(&admin, &treasury, &200);

    // Try to update to 15% fee (exceeds 10% max)
    client.update_protocol_fee(&1500);
}

#[test]
fn test_update_treasury() {
    let env = Env::default();
    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);

    env.mock_all_auths();

    client.initialize(&admin, &treasury, &200);

    // Create new treasury
    let new_treasury = Address::generate(&env);

    // Update treasury
    client.update_treasury(&new_treasury);

    assert_eq!(client.get_treasury(), new_treasury);
}

#[test]
fn test_calculate_fee() {
    let env = Env::default();
    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);

    env.mock_all_auths();

    // Initialize with 2% fee (200 basis points)
    client.initialize(&admin, &treasury, &200);

    // Test fee calculation
    let amount = 10000i128;
    let fee = client.calculate_fee(&amount);

    // 2% of 10000 = 200
    assert_eq!(fee, 200);

    // Test with different amounts
    assert_eq!(client.calculate_fee(&100000), 2000);
    assert_eq!(client.calculate_fee(&1000), 20);
    assert_eq!(client.calculate_fee(&50), 1);
}

#[test]
fn test_calculate_amount_after_fee() {
    let env = Env::default();
    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);

    env.mock_all_auths();

    // Initialize with 2% fee
    client.initialize(&admin, &treasury, &200);

    let amount = 10000i128;
    let (net_amount, fee) = client.calculate_amount_after_fee(&amount);

    assert_eq!(fee, 200); // 2% of 10000
    assert_eq!(net_amount, 9800); // 10000 - 200
    assert_eq!(net_amount + fee, amount); // Verify no loss
}

#[test]
fn test_create_pool_with_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, token, token_admin, admin, treasury) = setup_contract(&env);
    let creator = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &treasury, &200); // 2% fee

    // Mint tokens to creator
    token_admin.mint(&creator, &10000);

    // Create pool with 10000 tokens
    let pool_id = 1u64;
    let net_amount = client.create_pool(&pool_id, &creator, &token.address, &10000);

    // Net amount should be 9800 (10000 - 200 fee)
    assert_eq!(net_amount, 9800);

    // Verify treasury received the fee
    assert_eq!(token.balance(&treasury), 200);

    // Verify total fees collected
    assert_eq!(client.get_total_fees_collected(), 200);
}

#[test]
fn test_resolve_pool_with_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, token, token_admin, admin, treasury) = setup_contract(&env);
    let contract_address = client.address.clone();

    // Initialize contract
    client.initialize(&admin, &treasury, &200); // 2% fee

    // Mint tokens to contract
    token_admin.mint(&contract_address, &10000);

    // Resolve pool with 10000 tokens
    let pool_id = 1u64;
    let (net_amount, fee) = client.resolve_pool(&pool_id, &token.address, &10000);

    // Net amount should be 9800, fee 200
    assert_eq!(net_amount, 9800);
    assert_eq!(fee, 200);

    // Verify treasury received the fee
    assert_eq!(token.balance(&treasury), 200);

    // Verify total fees collected
    assert_eq!(client.get_total_fees_collected(), 200);
}

#[test]
fn test_distribute_winnings_with_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, token, token_admin, admin, treasury) = setup_contract(&env);
    let contract_address = client.address.clone();
    let winner = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &treasury, &200); // 2% fee

    // Mint tokens to contract
    token_admin.mint(&contract_address, &10000);

    // Distribute winnings
    let pool_id = 1u64;
    let net_winnings = client.distribute_winnings(&pool_id, &token.address, &winner, &10000);

    // Net winnings should be 9800
    assert_eq!(net_winnings, 9800);

    // Verify winner received net winnings
    assert_eq!(token.balance(&winner), 9800);

    // Verify treasury received the fee
    assert_eq!(token.balance(&treasury), 200);

    // Verify total fees collected
    assert_eq!(client.get_total_fees_collected(), 200);
}

#[test]
fn test_multiple_fee_collections() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, token, token_admin, admin, treasury) = setup_contract(&env);
    let creator1 = Address::generate(&env);
    let creator2 = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin, &treasury, &200); // 2% fee

    // Mint tokens to creators
    token_admin.mint(&creator1, &10000);
    token_admin.mint(&creator2, &20000);

    // Create two pools
    let net1 = client.create_pool(&1, &creator1, &token.address, &10000);
    let net2 = client.create_pool(&2, &creator2, &token.address, &20000);

    assert_eq!(net1, 9800); // 10000 - 200
    assert_eq!(net2, 19600); // 20000 - 400

    // Verify total fees collected: 200 + 400 = 600
    assert_eq!(client.get_total_fees_collected(), 600);

    // Verify treasury balance
    assert_eq!(token.balance(&treasury), 600);
}

#[test]
fn test_different_fee_percentages() {
    let env = Env::default();
    env.mock_all_auths();

    // Test with 5% fee (500 bps)
    {
        let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);
        client.initialize(&admin, &treasury, &500);

        let amount = 10000i128;
        let fee = client.calculate_fee(&amount);
        assert_eq!(fee, 500); // 5% of 10000
    }

    // Test with 0.5% fee (50 bps)
    {
        let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);
        client.initialize(&admin, &treasury, &50);

        let amount = 10000i128;
        let fee = client.calculate_fee(&amount);
        assert_eq!(fee, 50); // 0.5% of 10000
    }

    // Test with 10% fee (1000 bps) - maximum
    {
        let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);
        client.initialize(&admin, &treasury, &1000);

        let amount = 10000i128;
        let fee = client.calculate_fee(&amount);
        assert_eq!(fee, 1000); // 10% of 10000
    }
}

#[test]
fn test_zero_amount_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);
    client.initialize(&admin, &treasury, &200);

    let fee = client.calculate_fee(&0);
    assert_eq!(fee, 0);
}

#[test]
fn test_get_max_fee() {
    let env = Env::default();
    let (client, _token, _token_admin, _admin, _treasury) = setup_contract(&env);
    let max_fee = client.get_max_fee();
    assert_eq!(max_fee, 1000); // 10% in basis points
}

#[test]
fn test_fee_precision() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _token, _token_admin, admin, treasury) = setup_contract(&env);
    client.initialize(&admin, &treasury, &250); // 2.5% fee

    // Test small amounts
    assert_eq!(client.calculate_fee(&1000), 25); // 2.5% of 1000
    assert_eq!(client.calculate_fee(&100), 2); // 2.5% of 100
    assert_eq!(client.calculate_fee(&10), 0); // 2.5% of 10 = 0.25, rounds down

    // Test large amounts
    assert_eq!(client.calculate_fee(&1000000), 25000); // 2.5% of 1,000,000
}
