//! Full pool lifecycle integration tests with balance verification (Issue #1333).

#![cfg(test)]

use crate::test::ROLE_ADMIN;
use crate::PoolConfig;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

/// init → create_pool → place_prediction → close_staking → mark_pool_ready
/// → resolve_pool → claim_winnings
/// Verifies token balances at every stage (0% protocol fee).
#[test]
fn test_pool_full_lifecycle_with_balance_verification() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, token_address, token, token_admin_client, _treasury, operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let bettor_a = Address::generate(&env);
    let bettor_b = Address::generate(&env);

    token_admin_client.mint(&bettor_a, &500);
    token_admin_client.mint(&bettor_b, &300);

    // 1. Create pool (ends at timestamp 5000, min_pool_duration = 3600; 1000+3600 = 4600 < 5000).
    let end_time = 5_000u64;
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Lifecycle test pool"),
            metadata_url: String::from_str(&env, "ipfs://lifecycle"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "No"),
                String::from_str(&env, "Yes"),
            ],
        },
    );

    // 2. Place predictions.
    client.place_prediction(&bettor_a, &pool_id, &500, &0, &None, &None);
    client.place_prediction(&bettor_b, &pool_id, &300, &1, &None, &None);

    // Contract holds all staked tokens.
    assert_eq!(token.balance(&client.address), 800);
    assert_eq!(token.balance(&bettor_a), 0);
    assert_eq!(token.balance(&bettor_b), 0);

    // 3. Advance past end_time and close staking.
    env.ledger().with_mut(|li| li.timestamp = 5_001);
    client.close_staking(&pool_id);

    // 4. Signal pool is ready for resolution.
    client.mark_pool_ready(&pool_id);

    // 5. Resolve — outcome 0 wins.
    client.resolve_pool(&operator, &pool_id, &0u32);

    // 6. Bettor A (winner) claims: (500/500) * 800 = 800.
    let winnings_a = client.claim_winnings(&bettor_a, &pool_id);
    assert_eq!(winnings_a, 800);
    assert_eq!(token.balance(&bettor_a), 800);

    // 7. Bettor B (loser) claims 0.
    let winnings_b = client.claim_winnings(&bettor_b, &pool_id);
    assert_eq!(winnings_b, 0);
    assert_eq!(token.balance(&bettor_b), 0);

    // 8. Contract is empty after all claims (0% protocol fee).
    assert_eq!(token.balance(&client.address), 0);
}

/// Full lifecycle with a 2% protocol fee; verifies treasury accumulates the
/// expected fee and withdraw_treasury transfers it to a recipient.
#[test]
fn test_pool_lifecycle_with_fee_and_treasury_withdrawal() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let ac_id = env.register(
        crate::test::dummy_access_control::DummyAccessControl,
        (),
    );
    let ac_client =
        crate::test::dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);

    let contract_id = env.register(crate::PredifiContract, ());
    let client = crate::PredifiContractClient::new(&env, &contract_id);

    let token_admin_addr = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin_addr.clone());
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let treasury = Address::generate(&env);
    let operator = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);

    ac_client.grant_role(&operator, &crate::test::ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Init with 2% (200 bps) protocol fee.
    client.init(&ac_id, &treasury, &200u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_address);

    let bettor = Address::generate(&env);
    token_admin_client.mint(&bettor, &1_000);

    let end_time = 5_000u64;
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Fee lifecycle pool"),
            metadata_url: String::from_str(&env, "ipfs://fee-lifecycle"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "No"),
                String::from_str(&env, "Yes"),
            ],
        },
    );

    // Single bettor on outcome 0; fee = 2% of 1000 = 20; payout = 980.
    client.place_prediction(&bettor, &pool_id, &1_000, &0, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 5_001);
    client.close_staking(&pool_id);
    client.mark_pool_ready(&pool_id);
    client.resolve_pool(&operator, &pool_id, &0u32);

    let winnings = client.claim_winnings(&bettor, &pool_id);
    assert_eq!(winnings, 980);
    assert_eq!(token.balance(&bettor), 980);

    // Contract still holds the 20-token fee.
    assert_eq!(token.balance(&client.address), 20);

    // Treasury withdrawal transfers fee to a recipient.
    let recipient = Address::generate(&env);
    client.withdraw_treasury(&admin, &token_address, &20, &recipient);
    assert_eq!(token.balance(&recipient), 20);
    assert_eq!(token.balance(&client.address), 0);
}
