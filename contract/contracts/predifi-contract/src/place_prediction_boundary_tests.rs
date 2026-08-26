//! Boundary & Edge Case Tests: `place_prediction`
//!
//! Advanced boundary and edge case coverage for the `place_prediction`
//! entrypoint of the PrediFi prediction market contract (issue #1439).
//!
//! # Scenarios covered
//!
//! 1. **Min-stake boundary** — a stake of exactly `pool.min_stake` succeeds;
//!    one unit below it is rejected with `StakeBelowMinimum`.
//! 2. **Max-stake boundary** — a stake of exactly `pool.max_stake` succeeds;
//!    one unit above it is rejected with `StakeAboveMaximum`.
//! 3. **Pool total-stake cap** — a single stake that exactly fills
//!    `pool.max_total_stake` succeeds; a stake that would push the pool over
//!    the cap is rejected with `MaxTotalStakeExceeded`.
//! 4. **Pool start-time boundary** — a prediction placed at the exact
//!    `pool.start_time` succeeds (the contract only gates on `end_time`).
//! 5. **`i128::MAX` stake** — a single prediction for the maximum possible
//!    `i128` amount is accepted and correctly reflected in `pool.total_stake`.
//! 6. **Prediction cooldown boundary** — a second prediction from the same
//!    user succeeds at exactly `cooldown_seconds` elapsed, and is rejected one
//!    second short of that.
//! 7. **Concurrent (repeated) predictions from the same user** — placing two
//!    predictions on the same pool/outcome accumulates into a single
//!    `Prediction` record without double-counting `participants_count`.

#![cfg(test)]

use crate::{PoolConfig, PredifiContract, PredifiContractClient, PredifiError};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

// ─── Shared dummy access-control stub ────────────────────────────────────────
//
// Mirrors the minimal in-process access-control stub used by the other
// boundary-test modules in this crate (see `claim_winnings_boundary_tests`).

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

// ─── Test environment helpers ─────────────────────────────────────────────────

struct Ctx<'a> {
    client: PredifiContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    token_address: Address,
    admin: Address,
}

/// Deploys the dummy access-control contract, the predifi contract, and a
/// whitelisted Stellar asset token. Ledger timestamp starts at 1_000 so pools
/// can be created with a comfortably future `end_time`.
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
    ac_client.grant_role(&admin, &0u32); // Admin role
    ac_client.grant_role(&operator, &1u32); // Operator role (satisfies required_resolutions >= 1)

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);
    // fee_bps = 0, resolution_delay = 0, min_pool_duration = 3600, max_pred_per_user = 0
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

    let token_deployer = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_deployer);
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
    }
}

/// A minimal two-outcome pool config. `min_total_stake` must be strictly
/// positive per `create_pool`'s validation, even though it's unrelated to the
/// scenarios under test here.
fn base_config(env: &Env) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "place_prediction boundary pool"),
        metadata_url: String::from_str(env, "ipfs://boundary-place-prediction"),
        min_stake: 1i128,
        max_stake: 0i128,
        min_total_stake: 1i128,
        max_total_stake: 0i128,
        initial_liquidity: 0i128,
        required_resolutions: 1u32,
        private: false,
        whitelist_key: None,
        outcome_descriptions: vec![
            env,
            String::from_str(env, "No"),
            String::from_str(env, "Yes"),
        ],
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. MIN-STAKE BOUNDARY
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_stake_of_exactly_min_stake_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let mut config = base_config(&env);
    config.min_stake = 100;

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &100);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &config,
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &100, &0, &None, &None);

    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #107)")]
fn test_stake_one_below_min_stake_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let mut config = base_config(&env);
    config.min_stake = 100;

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &99);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &config,
    );

    // 99 < pool.min_stake (100) but still >= global min_stake (1) — must fail
    // with StakeBelowMinimum (#107), not InsufficientStake.
    ctx.client
        .place_prediction(&bettor, &pool_id, &99, &0, &None, &None);
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. MAX-STAKE BOUNDARY
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_stake_of_exactly_max_stake_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let mut config = base_config(&env);
    config.max_stake = 1_000;

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &1_000);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &config,
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &1_000, &1, &None, &None);

    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, 1_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #108)")]
fn test_stake_one_above_max_stake_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let mut config = base_config(&env);
    config.max_stake = 1_000;

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &1_001);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &config,
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &1_001, &1, &None, &None);
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. POOL TOTAL-STAKE CAP (`max_total_stake`)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_stake_that_exactly_fills_max_total_stake_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let mut config = base_config(&env);
    config.max_total_stake = 500;

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &500);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config,
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &500, &0, &None, &None);

    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, 500);
}

#[test]
#[should_panic(expected = "Error(Contract, #104)")]
fn test_stake_exceeding_max_total_stake_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let mut config = base_config(&env);
    config.max_total_stake = 500;

    let first = Address::generate(&env);
    let second = Address::generate(&env);
    ctx.token_admin.mint(&first, &400);
    ctx.token_admin.mint(&second, &200);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &first,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config,
    );

    // Within cap: 400 <= 500.
    ctx.client
        .place_prediction(&first, &pool_id, &400, &0, &None, &None);

    // 400 + 200 = 600 > 500 — must be rejected with MaxTotalStakeExceeded (#104).
    ctx.client
        .place_prediction(&second, &pool_id, &200, &0, &None, &None);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. POOL START-TIME BOUNDARY
// ═══════════════════════════════════════════════════════════════════════════

/// The contract only gates predictions on `pool.end_time`; there is no
/// explicit check against `pool.start_time`. This test documents and locks
/// in that behaviour: a prediction placed at the exact ledger timestamp
/// equal to `pool.start_time` must succeed.
#[test]
fn test_prediction_at_exact_pool_start_time_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &250);

    let mut config = base_config(&env);
    config.start_time = 5_000;
    let end_time = 20_000u64;

    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &config,
    );

    // Advance the ledger to exactly the pool's start_time.
    env.ledger().with_mut(|li| li.timestamp = 5_000);

    ctx.client
        .place_prediction(&bettor, &pool_id, &250, &1, &None, &None);

    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, 250);
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. `i128::MAX` STAKE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_prediction_with_i128_max_amount_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &i128::MAX);

    let end_time = 10_000u64;
    // Unbounded max_stake / max_total_stake so nothing else interferes.
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &i128::MAX, &0, &None, &None);

    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, i128::MAX);
    assert_eq!(ctx.token.balance(&bettor), 0);
    assert_eq!(ctx.token.balance(&ctx.client.address), i128::MAX);
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. PREDICTION COOLDOWN BOUNDARY
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_second_prediction_at_exact_cooldown_boundary_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    ctx.client.set_prediction_cooldown(&ctx.admin, &60u64);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &200);

    let end_time = 100_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &base_config(&env),
    );

    // First prediction at t = 1_000 (setup's initial ledger timestamp).
    ctx.client
        .place_prediction(&bettor, &pool_id, &100, &0, &None, &None);

    // Advance exactly 60 seconds — the cooldown period — and try again.
    // `now.saturating_sub(last) < cooldown` is false at exactly 60, so this
    // must succeed.
    env.ledger().with_mut(|li| li.timestamp = 1_000 + 60);
    ctx.client
        .place_prediction(&bettor, &pool_id, &100, &0, &None, &None);

    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, 200);
}

#[test]
#[should_panic(expected = "Error(Contract, #190)")]
fn test_second_prediction_one_second_short_of_cooldown_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    ctx.client.set_prediction_cooldown(&ctx.admin, &60u64);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &200);

    let end_time = 100_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &base_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &100, &0, &None, &None);

    // Only 59 seconds elapsed — one second short of the 60s cooldown.
    env.ledger().with_mut(|li| li.timestamp = 1_000 + 59);
    ctx.client
        .place_prediction(&bettor, &pool_id, &100, &0, &None, &None);
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. CONCURRENT / REPEATED PREDICTIONS FROM THE SAME USER
// ═══════════════════════════════════════════════════════════════════════════

/// Two predictions from the same user on the same pool and outcome must
/// accumulate into a single `Prediction` record rather than being tracked as
/// two separate participants.
#[test]
fn test_repeated_predictions_same_user_same_outcome_accumulate() {
    let env = Env::default();
    let ctx = setup(&env);

    // Disable the cooldown so both calls can be made at the same timestamp,
    // simulating near-simultaneous ("concurrent") submissions.
    ctx.client.set_prediction_cooldown(&ctx.admin, &0u64);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &300);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &base_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &120, &1, &None, &None);
    ctx.client
        .place_prediction(&bettor, &pool_id, &180, &1, &None, &None);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool.total_stake, 300,
        "repeated stakes from the same user must accumulate in pool.total_stake"
    );
    // A single participant placed two predictions — participants_count must
    // reflect one unique participant, not two.
    assert_eq!(
        pool.participants_count, 1,
        "repeated predictions from the same user must not double-count participants"
    );

    let predictions = ctx.client.get_user_predictions(&bettor, &0u32, &10u32);
    assert_eq!(
        predictions.len(),
        1,
        "the user should have exactly one aggregated prediction record for this pool"
    );
    assert_eq!(predictions.get(0).unwrap().amount, 300);
}
