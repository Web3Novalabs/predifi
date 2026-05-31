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
            start_time: 0,
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
            start_time: 0,
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
            start_time: 0,
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

    let ac_id = env.register(crate::test::dummy_access_control::DummyAccessControl, ());
    let ac_client = crate::test::dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(crate::PredifiContract, ());
    let client = crate::PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token::Client::new(&env, &token_contract.address());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract.address());

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let creator = Address::generate(&env);

    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    // Init with 3% fallback fee
    client.init(
        &ac_id,
        &Address::generate(&env),
        &300u32,
        &0u64,
        &3600u64,
        &0u32,
    );
    client.add_token_to_whitelist(&admin, &token_contract.address());

    // Fee tiers (small numbers for easy math)
    // Tier 1: total_stake >= 1_000 → 1% (100 bps)
    // Tier 2: total_stake >= 5_000 → 0.5% (50 bps)
    let tiers = Vec::from_array(
        &env,
        [
            FeeTier {
                stake_threshold: 1_000,
                fee_bps: 100,
            },
            FeeTier {
                stake_threshold: 5_000,
                fee_bps: 50,
            },
        ],
    );
    client.set_fee_tiers(&admin, &tiers);

    let make_pool = |end_time: u64| -> u64 {
        client.create_pool(
            &creator,
            &end_time,
            &token_contract.address(),
            &2u32,
            &symbol_short!("Tech"),
            &PoolConfig {
                start_time: 0,
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
fn test_set_fee_tiers_unsorted_thresholds() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _, _, _, _, _, _) = crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let tiers = Vec::from_array(
        &env,
        [
            FeeTier {
                stake_threshold: 5_000_000,
                fee_bps: 50,
            },
            FeeTier {
                stake_threshold: 1_000_000,
                fee_bps: 100,
            }, // out of order
        ],
    );

    let result = client.try_set_fee_tiers(&admin, &tiers);
    assert_eq!(
        result.err().unwrap().unwrap(),
        crate::PredifiError::InvalidFeeBps
    );
}

#[test]
fn test_set_fee_tiers_duplicate_thresholds() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _, _, _, _, _, _) = crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let tiers = Vec::from_array(
        &env,
        [
            FeeTier {
                stake_threshold: 1_000_000,
                fee_bps: 100,
            },
            FeeTier {
                stake_threshold: 1_000_000,
                fee_bps: 50,
            }, // duplicate threshold
        ],
    );

    let result = client.try_set_fee_tiers(&admin, &tiers);
    assert_eq!(
        result.err().unwrap().unwrap(),
        crate::PredifiError::InvalidFeeBps
    );
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

#[test]
fn test_fee_tiers_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, token, token_admin_client, treasury, operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Setup: configure 3 fee tiers
    // Tier 1: 0+ tokens -> 1% fee (100 bps)
    // Tier 2: 1,000+ tokens -> 2% fee (200 bps)
    // Tier 3: 10,000+ tokens -> 3% fee (300 bps)
    let tiers = Vec::from_array(
        &env,
        [
            FeeTier {
                stake_threshold: 0,
                fee_bps: 100,
            },
            FeeTier {
                stake_threshold: 1_000,
                fee_bps: 200,
            },
            FeeTier {
                stake_threshold: 10_000,
                fee_bps: 300,
            },
        ],
    );
    client.set_fee_tiers(&admin, &tiers);

    let winner = Address::generate(&env);
    let loser = Address::generate(&env);

    // --- Pool 1: total_stake = 500 (expect 1% fee) ---
    let pool_id_1 = client.create_pool(
        &creator,
        &(env.ledger().timestamp() + 3600),
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Pool 1"),
            metadata_url: String::from_str(&env, "ipfs://1"),
            min_stake: 1,
            max_stake: 0,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "A"),
                String::from_str(&env, "B"),
            ],
        },
    );

    token_admin_client.mint(&winner, &400);
    token_admin_client.mint(&loser, &100);
    client.place_prediction(&winner, &pool_id_1, &400, &0, &None, &None);
    client.place_prediction(&loser, &pool_id_1, &100, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp += 3601);
    client.resolve_pool(&operator, &pool_id_1, &0);

    let winnings_1 = client.claim_winnings(&winner, &pool_id_1);
    // 500 * 0.01 = 5 fee. Payout = 495. Winner has 100% of winner stake.
    assert_eq!(winnings_1, 495);
    assert_eq!(token.balance(&winner), 495);
    assert_eq!(token.balance(&client.address), 5); // 5 tokens left in contract as fee

    // --- Pool 2: total_stake = 5,000 (expect 2% fee) ---
    let pool_id_2 = client.create_pool(
        &creator,
        &(env.ledger().timestamp() + 3600),
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Pool 2"),
            metadata_url: String::from_str(&env, "ipfs://2"),
            min_stake: 1,
            max_stake: 0,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "A"),
                String::from_str(&env, "B"),
            ],
        },
    );

    token_admin_client.mint(&winner, &4000);
    token_admin_client.mint(&loser, &1000);
    client.place_prediction(&winner, &pool_id_2, &4000, &0, &None, &None);
    client.place_prediction(&loser, &pool_id_2, &1000, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp += 3601);
    client.resolve_pool(&operator, &pool_id_2, &0);

    let winnings_2 = client.claim_winnings(&winner, &pool_id_2);
    // 5000 * 0.02 = 100 fee. Payout = 4900.
    assert_eq!(winnings_2, 4900);
    assert_eq!(token.balance(&winner), 495 + 4900); // 5395
    assert_eq!(token.balance(&client.address), 5 + 100); // 105 tokens total fee

    // --- Pool 3: total_stake = 50,000 (expect 3% fee) ---
    let pool_id_3 = client.create_pool(
        &creator,
        &(env.ledger().timestamp() + 3600),
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Pool 3"),
            metadata_url: String::from_str(&env, "ipfs://3"),
            min_stake: 1,
            max_stake: 0,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "A"),
                String::from_str(&env, "B"),
            ],
        },
    );

    token_admin_client.mint(&winner, &40000);
    token_admin_client.mint(&loser, &10000);
    client.place_prediction(&winner, &pool_id_3, &40000, &0, &None, &None);
    client.place_prediction(&loser, &pool_id_3, &10000, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp += 3601);
    client.resolve_pool(&operator, &pool_id_3, &0);

    let winnings_3 = client.claim_winnings(&winner, &pool_id_3);
    // 50000 * 0.03 = 1500 fee. Payout = 48500.
    assert_eq!(winnings_3, 48500);
    assert_eq!(token.balance(&winner), 5395 + 48500); // 53895
    assert_eq!(token.balance(&client.address), 105 + 1500); // 1605 total fee

    // --- Assert treasury balance after withdrawal ---
    // Total fees accumulated: 5 + 100 + 1500 = 1605
    client.withdraw_treasury(&admin, &token_address, &1605, &treasury);
    assert_eq!(token.balance(&treasury), 1605);
    assert_eq!(token.balance(&client.address), 0);
}
