//! Fee tier transition integration tests (Issue #1335).
//!
//! Covers: fee applied below/at/above tier thresholds, multiple tiers,
//! and treasury accumulation across several pools.

#![cfg(test)]

use crate::test::ROLE_ADMIN;
use crate::{FeeTier, PoolConfig};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String, Vec,
};

fn create_pool(
    env: &Env,
    client: &crate::PredifiContractClient,
    creator: &Address,
    token_address: &Address,
    end_time: u64,
) -> u64 {
    client.create_pool(
        creator,
        &end_time,
        token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(env, "Fee tier test pool"),
            metadata_url: String::from_str(env, "ipfs://fee-tier"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                env,
                String::from_str(env, "No"),
                String::from_str(env, "Yes"),
            ],
        },
    )
}

/// Pool with total stake below the tier threshold uses the base fee.
/// Pool above the threshold uses the reduced tier fee.
#[test]
fn test_fee_tier_boundary_below_and_above_threshold() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, token_address, token, token_admin_client, _treasury, operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Base fee 3% (300 bps). Tier: stake >= 10_000 → 1% (100 bps).
    client.set_fee_bps(&admin, &300u32);
    env.ledger()
        .with_mut(|li| li.timestamp += crate::FEE_CHANGE_TIMELOCK_SECONDS + 1);
    client.apply_fee_bps(&admin);

    let tiers = Vec::from_array(
        &env,
        [FeeTier {
            stake_threshold: 10_000i128,
            fee_bps: 100,
        }],
    );
    client.set_fee_tiers(&admin, &tiers);

    // ── Pool A: 500 stake — below threshold, base fee applies (3%).
    let bettor_a = Address::generate(&env);
    token_admin_client.mint(&bettor_a, &500);

    let now = env.ledger().timestamp();
    let pool_a = create_pool(&env, &client, &creator, &token_address, now + 4_000);
    client.place_prediction(&bettor_a, &pool_a, &500, &0, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = now + 4_001);
    client.resolve_pool(&operator, &pool_a, &0u32);

    // fee = 3% of 500 = 15; payout = 485.
    let winnings_a = client.claim_winnings(&bettor_a, &pool_a);
    assert_eq!(winnings_a, 485);
    assert_eq!(token.balance(&bettor_a), 485);

    // ── Pool B: 15_000 stake — above threshold, tier fee applies (1%).
    let bettor_b = Address::generate(&env);
    token_admin_client.mint(&bettor_b, &15_000);

    let now2 = env.ledger().timestamp();
    let pool_b = create_pool(&env, &client, &creator, &token_address, now2 + 4_000);
    client.place_prediction(&bettor_b, &pool_b, &15_000, &0, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = now2 + 4_001);
    client.resolve_pool(&operator, &pool_b, &0u32);

    // fee = 1% of 15000 = 150; payout = 14850.
    let winnings_b = client.claim_winnings(&bettor_b, &pool_b);
    assert_eq!(winnings_b, 14_850);
    assert_eq!(token.balance(&bettor_b), 14_850);
}

/// Treasury accumulates fees from multiple pools.
#[test]
fn test_treasury_accumulation_across_pools() {
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
    let tok = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let treasury = Address::generate(&env);
    let operator = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);

    ac_client.grant_role(&operator, &crate::test::ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // 2% base fee, no tiers.
    client.init(&ac_id, &treasury, &200u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_address);

    let bettor1 = Address::generate(&env);
    let bettor2 = Address::generate(&env);
    token_admin_client.mint(&bettor1, &1_000);
    token_admin_client.mint(&bettor2, &2_000);

    let now = env.ledger().timestamp();

    // Pool 1: 1000 stake → fee = 20.
    let pool1 = create_pool(&env, &client, &creator, &token_address, now + 4_000);
    client.place_prediction(&bettor1, &pool1, &1_000, &0, &None, &None);
    env.ledger().with_mut(|li| li.timestamp = now + 4_001);
    client.resolve_pool(&operator, &pool1, &0u32);
    client.claim_winnings(&bettor1, &pool1);

    // Pool 2: 2000 stake → fee = 40.
    let now2 = env.ledger().timestamp();
    let pool2 = create_pool(&env, &client, &creator, &token_address, now2 + 4_000);
    client.place_prediction(&bettor2, &pool2, &2_000, &0, &None, &None);
    env.ledger().with_mut(|li| li.timestamp = now2 + 4_001);
    client.resolve_pool(&operator, &pool2, &0u32);
    client.claim_winnings(&bettor2, &pool2);

    // Contract holds exactly 20 + 40 = 60 tokens of accumulated fees.
    assert_eq!(tok.balance(&client.address), 60);

    // Admin withdraws all accumulated fees.
    let recipient = Address::generate(&env);
    client.withdraw_treasury(&admin, &token_address, &60, &recipient);
    assert_eq!(tok.balance(&recipient), 60);
    assert_eq!(tok.balance(&client.address), 0);
}

/// Two-tier config: stake in the middle tier pays the correct rate.
#[test]
fn test_two_tier_fee_rates() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, token_address, token, token_admin_client, _treasury, operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Base 5% (500 bps).
    // Tier 1: stake >= 1_000  → 2% (200 bps).
    // Tier 2: stake >= 10_000 → 1% (100 bps).
    client.set_fee_bps(&admin, &500u32);
    env.ledger()
        .with_mut(|li| li.timestamp += crate::FEE_CHANGE_TIMELOCK_SECONDS + 1);
    client.apply_fee_bps(&admin);

    let tiers = Vec::from_array(
        &env,
        [
            FeeTier { stake_threshold: 1_000i128, fee_bps: 200 },
            FeeTier { stake_threshold: 10_000i128, fee_bps: 100 },
        ],
    );
    client.set_fee_tiers(&admin, &tiers);

    // Mid-tier stake (5_000): tier 1 applies → 2% fee.
    let bettor = Address::generate(&env);
    token_admin_client.mint(&bettor, &5_000);

    let now = env.ledger().timestamp();
    let pool = create_pool(&env, &client, &creator, &token_address, now + 4_000);
    client.place_prediction(&bettor, &pool, &5_000, &0, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = now + 4_001);
    client.resolve_pool(&operator, &pool, &0u32);

    // fee = 2% of 5000 = 100; payout = 4900.
    let winnings = client.claim_winnings(&bettor, &pool);
    assert_eq!(winnings, 4_900);
    assert_eq!(token.balance(&bettor), 4_900);
}
