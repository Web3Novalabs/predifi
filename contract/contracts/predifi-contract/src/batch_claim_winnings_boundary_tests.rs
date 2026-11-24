//! Advanced Boundary & Edge Case Tests: `batch_claim_winnings`
//!
//! This module provides comprehensive boundary, edge-case, and partial-failure testing
//! for the `batch_claim_winnings` entry point in the PrediFi contract.
//!
//! # Scenarios Covered:
//! 1. Empty `pool_ids` vector (no-op, returns empty Map).
//! 2. Duplicate `pool_ids` in a single batch (duplicate processing, token safety, Map key overwrite behavior).
//! 3. Mixed valid & invalid `pool_ids` (non-existent IDs, active pools, resolved winning/losing pools, canceled pools).
//! 4. Large array limits / stress testing (large batches of 50+ pools).
//! 5. Partial failure handling & isolation (failures in early/middle pools do not abort subsequent valid claims).
//! 6. Multi-pool claims with referral rewards & fee distribution.
//! 7. Pause status & authorization enforcement.

#![cfg(test)]

use crate::{MarketState, PoolConfig, PredifiContract, PredifiContractClient, PredifiError};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, Map, String, Vec,
};

// ─── Shared dummy access-control stub ────────────────────────────────────────

mod dummy_ac {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct DummyAC;

    #[contractimpl]
    impl DummyAC {
        pub fn grant_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user.clone(), role);
            let already: bool = env.storage().instance().get(&key).unwrap_or(false);
            env.storage().instance().set(&key, &true);
            if role == 1 && !already {
                let ck = Symbol::new(&env, "op_count");
                let c: u32 = env.storage().instance().get(&ck).unwrap_or(0);
                env.storage().instance().set(&ck, &(c + 1));
            }
        }

        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }

        pub fn get_operator_count(env: Env) -> u32 {
            env.storage()
                .instance()
                .get(&Symbol::new(&env, "op_count"))
                .unwrap_or(0)
        }
    }
}

// ─── Test Context Setup ──────────────────────────────────────────────────────

struct Ctx<'a> {
    client: PredifiContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    token_address: Address,
    admin: Address,
    operator: Address,
    treasury: Address,
}

fn setup(env: &Env) -> Ctx<'_> {
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.protocol_version = 23;
        li.timestamp = 1_000;
    });

    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let treasury = Address::generate(env);

    let ac_id = env.register(dummy_ac::DummyAC, ());
    let ac_client = dummy_ac::DummyACClient::new(env, &ac_id);
    ac_client.grant_role(&admin, &0u32);    // Admin role
    ac_client.grant_role(&operator, &1u32); // Operator role

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);
    // fee_bps = 0, resolution_delay = 0, min_pool_duration = 3600, max_pred_per_user = 0
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
    client.set_prediction_cooldown(&admin, &0u64);

    let token_deployer = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_deployer.clone());
    let token_address = token_contract.address();
    let token = token::Client::new(env, &token_address);
    let token_admin = token::StellarAssetClient::new(env, &token_address);

    client.add_token_to_whitelist(&admin, &token_address);

    Ctx {
        client,
        token,
        token_admin,
        token_address,
        admin,
        operator,
        treasury,
    }
}

fn create_test_pool(ctx: &Ctx<'_>, env: &Env, desc: &str) -> u64 {
    ctx.client.create_pool(
        &ctx.admin,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(env, desc),
            metadata_url: String::from_str(env, "ipfs://batch-test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                env,
                String::from_str(env, "Outcome 0"),
                String::from_str(env, "Outcome 1"),
            ],
        },
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. EMPTY POOL IDS ARRAY
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_claim_empty_array_returns_empty_map() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    let empty_ids: Vec<u64> = vec![&env];

    let results = ctx.client.batch_claim_winnings(&user, &empty_ids);

    assert_eq!(results.len(), 0, "empty input should return empty map");
    assert_eq!(ctx.token.balance(&ctx.client.address), 0);
    assert_eq!(ctx.token.balance(&user), 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. DUPLICATE POOL IDS IN ARRAY
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_claim_duplicate_pool_ids_handles_double_claim_gracefully() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let pool_a = create_test_pool(&ctx, &env, "Pool A");
    let pool_b = create_test_pool(&ctx, &env, "Pool B");

    // User places predictions on both pools
    ctx.client.place_prediction(&user, &pool_a, &400, &0, &None, &None);
    ctx.client.place_prediction(&user, &pool_b, &600, &1, &None, &None);

    // Resolve pools
    env.ledger().with_mut(|l| l.timestamp = 100_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_a, &0u32);
    ctx.client.resolve_pool(&ctx.operator, &pool_b, &1u32);

    // Pass duplicate IDs: [pool_a, pool_b, pool_a, pool_a]
    let pool_ids = vec![&env, pool_a, pool_b, pool_a, pool_a];
    let results = ctx.client.batch_claim_winnings(&user, &pool_ids);

    // Results map is keyed by pool_id, so length is unique pool IDs count = 2
    assert_eq!(results.len(), 2);

    // Tokens should be transferred once per pool (total 1000)
    assert_eq!(ctx.token.balance(&user), 1_000, "user should receive full 1000 winnings");
    assert_eq!(ctx.token.balance(&ctx.client.address), 0, "contract balance drained");

    // Duplicate iterations hit AlreadyClaimed on subsequent passes for pool_a, setting map to 0
    assert_eq!(results.get(pool_a).unwrap(), 0, "subsequent duplicate pass yields 0 in Map");
    assert_eq!(results.get(pool_b).unwrap(), 600, "pool_b claimed 600");
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. MIX OF VALID AND INVALID POOL IDS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_claim_mix_of_valid_and_invalid_pool_ids() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let pool_winning = create_test_pool(&ctx, &env, "Winning Pool");
    let pool_losing = create_test_pool(&ctx, &env, "Losing Pool");
    let pool_active = create_test_pool(&ctx, &env, "Active Pool");
    let pool_canceled = create_test_pool(&ctx, &env, "Canceled Pool");
    let nonexistent_id_1: u64 = 99_999;
    let nonexistent_id_2: u64 = 88_888;

    // User stakes
    ctx.client.place_prediction(&user, &pool_winning, &300, &0, &None, &None);
    ctx.client.place_prediction(&user, &pool_losing, &200, &0, &None, &None);
    ctx.client.place_prediction(&user, &pool_active, &100, &0, &None, &None);
    ctx.client.place_prediction(&user, &pool_canceled, &400, &0, &None, &None);

    // Resolve winning and losing pools, cancel pool_canceled
    env.ledger().with_mut(|l| l.timestamp = 100_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_winning, &0u32);
    ctx.client.resolve_pool(&ctx.operator, &pool_losing, &1u32); // user bet 0, outcome is 1 -> user lost
    ctx.client.cancel_pool(&ctx.operator, &pool_canceled, &String::from_str(&env, "canceled reason"));

    // Batch claim with valid winning, valid losing, active, canceled, non-existent IDs
    let pool_ids = vec![
        &env,
        pool_winning,
        nonexistent_id_1,
        pool_losing,
        pool_active,
        pool_canceled,
        nonexistent_id_2,
    ];

    let results = ctx.client.batch_claim_winnings(&user, &pool_ids);

    assert_eq!(results.len(), 6, "all 6 requested IDs mapped in result");
    assert_eq!(results.get(pool_winning).unwrap(), 300, "winning pool returns 300");
    assert_eq!(results.get(pool_losing).unwrap(), 0, "losing pool returns 0");
    assert_eq!(results.get(pool_active).unwrap(), 0, "active pool returns 0");
    assert_eq!(results.get(pool_canceled).unwrap(), 400, "canceled pool returns 400 refund");
    assert_eq!(results.get(nonexistent_id_1).unwrap(), 0, "non-existent ID 1 returns 0");
    assert_eq!(results.get(nonexistent_id_2).unwrap(), 0, "non-existent ID 2 returns 0");

    // Total user payout: 300 (winning) + 400 (canceled refund) = 700
    assert_eq!(ctx.token.balance(&user), 700);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. LARGE ARRAY SIZE LIMITS / STRESS TESTING
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_claim_large_batch_50_pools() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    let total_pools = 50u64;
    let stake_per_pool = 10i128;

    ctx.token_admin.mint(&user, &(total_pools as i128 * stake_per_pool));

    let mut pool_ids = vec![&env];
    for _i in 0..total_pools {
        let pool_id = create_test_pool(&ctx, &env, "Batch Pool");
        ctx.client.place_prediction(&user, &pool_id, &stake_per_pool, &0, &None, &None);
        pool_ids.push_back(pool_id);
    }

    env.ledger().with_mut(|l| l.timestamp = 100_001);

    // Resolve half the pools to 0 (user wins) and half to 1 (user loses)
    for i in 0..total_pools {
        let outcome = if i % 2 == 0 { 0u32 } else { 1u32 };
        ctx.client.resolve_pool(&ctx.operator, &pool_ids.get(i as u32).unwrap(), &outcome);
    }

    let results = ctx.client.batch_claim_winnings(&user, &pool_ids);

    assert_eq!(results.len(), 50);

    let mut expected_claimed_total = 0i128;
    for i in 0..total_pools {
        let pid = pool_ids.get(i as u32).unwrap();
        let claimed = results.get(pid).unwrap();
        if i % 2 == 0 {
            assert_eq!(claimed, stake_per_pool, "even indexed pool is winning");
            expected_claimed_total += stake_per_pool;
        } else {
            assert_eq!(claimed, 0, "odd indexed pool is losing");
        }
    }

    assert_eq!(ctx.token.balance(&user), expected_claimed_total);
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. PARTIAL FAILURE HANDLING & ISOLATION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_claim_partial_failure_isolation() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let pool_1_win = create_test_pool(&ctx, &env, "Win Pool 1");
    let pool_2_already_claimed = create_test_pool(&ctx, &env, "Already Claimed Pool");
    let pool_3_win = create_test_pool(&ctx, &env, "Win Pool 3");

    ctx.client.place_prediction(&user, &pool_1_win, &200, &0, &None, &None);
    ctx.client.place_prediction(&user, &pool_2_already_claimed, &300, &0, &None, &None);
    ctx.client.place_prediction(&user, &pool_3_win, &500, &0, &None, &None);

    env.ledger().with_mut(|l| l.timestamp = 100_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_1_win, &0u32);
    ctx.client.resolve_pool(&ctx.operator, &pool_2_already_claimed, &0u32);
    ctx.client.resolve_pool(&ctx.operator, &pool_3_win, &0u32);

    // Individually claim pool 2 first
    let claimed_2 = ctx.client.claim_winnings(&user, &pool_2_already_claimed);
    assert_eq!(claimed_2, 300);

    // Now batch claim [pool_1_win, pool_2_already_claimed, pool_3_win]
    let pool_ids = vec![&env, pool_1_win, pool_2_already_claimed, pool_3_win];
    let results = ctx.client.batch_claim_winnings(&user, &pool_ids);

    assert_eq!(results.get(pool_1_win).unwrap(), 200, "pool 1 claimed 200");
    assert_eq!(results.get(pool_2_already_claimed).unwrap(), 0, "pool 2 double claim yields 0");
    assert_eq!(results.get(pool_3_win).unwrap(), 500, "pool 3 claimed 500 despite pool 2 failure");

    // Total user balance: 300 (pre-claimed) + 200 (pool 1) + 500 (pool 3) = 1000
    assert_eq!(ctx.token.balance(&user), 1_000);
}

fn setup_with_fee(env: &Env, fee_bps: u32) -> Ctx<'_> {
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.protocol_version = 23;
        li.timestamp = 1_000;
    });

    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let treasury = Address::generate(env);

    let ac_id = env.register(dummy_ac::DummyAC, ());
    let ac_client = dummy_ac::DummyACClient::new(env, &ac_id);
    ac_client.grant_role(&admin, &0u32);
    ac_client.grant_role(&operator, &1u32);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);
    client.init(&ac_id, &treasury, &fee_bps, &0u64, &3600u64, &0u32);
    client.set_prediction_cooldown(&admin, &0u64);

    let token_deployer = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_deployer.clone());
    let token_address = token_contract.address();
    let token = token::Client::new(env, &token_address);
    let token_admin = token::StellarAssetClient::new(env, &token_address);

    client.add_token_to_whitelist(&admin, &token_address);

    Ctx {
        client,
        token,
        token_admin,
        token_address,
        admin,
        operator,
        treasury,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. REFERRAL REWARDS & PROTOCOL FEES IN BATCH CLAIM
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_claim_with_referrals_and_fees() {
    let env = Env::default();
    let ctx = setup_with_fee(&env, 500); // 5% fee configured at init

    let referrer = Address::generate(&env);
    let winner = Address::generate(&env);
    let loser = Address::generate(&env);

    ctx.token_admin.mint(&winner, &1_000);
    ctx.token_admin.mint(&loser, &1_000);

    ctx.client.set_referral_cut_bps(&ctx.admin, &1000u32); // 10% cut of fee

    let pool_a = create_test_pool(&ctx, &env, "Pool A with Referral");
    let pool_b = create_test_pool(&ctx, &env, "Pool B without Referral");

    // Winner places predictions with referrer on pool A, without referrer on pool B
    ctx.client.place_prediction(&winner, &pool_a, &500, &0, &Some(referrer.clone()), &None);
    ctx.client.place_prediction(&loser, &pool_a, &500, &1, &None, &None);

    ctx.client.place_prediction(&winner, &pool_b, &500, &0, &None, &None);
    ctx.client.place_prediction(&loser, &pool_b, &500, &1, &None, &None);

    env.ledger().with_mut(|l| l.timestamp = 100_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_a, &0u32);
    ctx.client.resolve_pool(&ctx.operator, &pool_b, &0u32);

    let batch_ids = vec![&env, pool_a, pool_b];
    let results = ctx.client.batch_claim_winnings(&winner, &batch_ids);

    assert_eq!(results.len(), 2);
    assert!(results.get(pool_a).unwrap() > 0);
    assert!(results.get(pool_b).unwrap() > 0);

    // Verify referrer received referral cut from pool_a
    assert!(ctx.token.balance(&referrer) > 0, "referrer should receive cut from batch claim");
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. CONTRACT PAUSED AND AUTH ENFORCEMENT
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_claim_blocked_when_contract_paused() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    let pool_id = create_test_pool(&ctx, &env, "Paused Pool");

    ctx.client.pause(&ctx.admin);

    let res = ctx.client.try_batch_claim_winnings(&user, &vec![&env, pool_id]);
    assert_eq!(res, Err(Ok(PredifiError::ContractPaused)));
}
