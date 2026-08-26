//! Boundary & Edge Case Tests: `resolve_pool`
//!
//! Advanced boundary and edge case coverage for the `resolve_pool` entrypoint
//! of the PrediFi prediction market contract (issue #1442).
//!
//! # Scenarios covered
//!
//! 1. **No predictions** — resolving a pool nobody staked into (`total_stake
//!    == 0`) must succeed without panicking.
//! 2. **Only one side has stakes** — resolving to the side that received all
//!    the stakes, and resolving to the side that received none, both succeed.
//! 3. **Exact `resolution_delay` boundary** — resolution one second before
//!    `end_time + resolution_delay` is rejected; at the exact boundary it
//!    succeeds.
//! 4. **Maximum number of participants** — a pool with a large number of
//!    distinct bettors still resolves correctly and its `participants_count`
//!    matches.
//! 5. **Re-resolution prevention** — a resolved pool cannot be resolved again
//!    (`InvalidPoolState`), and a duplicate vote from the same operator before
//!    the threshold is reached is rejected (`OracleAlreadyVoted`).
//! 6. **Authorization & outcome validation** — non-operator callers and
//!    out-of-bounds / sentinel outcome values are rejected.
//!
//! Note: the parent issue also mentions "oracle price edge cases (price = 0,
//! price = i128::MAX)". Those apply to the separate price-feed-driven
//! resolution path (`update_price_feed` / `resolve_pool_from_price` in
//! `oracle.rs`), not to this operator-voting `resolve_pool` entrypoint, so
//! they are out of scope for this file and are not covered here.

#![cfg(test)]

use crate::{MarketState, PoolConfig, PredifiContract, PredifiContractClient, PredifiError};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
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

// ─── Test environment helpers ─────────────────────────────────────────────────

struct Ctx<'a> {
    client: PredifiContractClient<'a>,
    token_admin: token::StellarAssetClient<'a>,
    token_address: Address,
    #[allow(dead_code)]
    admin: Address,
    operator: Address,
}

/// Deploys the dummy access-control contract, the predifi contract (with
/// `resolution_delay = 100` seconds), and a whitelisted Stellar asset token.
/// The ledger timestamp starts at 1_000.
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
    ac_client.grant_role(&admin, &0u32);
    ac_client.grant_role(&operator, &1u32);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);
    // fee_bps = 0, resolution_delay = 100, min_pool_duration = 3600, max_pred_per_user = 0
    client.init(&ac_id, &treasury, &0u32, &100u64, &3600u64, &0u32);

    let token_deployer = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_deployer);
    let token_address = token_contract.address();
    let token_admin = token::StellarAssetClient::new(env, &token_address);

    client.add_token_to_whitelist(&admin, &token_address);

    Ctx {
        client,
        token_admin,
        token_address,
        admin,
        operator,
    }
}

fn base_config(env: &Env) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "resolve_pool boundary pool"),
        metadata_url: String::from_str(env, "ipfs://boundary-resolve-pool"),
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
// 1. NO PREDICTIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_resolve_pool_with_no_predictions_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    // No predictions placed. Advance past end_time + resolution_delay.
    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 0u32);
    assert_eq!(pool.total_stake, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. ONLY ONE SIDE HAS STAKES
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_resolve_pool_when_only_winning_side_has_stakes_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &1_000);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &base_config(&env),
    );

    // All stake is on outcome 1; outcome 0 has zero stake.
    ctx.client
        .place_prediction(&bettor, &pool_id, &1_000, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &1u32);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 1u32);
    assert_eq!(ctx.client.get_outcome_stake(&pool_id, &0u32), 0);
    assert_eq!(ctx.client.get_outcome_stake(&pool_id, &1u32), 1_000);
}

#[test]
fn test_resolve_pool_to_the_side_with_zero_stake_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &1_000);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &base_config(&env),
    );

    // Everybody bets on outcome 1.
    ctx.client
        .place_prediction(&bettor, &pool_id, &1_000, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    // Operator resolves to outcome 0 — the side with zero stake. Must not
    // panic; the pool simply has no winners.
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 0u32);
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. EXACT `resolution_delay` BOUNDARY
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "Error(Contract, #81)")]
fn test_resolve_pool_one_second_before_delay_elapses_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    // eligible_at = end_time + resolution_delay(100) = 10_100.
    // One second short of that boundary.
    env.ledger().with_mut(|li| li.timestamp = end_time + 99);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
}

#[test]
fn test_resolve_pool_at_exact_delay_boundary_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    // Exactly at eligible_at = end_time + resolution_delay.
    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);

    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Resolved);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. MAXIMUM NUMBER OF PARTICIPANTS
// ═══════════════════════════════════════════════════════════════════════════

/// Resolves a pool with a large number of distinct participants to verify
/// `resolve_pool` (and the stake bookkeeping it depends on) scales correctly
/// and `participants_count` is accurate at resolution time.
#[test]
fn test_resolve_pool_with_many_participants_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    let participant_count = 50u32;
    let mut total_expected: i128 = 0;
    for i in 0..participant_count {
        let user = Address::generate(&env);
        let amount: i128 = 10 + i as i128;
        ctx.token_admin.mint(&user, &amount);
        // Alternate outcomes so both sides accumulate stake.
        let outcome = i % 2;
        ctx.client
            .place_prediction(&user, &pool_id, &amount, &outcome, &None, &None);
        total_expected += amount;
    }

    let pool_before = ctx.client.get_pool(&pool_id);
    assert_eq!(pool_before.participants_count, participant_count);
    assert_eq!(pool_before.total_stake, total_expected);

    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &1u32);

    let pool_after = ctx.client.get_pool(&pool_id);
    assert_eq!(pool_after.state, MarketState::Resolved);
    assert_eq!(pool_after.outcome, 1u32);
    assert_eq!(pool_after.participants_count, participant_count);
    assert_eq!(pool_after.total_stake, total_expected);
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. RE-RESOLUTION PREVENTION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_resolve_pool_after_finalized_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Resolved);

    // Second attempt (even to a different outcome) must be rejected with
    // InvalidPoolState (#24) because the pool is no longer Active.
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &1u32);
}

/// More precise variant of the re-resolution guard using `try_resolve_pool`
/// to assert the exact typed error without relying on panic message text.
#[test]
fn test_resolve_pool_after_finalized_returns_invalid_pool_state() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);

    let result = ctx.client.try_resolve_pool(&ctx.operator, &pool_id, &1u32);
    assert_eq!(
        result.unwrap_err().unwrap(),
        PredifiError::InvalidPoolState,
        "resolving an already-resolved pool must return InvalidPoolState"
    );
}

/// Before the resolution threshold is reached, the *same* operator voting a
/// second time for the same pool must be rejected with `OracleAlreadyVoted`.
#[test]
fn test_duplicate_vote_by_same_operator_before_threshold_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let operator_2 = Address::generate(&env);
    // Register a second operator directly against the same access-control
    // contract so `required_resolutions = 2` is satisfiable.
    let ac_client_for_second =
        dummy_ac::DummyACClient::new(&env, &super_ac_address(&env, &ctx));
    ac_client_for_second.grant_role(&operator_2, &1u32);

    let mut config = base_config(&env);
    config.required_resolutions = 2;

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &config,
    );

    env.ledger().with_mut(|li| li.timestamp = end_time + 100);

    // First vote from `operator` — threshold (2) not yet reached, pool
    // remains Active.
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Active);

    // Second vote attempt from the *same* operator — must be rejected.
    let result = ctx.client.try_resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(
        result.unwrap_err().unwrap(),
        PredifiError::OracleAlreadyVoted
    );
}

/// Helper to recover the access-control contract address registered during
/// `setup`, since `Ctx` does not expose it directly. Reads it back out of the
/// predifi contract's own `Config` via `get_contract_info`.
fn super_ac_address(_env: &Env, ctx: &Ctx<'_>) -> Address {
    ctx.client.get_contract_info().access_control
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. AUTHORIZATION & OUTCOME VALIDATION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_resolve_pool_by_non_operator_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let non_operator = Address::generate(&env);
    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    let result = ctx
        .client
        .try_resolve_pool(&non_operator, &pool_id, &0u32);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::Unauthorized);
}

#[test]
fn test_resolve_pool_with_out_of_bounds_outcome_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    // options_count == 2, so valid outcomes are {0, 1}. Outcome 2 is invalid.
    let result = ctx.client.try_resolve_pool(&ctx.operator, &pool_id, &2u32);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::InvalidOutcome);
}

#[test]
fn test_resolve_pool_with_sentinel_outcome_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    env.ledger().with_mut(|li| li.timestamp = end_time + 100);
    let result = ctx
        .client
        .try_resolve_pool(&ctx.operator, &pool_id, &u32::MAX);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::InvalidOutcome);
}
