#![cfg(test)]

use crate::test::ROLE_ADMIN;
use crate::{FeeTier, PoolConfig};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String, Vec,
};

#[test]
fn test_dynamic_fee_tiers_application() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        ac_client,
        client,
        token_address,
        _token,
        token_admin_client,
        _treasury,
        operator,
        creator,
    ) = crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Set global fee to 3%
    client.set_fee_bps(&admin, &300u32);

    // Set up fee tiers
    // Threshold 1M (1,000,000 * 10^7) -> 1% (100 bps)
    // Threshold 5M (5,000,000 * 10^7) -> 0.5% (50 bps)
    let tiers = Vec::from_array(
        &env,
        [
            FeeTier {
                stake_threshold: 1_000_000 * 10_000_000,
                fee_bps: 100,
            },
            FeeTier {
                stake_threshold: 5_000_000 * 10_000_000,
                fee_bps: 50,
            },
        ],
    );

    client.set_fee_tiers(&admin, &tiers);

    // 1. Create a pool with low volume (below 1M)
    let end_time_1 = env.ledger().timestamp() + 10000;
    let pool_id = client.create_pool(
        &creator,
        &end_time_1,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Low Volume Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "A"),
                String::from_str(&env, "B"),
            ],
        },
    );

    // Resolve it
    env.ledger().set_timestamp(end_time_1 + 1000);
    client.resolve_pool(&operator, &pool_id, &0);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.fee_bps, 300); // Should use global default (300)

    // 2. Create a pool with medium volume (2M)
    let end_time_2 = env.ledger().timestamp() + 10000;
    let pool_id_2 = client.create_pool(
        &creator,
        &end_time_2,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Med Volume Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "A"),
                String::from_str(&env, "B"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &(2_000_000 * 10_000_000));
    client.place_prediction(
        &user,
        &pool_id_2,
        &(2_000_000 * 10_000_000),
        &0,
        &None,
        &None,
    );

    env.ledger().set_timestamp(end_time_2 + 1000);
    client.resolve_pool(&operator, &pool_id_2, &0);

    let pool2 = client.get_pool(&pool_id_2);
    assert_eq!(pool2.fee_bps, 100); // Should apply 1% tier

    // 3. Create a pool with high volume (6M)
    let end_time_3 = env.ledger().timestamp() + 10000;
    let pool_id_3 = client.create_pool(
        &creator,
        &end_time_3,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "High Volume Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "A"),
                String::from_str(&env, "B"),
            ],
        },
    );
    token_admin_client.mint(&user, &(6_000_000 * 10_000_000));
    client.place_prediction(
        &user,
        &pool_id_3,
        &(6_000_000 * 10_000_000),
        &0,
        &None,
        &None,
    );

    env.ledger().set_timestamp(end_time_3 + 1000);
    client.resolve_pool(&operator, &pool_id_3, &0);

    let pool3 = client.get_pool(&pool_id_3);
    assert_eq!(pool3.fee_bps, 50); // Should apply 0.5% tier
}

/// Verifies that the contract retains the correct fee amount (treasury intake)
/// for each fee tier after winners claim their winnings.
///
/// Tier setup:
///   - Default (fallback): 300 bps (3%)
///   - Tier 1: stake >= 1_000 → 100 bps (1%)
///   - Tier 2: stake >= 5_000 → 50 bps (0.5%)
///
/// For each pool a single user stakes the full amount on the winning outcome,
/// so fee = total_stake * fee_bps / 10_000 (integer floor).
#[test]
fn test_fee_tier_treasury_intake() {
    let env = Env::default();
    env.mock_all_auths();

    // ── Manual setup so we can control fee_bps in init ───────────────────────
    use crate::test::ROLE_OPERATOR;

    let ac_id = env.register(
        crate::test::dummy_access_control::DummyAccessControl,
        (),
    );
    let ac_client =
        crate::test::dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(crate::PredifiContract, ());
    let client = crate::PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let creator = Address::generate(&env);

    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    // Init with 3% fallback fee
    client.init(&ac_id, &Address::generate(&env), &300u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_contract);

    // Fee tiers (small numbers for easy math)
    // Tier 1: total_stake >= 1_000 → 1% (100 bps)
    // Tier 2: total_stake >= 5_000 → 0.5% (50 bps)
    let tiers = Vec::from_array(
        &env,
        [
            FeeTier { stake_threshold: 1_000, fee_bps: 100 },
            FeeTier { stake_threshold: 5_000, fee_bps: 50 },
        ],
    );
    client.set_fee_tiers(&admin, &tiers);

    let make_pool = |end_time: u64| -> u64 {
        client.create_pool(
            &creator,
            &end_time,
            &token_contract,
            &2u32,
            &symbol_short!("Tech"),
            &PoolConfig {
                description: String::from_str(&env, "pool"),
                metadata_url: String::from_str(&env, "ipfs://x"),
                min_stake: 1i128,
                max_stake: 0i128,
                max_total_stake: 0,
                min_total_stake: 1,
                initial_liquidity: 0i128,
                required_resolutions: 1u32,
                private: false,
                whitelist_key: None,
                outcome_descriptions: vec![
                    &env,
                    String::from_str(&env, "A"),
                    String::from_str(&env, "B"),
                ],
            },
        )
    };

    // ── Pool 1: stake = 500 → below tier 1, uses fallback 3% ─────────────────
    // fee = 500 * 300 / 10_000 = 15
    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &500);
    let t1 = env.ledger().timestamp() + 10_000;
    let pid1 = make_pool(t1);
    client.place_prediction(&user1, &pid1, &500, &0, &None, &None);
    env.ledger().set_timestamp(t1 + 3_601);
    client.resolve_pool(&operator, &pid1, &0);
    client.claim_winnings(&user1, &pid1);
    let fee1 = token.balance(&contract_id);
    assert_eq!(fee1, 15, "tier fallback (3%): expected fee 15, got {fee1}");

    // ── Pool 2: stake = 2_000 → tier 1 (1%) ──────────────────────────────────
    // fee = 2_000 * 100 / 10_000 = 20
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user2, &2_000);
    let t2 = env.ledger().timestamp() + 10_000;
    let pid2 = make_pool(t2);
    client.place_prediction(&user2, &pid2, &2_000, &0, &None, &None);
    env.ledger().set_timestamp(t2 + 3_601);
    client.resolve_pool(&operator, &pid2, &0);
    client.claim_winnings(&user2, &pid2);
    // contract now holds fee1 + fee2
    let fee2 = token.balance(&contract_id) - fee1;
    assert_eq!(fee2, 20, "tier 1 (1%): expected fee 20, got {fee2}");

    // ── Pool 3: stake = 6_000 → tier 2 (0.5%) ────────────────────────────────
    // fee = 6_000 * 50 / 10_000 = 30
    let user3 = Address::generate(&env);
    token_admin_client.mint(&user3, &6_000);
    let t3 = env.ledger().timestamp() + 10_000;
    let pid3 = make_pool(t3);
    client.place_prediction(&user3, &pid3, &6_000, &0, &None, &None);
    env.ledger().set_timestamp(t3 + 3_601);
    client.resolve_pool(&operator, &pid3, &0);
    client.claim_winnings(&user3, &pid3);
    let fee3 = token.balance(&contract_id) - fee1 - fee2;
    assert_eq!(fee3, 30, "tier 2 (0.5%): expected fee 30, got {fee3}");
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_set_fee_tiers_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, _) = crate::test::setup(&env);
    let user = Address::generate(&env);
    let tiers = Vec::new(&env);

    client.set_fee_tiers(&user, &tiers);
}
