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

/// Comprehensive test verifying exact boundary transitions:
/// threshold - 1 (lower tier / base fee), threshold (exact tier), and threshold + 1 (tier fee)
/// across multiple tiers (Tier 1: 1_000 -> 3%, Tier 2: 5_000 -> 2%, Tier 3: 10_000 -> 1%, Base: 5%).
#[test]
fn test_fee_tier_exact_boundary_transitions() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, token_address, token, token_admin_client, _treasury, operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Base fee 5% (500 bps).
    client.set_fee_bps(&admin, &500u32);
    env.ledger()
        .with_mut(|li| li.timestamp += crate::FEE_CHANGE_TIMELOCK_SECONDS + 1);
    client.apply_fee_bps(&admin);

    let tiers = Vec::from_array(
        &env,
        [
            FeeTier { stake_threshold: 1_000i128, fee_bps: 300 },   // Tier 1: 3%
            FeeTier { stake_threshold: 5_000i128, fee_bps: 200 },   // Tier 2: 2%
            FeeTier { stake_threshold: 10_000i128, fee_bps: 100 },  // Tier 3: 1%
        ],
    );
    client.set_fee_tiers(&admin, &tiers);

    let test_cases = [
        // (stake, expected_fee_bps, expected_fee, expected_payout)
        (999i128, 500u32, 49i128, 950i128),      // Tier 1 threshold - 1 -> Base fee (5%)
        (1_000i128, 300u32, 30i128, 970i128),    // Tier 1 exact threshold -> Tier 1 (3%)
        (1_001i128, 300u32, 30i128, 971i128),    // Tier 1 threshold + 1 -> Tier 1 (3%)
        (4_999i128, 300u32, 149i128, 4_850i128), // Tier 2 threshold - 1 -> Tier 1 (3%)
        (5_000i128, 200u32, 100i128, 4_900i128), // Tier 2 exact threshold -> Tier 2 (2%)
        (5_001i128, 200u32, 100i128, 4_901i128), // Tier 2 threshold + 1 -> Tier 2 (2%)
        (9_999i128, 200u32, 199i128, 9_800i128), // Tier 3 threshold - 1 -> Tier 2 (2%)
        (10_000i128, 100u32, 100i128, 9_900i128),// Tier 3 exact threshold -> Tier 3 (1%)
        (10_001i128, 100u32, 100i128, 9_901i128),// Tier 3 threshold + 1 -> Tier 3 (1%)
    ];

    for (stake, expected_fee_bps, expected_fee, expected_payout) in test_cases {
        let bettor = Address::generate(&env);
        token_admin_client.mint(&bettor, &stake);

        let now = env.ledger().timestamp();
        let pool = create_pool(&env, &client, &creator, &token_address, now + 4_000);
        client.place_prediction(&bettor, &pool, &stake, &0, &None, &None);

        env.ledger().with_mut(|li| li.timestamp = now + 4_001);
        client.resolve_pool(&operator, &pool, &0u32);

        let pool_data = client.get_pool(&pool);
        assert_eq!(
            pool_data.fee_bps, expected_fee_bps,
            "Stake {stake}: expected fee_bps {expected_fee_bps}, got {}", pool_data.fee_bps
        );

        let initial_contract_balance = token.balance(&client.address);
        let winnings = client.claim_winnings(&bettor, &pool);
        assert_eq!(
            winnings, expected_payout,
            "Stake {stake}: expected payout {expected_payout}, got {winnings}"
        );
        assert_eq!(token.balance(&bettor), expected_payout);

        let fee_retained = token.balance(&client.address) - (initial_contract_balance - stake);
        assert_eq!(
            fee_retained, expected_fee,
            "Stake {stake}: expected fee retained {expected_fee}, got {fee_retained}"
        );
    }
}

/// Tests mid-pool fee rate progression as total stake increases across tier thresholds.
/// Multiple bets are placed over time, crossing from below Tier 1 to Tier 2 and finally Tier 3.
/// Verifies that resolution dynamically applies the final overall tier fee (Tier 3) to the pool.
#[test]
fn test_fee_change_mid_pool_stake_crossings() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, token_address, token, token_admin_client, _treasury, operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Base fee 5% (500 bps).
    client.set_fee_bps(&admin, &500u32);
    env.ledger()
        .with_mut(|li| li.timestamp += crate::FEE_CHANGE_TIMELOCK_SECONDS + 1);
    client.apply_fee_bps(&admin);

    // Tiers: Tier 1: 1_000 -> 300 bps (3%), Tier 2: 5_000 -> 200 bps (2%), Tier 3: 10_000 -> 100 bps (1%)
    let tiers = Vec::from_array(
        &env,
        [
            FeeTier { stake_threshold: 1_000i128, fee_bps: 300 },
            FeeTier { stake_threshold: 5_000i128, fee_bps: 200 },
            FeeTier { stake_threshold: 10_000i128, fee_bps: 100 },
        ],
    );
    client.set_fee_tiers(&admin, &tiers);

    let now = env.ledger().timestamp();
    let pool_id = create_pool(&env, &client, &creator, &token_address, now + 4_000);

    // Prediction 1: 800 tokens on outcome 0 -> total stake = 800 (below Tier 1 threshold 1,000)
    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &800);
    client.place_prediction(&user1, &pool_id, &800, &0, &None, &None);
    let pool_state1 = client.get_pool(&pool_id);
    assert_eq!(pool_state1.total_stake, 800);

    // Prediction 2: 4,500 tokens on outcome 0 -> total stake = 5,300 (crosses Tier 1 and enters Tier 2 threshold 5,000)
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user2, &4_500);
    client.place_prediction(&user2, &pool_id, &4_500, &0, &None, &None);
    let pool_state2 = client.get_pool(&pool_id);
    assert_eq!(pool_state2.total_stake, 5_300);

    // Prediction 3: 6,000 tokens on outcome 1 (losing outcome) -> total stake = 11,300 (crosses into Tier 3 threshold 10,000)
    let user3 = Address::generate(&env);
    token_admin_client.mint(&user3, &6_000);
    client.place_prediction(&user3, &pool_id, &6_000, &1, &None, &None);
    let pool_state3 = client.get_pool(&pool_id);
    assert_eq!(pool_state3.total_stake, 11_300);

    // Resolve pool for outcome 0
    env.ledger().with_mut(|li| li.timestamp = now + 4_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    let pool_resolved = client.get_pool(&pool_id);
    // Dynamic fee must evaluate total_stake (11,300) -> Tier 3 fee applies (100 bps / 1%)
    assert_eq!(pool_resolved.fee_bps, 100);

    // Winner 1 claim: gross = 800 * 11300 / 5300 = 1705, fee = 1705 * 100 / 10000 = 17, net = 1688
    let winnings1 = client.claim_winnings(&user1, &pool_id);
    assert_eq!(winnings1, 1688);
    assert_eq!(token.balance(&user1), 1688);

    // Winner 2 claim: gross payout pool = 11,187. user2 share = (4500 * 11187) / 5300 = 9498
    let winnings2 = client.claim_winnings(&user2, &pool_id);
    assert_eq!(winnings2, 9498);
    assert_eq!(token.balance(&user2), 9498);

    // Remaining contract balance holds accumulated protocol fee + dust
    let contract_balance = token.balance(&client.address);
    // Total deposited: 11,300. Total payouts: 1688 + 9498 = 11186. Retained: 114 (113 fee + 1 dust).
    assert_eq!(contract_balance, 114);
}

/// Tests admin updating fee tier rules while a pool is open/active mid-pool.
/// Verifies resolution evaluates against the newly configured fee tiers.
#[test]
fn test_fee_tier_reconfiguration_mid_pool() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, token_address, token, token_admin_client, _treasury, operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Initial Tiers: Tier 1: 10_000 -> 300 bps (3%)
    let initial_tiers = Vec::from_array(
        &env,
        [FeeTier {
            stake_threshold: 10_000i128,
            fee_bps: 300,
        }],
    );
    client.set_fee_tiers(&admin, &initial_tiers);

    let now = env.ledger().timestamp();
    let pool_id = create_pool(&env, &client, &creator, &token_address, now + 4_000);

    let bettor = Address::generate(&env);
    token_admin_client.mint(&bettor, &12_000);
    client.place_prediction(&bettor, &pool_id, &12_000, &0, &None, &None);

    // Mid-pool (before resolution), Admin updates fee tiers to lower rate for 10k threshold (50 bps / 0.5%)
    let updated_tiers = Vec::from_array(
        &env,
        [
            FeeTier { stake_threshold: 5_000i128, fee_bps: 150 },
            FeeTier { stake_threshold: 10_000i128, fee_bps: 50 },
        ],
    );
    client.set_fee_tiers(&admin, &updated_tiers);

    // Resolve pool
    env.ledger().with_mut(|li| li.timestamp = now + 4_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    let pool_resolved = client.get_pool(&pool_id);
    // Must use newly configured fee rate (50 bps / 0.5%)
    assert_eq!(pool_resolved.fee_bps, 50);

    // Claim winnings: fee = 12000 * 50 / 10000 = 60, payout = 11940
    let winnings = client.claim_winnings(&bettor, &pool_id);
    assert_eq!(winnings, 11_940);
    assert_eq!(token.balance(&bettor), 11_940);
    assert_eq!(token.balance(&client.address), 60);
}

/// Tests treasury fee accumulation and withdrawal accuracy across multiple pools and fee tiers.
/// Verifies step-by-step token balance tracking, partial treasury withdrawal, and full cleanup.
#[test]
fn test_treasury_accumulation_and_withdrawal_accuracy_multi_tier() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, token_address, token, token_admin_client, treasury, operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Base fee: 4% (400 bps)
    client.set_fee_bps(&admin, &400u32);
    env.ledger()
        .with_mut(|li| li.timestamp += crate::FEE_CHANGE_TIMELOCK_SECONDS + 1);
    client.apply_fee_bps(&admin);

    // Tiers: Tier 1: 1,000 -> 200 bps (2%), Tier 2: 5,000 -> 100 bps (1%)
    let tiers = Vec::from_array(
        &env,
        [
            FeeTier { stake_threshold: 1_000i128, fee_bps: 200 },
            FeeTier { stake_threshold: 5_000i128, fee_bps: 100 },
        ],
    );
    client.set_fee_tiers(&admin, &tiers);

    let mut accumulated_expected_fees = 0i128;

    // --- Pool 1: Stake = 500 (Base fee 4% -> 400 bps) ---
    // Fee = 500 * 400 / 10000 = 20
    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &500);
    let now1 = env.ledger().timestamp();
    let pid1 = create_pool(&env, &client, &creator, &token_address, now1 + 4_000);
    client.place_prediction(&user1, &pid1, &500, &0, &None, &None);
    env.ledger().with_mut(|li| li.timestamp = now1 + 4_001);
    client.resolve_pool(&operator, &pid1, &0u32);
    client.claim_winnings(&user1, &pid1);

    accumulated_expected_fees += 20;
    assert_eq!(token.balance(&client.address), accumulated_expected_fees);

    // --- Pool 2: Stake = 2,500 (Tier 1 fee 2% -> 200 bps) ---
    // Fee = 2,500 * 200 / 10000 = 50
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user2, &2_500);
    let now2 = env.ledger().timestamp();
    let pid2 = create_pool(&env, &client, &creator, &token_address, now2 + 4_000);
    client.place_prediction(&user2, &pid2, &2_500, &0, &None, &None);
    env.ledger().with_mut(|li| li.timestamp = now2 + 4_001);
    client.resolve_pool(&operator, &pid2, &0u32);
    client.claim_winnings(&user2, &pid2);

    accumulated_expected_fees += 50;
    assert_eq!(token.balance(&client.address), accumulated_expected_fees);

    // --- Pool 3: Stake = 10,000 (Tier 2 fee 1% -> 100 bps) ---
    // Fee = 10,000 * 100 / 10000 = 100
    let user3 = Address::generate(&env);
    token_admin_client.mint(&user3, &10_000);
    let now3 = env.ledger().timestamp();
    let pid3 = create_pool(&env, &client, &creator, &token_address, now3 + 4_000);
    client.place_prediction(&user3, &pid3, &10_000, &0, &None, &None);
    env.ledger().with_mut(|li| li.timestamp = now3 + 4_001);
    client.resolve_pool(&operator, &pid3, &0u32);
    client.claim_winnings(&user3, &pid3);

    accumulated_expected_fees += 100;
    assert_eq!(token.balance(&client.address), accumulated_expected_fees);

    // Total accumulated fees across 3 pools = 20 + 50 + 100 = 170
    assert_eq!(accumulated_expected_fees, 170);

    // --- Partial Treasury Withdrawal ---
    // Admin withdraws 70 tokens to treasury
    client.withdraw_treasury(&admin, &token_address, &70, &treasury);
    assert_eq!(token.balance(&treasury), 70);
    assert_eq!(token.balance(&client.address), 100);

    // --- Full Remaining Treasury Withdrawal ---
    // Admin withdraws remaining 100 tokens to secondary recipient
    let treasury2 = Address::generate(&env);
    client.withdraw_treasury(&admin, &token_address, &100, &treasury2);
    assert_eq!(token.balance(&treasury2), 100);
    assert_eq!(token.balance(&client.address), 0);
}

