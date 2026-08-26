//! Boundary & Edge Case Tests: `claim_winnings`
//!
//! This module provides advanced boundary and edge case coverage for the
//! `claim_winnings` function in the PrediFi prediction market contract.
//!
//! # Scenarios covered
//!
//! 1. **Single participant** — a pool where only one user staked; they win the
//!    entire pot (their own stake back, minus any protocol fee).
//!
//! 2. **Zero winning-side stake** — the resolved winning outcome has zero total
//!    stake (because no user bet on it, e.g. initial-liquidity-only pool or
//!    resolution to an unbetted outcome).  All claimants should receive 0 and
//!    the contract must not panic.
//!
//! 3. **Claim window boundaries** — verifies that:
//!    - Claiming is rejected while the pool is still `Active` (`PoolNotResolved`).
//!    - Claiming succeeds at the exact moment the pool transitions to `Resolved`.
//!    - Claiming continues to succeed long after resolution (no expiry).
//!
//! 4. **Double-claim prevention** — a user who already claimed must receive
//!    `AlreadyClaimed` on a second attempt, and the contract balance must not
//!    decrease a second time.
//!
//! 5. **All participants chose the same outcome** — every bettor wins; their
//!    individual payouts must sum to exactly `total_stake` (INV-5) and the
//!    contract balance drains to zero.

#![cfg(test)]

use crate::{MarketState, PoolConfig, PredifiContract, PredifiContractClient, PredifiError};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

// ─── Shared dummy access-control stub ────────────────────────────────────────
//
// A minimal in-process implementation of the access-control interface used by
// the predifi contract.  All roles are stored in instance storage so the same
// contract address can serve multiple concurrent test pools.

mod dummy_ac {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct DummyAC;

    #[contractimpl]
    impl DummyAC {
        /// Grants `role` to `user` and tracks the operator count for role 1.
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

        /// Returns `true` when `user` holds `role`.
        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }

        /// Returns the number of addresses holding the Operator role (role = 1).
        pub fn get_operator_count(env: Env) -> u32 {
            env.storage()
                .instance()
                .get(&Symbol::new(&env, "op_count"))
                .unwrap_or(0)
        }
    }
}

// ─── Test environment helpers ─────────────────────────────────────────────────

/// Full test context returned by [`setup`].
struct Ctx<'a> {
    client: PredifiContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    token_address: Address,
    admin: Address,
    operator: Address,
    treasury: Address,
}

/// Deploys the dummy access-control contract, the predifi contract, and a
/// whitelisted Stellar asset token.
///
/// The ledger timestamp is initialised to 1_000 so tests can place pools with
/// an `end_time` that is safely in the future.
/// `resolution_delay` is set to 0 so tests can resolve immediately after
/// `end_time` without adding extra ledger manipulation.
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

/// Builds a minimal two-outcome [`PoolConfig`] with no fee, no liquidity, and
/// no stake limits.  Tests that need different settings clone and modify.
fn two_outcome_config(env: &Env) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "Boundary test pool"),
        metadata_url: String::from_str(env, "ipfs://boundary"),
        min_stake: 1i128,
        max_stake: 0i128,
        min_total_stake: 0i128,
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

/// Convenience: advance ledger to `ts` and resolve `pool_id` to `outcome`.
fn resolve_at(ctx: &Ctx<'_>, env: &Env, pool_id: u64, ts: u64, outcome: u32) {
    env.ledger().with_mut(|li| li.timestamp = ts);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &outcome);
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. SINGLE PARTICIPANT
// ═══════════════════════════════════════════════════════════════════════════

/// A pool with exactly one participant who bets on the winning outcome.
///
/// With no protocol fee the winner receives back exactly their stake (the full
/// pot), i.e. `winnings == stake`.  The contract balance must drain to zero.
///
/// This is the degenerate lower bound of "multiple winners" — one participant
/// gets 100 % of the pot.
#[test]
fn test_single_participant_wins_entire_pot() {
    let env = Env::default();
    let ctx = setup(&env);

    let sole_bettor = Address::generate(&env);
    ctx.token_admin.mint(&sole_bettor, &500);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &sole_bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&sole_bettor, &pool_id, &500, &1, &None, &None);
    assert_eq!(ctx.token.balance(&ctx.client.address), 500);

    resolve_at(&ctx, &env, pool_id, end_time, 1);

    let winnings = ctx.client.claim_winnings(&sole_bettor, &pool_id);

    // The sole bettor staked 500 on the winning outcome.
    // winning_stake == total_stake == 500; payout_pool == 500 (0 % fee).
    // winnings = (500 / 500) * 500 = 500.
    assert_eq!(winnings, 500, "sole winner should receive the full pot");
    assert_eq!(
        ctx.token.balance(&sole_bettor),
        500,
        "bettor balance should be restored to original amount"
    );
    assert_eq!(
        ctx.token.balance(&ctx.client.address),
        0,
        "contract must hold no residual funds after sole-winner claim"
    );
}

/// A single participant bets on outcome 0; the operator resolves to outcome 0.
/// Verifies the "sole loser" mirror case: the participant chose correctly, so
/// the result should be identical to `test_single_participant_wins_entire_pot`.
#[test]
fn test_single_participant_outcome_zero_wins() {
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
        &two_outcome_config(&env),
    );

    // Bet on outcome 0 (the first option — must not be confused with
    // UNRESOLVED_OUTCOME which is u32::MAX).
    ctx.client
        .place_prediction(&bettor, &pool_id, &1_000, &0, &None, &None);

    resolve_at(&ctx, &env, pool_id, end_time, 0);

    let winnings = ctx.client.claim_winnings(&bettor, &pool_id);
    assert_eq!(winnings, 1_000);
    assert_eq!(ctx.token.balance(&ctx.client.address), 0);
}

/// A single participant bets on the losing outcome.
/// `claim_winnings` must return 0 (not panic) and not transfer any tokens.
#[test]
fn test_single_participant_loses_returns_zero() {
    let env = Env::default();
    let ctx = setup(&env);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &300);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &two_outcome_config(&env),
    );

    // Bet on outcome 0 but pool resolves to outcome 1.
    ctx.client
        .place_prediction(&bettor, &pool_id, &300, &0, &None, &None);

    resolve_at(&ctx, &env, pool_id, end_time, 1);

    let winnings = ctx.client.claim_winnings(&bettor, &pool_id);
    assert_eq!(
        winnings, 0,
        "loser in single-participant pool must receive 0"
    );
    // Bettor's balance remains 0 (they staked everything and lost).
    assert_eq!(ctx.token.balance(&bettor), 0);
    // Stakes are still held by the contract (no winner to claim them).
    assert_eq!(ctx.token.balance(&ctx.client.address), 300);
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. ZERO WINNING-SIDE STAKE
// ═══════════════════════════════════════════════════════════════════════════

/// The operator resolves the pool to an outcome that received zero bets.
///
/// All real bettors staked on outcome 0; the operator resolves to outcome 1.
/// Nobody bet on outcome 1, so `winning_stake == 0`.
///
/// Contract invariant from `claim_winnings_internal`:
/// ```text
/// if winning_stake == 0 { return Ok(0) }
/// ```
/// Every user must receive 0 and the call must not panic.
#[test]
fn test_claim_when_winning_outcome_has_zero_stake_returns_zero() {
    let env = Env::default();
    let ctx = setup(&env);

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    ctx.token_admin.mint(&user_a, &400);
    ctx.token_admin.mint(&user_b, &600);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &user_a,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Politics"),
        &two_outcome_config(&env),
    );

    // Both users stake on outcome 0; outcome 1 has zero stake.
    ctx.client
        .place_prediction(&user_a, &pool_id, &400, &0, &None, &None);
    ctx.client
        .place_prediction(&user_b, &pool_id, &600, &0, &None, &None);

    assert_eq!(ctx.token.balance(&ctx.client.address), 1_000);

    // Resolve to outcome 1 — which has zero stake.
    resolve_at(&ctx, &env, pool_id, end_time, 1);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool.state,
        MarketState::Resolved,
        "pool should be in Resolved state"
    );
    assert_eq!(pool.outcome, 1u32, "pool should be resolved to outcome 1");

    // Both users bet on outcome 0 (losing side); claim must return 0.
    let claim_a = ctx.client.claim_winnings(&user_a, &pool_id);
    let claim_b = ctx.client.claim_winnings(&user_b, &pool_id);

    assert_eq!(
        claim_a, 0,
        "user_a claim must be 0 when winning side has zero stake"
    );
    assert_eq!(
        claim_b, 0,
        "user_b claim must be 0 when winning side has zero stake"
    );

    // Funds remain in the contract — no transfers occurred.
    assert_eq!(ctx.token.balance(&ctx.client.address), 1_000);
}

/// A pool where nobody placed any predictions is resolved (possible via
/// initial_liquidity only scenario or admin force-resolve edge case).
/// Any address that never placed a prediction must receive 0 without error.
#[test]
fn test_claim_from_pool_with_no_predictions_returns_zero() {
    let env = Env::default();
    let ctx = setup(&env);

    let random_user = Address::generate(&env);
    let creator = Address::generate(&env);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &two_outcome_config(&env),
    );

    // Nobody places a prediction — pool has 0 total_stake.
    resolve_at(&ctx, &env, pool_id, end_time, 0);

    // A user who never interacted with the pool calls claim_winnings.
    let claim = ctx.client.claim_winnings(&random_user, &pool_id);
    assert_eq!(
        claim, 0,
        "claim from pool with no predictions must return 0"
    );
    assert_eq!(ctx.token.balance(&ctx.client.address), 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. CLAIM WINDOW BOUNDARIES
// ═══════════════════════════════════════════════════════════════════════════

/// Attempting to claim while the pool is still `Active` (before end_time and
/// before any resolution) must return `Err(PoolNotResolved)` (error code #22).
///
/// The claim window does not open until the pool state leaves `Active`.
#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_claim_while_pool_is_active_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &100);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &100, &1, &None, &None);

    // Pool is still Active — claim must panic with PoolNotResolved (#22).
    ctx.client.claim_winnings(&bettor, &pool_id);
}

/// Claim succeeds at the exact ledger timestamp at which the pool transitions
/// to `Resolved` (resolution_delay = 0, so eligible_at == end_time).
///
/// This verifies that the claim window opens immediately after resolution,
/// with no off-by-one in the timestamp comparison.
#[test]
fn test_claim_succeeds_at_exact_resolution_timestamp() {
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
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &1_000, &1, &None, &None);

    // Advance to exactly end_time and resolve (resolution_delay = 0).
    env.ledger().with_mut(|li| li.timestamp = end_time);
    ctx.client
        .resolve_pool(&ctx.operator, &pool_id, &1u32);

    // Claim at the same timestamp — must succeed immediately.
    let winnings = ctx.client.claim_winnings(&bettor, &pool_id);
    assert_eq!(
        winnings, 1_000,
        "claim must succeed at exact resolution timestamp"
    );
    assert_eq!(ctx.token.balance(&ctx.client.address), 0);
}

/// Claim succeeds long after resolution — the contract must not impose any
/// expiry on the claim window once a pool is resolved.
///
/// Simulates a user who waits 30 days (in ledger seconds) before claiming.
#[test]
fn test_claim_succeeds_long_after_resolution() {
    let env = Env::default();
    let ctx = setup(&env);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &200);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &200, &0, &None, &None);

    resolve_at(&ctx, &env, pool_id, end_time, 0);

    // Jump 30 days into the future (30 * 86_400 = 2_592_000 seconds).
    let thirty_days_later = end_time + 2_592_000;
    env.ledger().with_mut(|li| li.timestamp = thirty_days_later);

    let winnings = ctx.client.claim_winnings(&bettor, &pool_id);
    assert_eq!(
        winnings, 200,
        "claim must succeed 30 days after resolution"
    );
    assert_eq!(ctx.token.balance(&ctx.client.address), 0);
}

/// Attempting to claim one second before the pool ends (i.e. while still
/// `Active`) must be rejected.  This is the boundary immediately inside the
/// forbidden zone, complementing the "at exact resolution" test above.
#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_claim_one_second_before_end_time_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &50);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &50, &1, &None, &None);

    // Advance to one second before end_time — pool is still Active.
    env.ledger().with_mut(|li| li.timestamp = end_time - 1);

    // Must panic with PoolNotResolved (#22).
    ctx.client.claim_winnings(&bettor, &pool_id);
}

/// Claim succeeds at the exact `claim_window_seconds` deadline
/// (`resolution_timestamp + claim_window_seconds`), using a short,
/// admin-configured window (`MIN_CLAIM_WINDOW` = 86_400s / 1 day) so the
/// boundary can be exercised without simulating the default 30-day window.
///
/// Complements `test_claim_succeeds_long_after_resolution`, which only
/// verifies that claiming still works long after resolution under the
/// *default* window; this test pins down the precise deadline itself
/// (`current_time == claim_deadline` must succeed — the check in
/// `claim_winnings_internal` is `current_time > claim_deadline`, so equality
/// is inside the allowed window).
#[test]
fn test_claim_at_exact_claim_window_deadline_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);

    // Shrink the claim window to the protocol minimum (1 day) via admin.
    ctx.client.set_claim_window(&ctx.admin, &86_400u64);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &1_000);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &1_000, &1, &None, &None);

    resolve_at(&ctx, &env, pool_id, end_time, 1);
    // resolution_timestamp == end_time (resolution_delay = 0).
    let claim_deadline = end_time + 86_400;

    env.ledger().with_mut(|li| li.timestamp = claim_deadline);
    let winnings = ctx.client.claim_winnings(&bettor, &pool_id);
    assert_eq!(
        winnings, 1_000,
        "claim must succeed at the exact claim window deadline"
    );
}

/// One second past the claim window deadline, claims must be rejected with
/// `InvalidTimestamp` (#80) — the mirror boundary case of the test above.
#[test]
#[should_panic(expected = "Error(Contract, #80)")]
fn test_claim_one_second_after_claim_window_deadline_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    ctx.client.set_claim_window(&ctx.admin, &86_400u64);

    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &1_000);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &bettor,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &1_000, &1, &None, &None);

    resolve_at(&ctx, &env, pool_id, end_time, 1);
    let claim_deadline = end_time + 86_400;

    // One second past the deadline — must be rejected.
    env.ledger()
        .with_mut(|li| li.timestamp = claim_deadline + 1);
    ctx.client.claim_winnings(&bettor, &pool_id);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. DOUBLE-CLAIM PREVENTION
// ═══════════════════════════════════════════════════════════════════════════

/// A winner who calls `claim_winnings` twice must receive `AlreadyClaimed`
/// (error code #60) on the second call.
///
/// Protocol invariant INV-3: `HasClaimed(user, pool)` is write-once.
#[test]
#[should_panic(expected = "Error(Contract, #60)")]
fn test_winner_double_claim_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let winner = Address::generate(&env);
    ctx.token_admin.mint(&winner, &1_000);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &winner,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&winner, &pool_id, &1_000, &1, &None, &None);
    resolve_at(&ctx, &env, pool_id, end_time, 1);

    // First claim — succeeds.
    let w1 = ctx.client.claim_winnings(&winner, &pool_id);
    assert_eq!(w1, 1_000);

    // Second claim — must panic with AlreadyClaimed (#60).
    ctx.client.claim_winnings(&winner, &pool_id);
}

/// A *loser* who calls `claim_winnings` twice must also receive `AlreadyClaimed`
/// on the second attempt.  The claimed flag is set regardless of payout amount.
#[test]
#[should_panic(expected = "Error(Contract, #60)")]
fn test_loser_double_claim_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let loser = Address::generate(&env);
    let winner = Address::generate(&env);
    ctx.token_admin.mint(&loser, &300);
    ctx.token_admin.mint(&winner, &700);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &loser,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&loser, &pool_id, &300, &0, &None, &None);
    ctx.client
        .place_prediction(&winner, &pool_id, &700, &1, &None, &None);

    resolve_at(&ctx, &env, pool_id, end_time, 1);

    // First claim — loser gets 0.
    let w1 = ctx.client.claim_winnings(&loser, &pool_id);
    assert_eq!(w1, 0, "loser first claim must be 0");

    // Second claim — must panic with AlreadyClaimed (#60).
    ctx.client.claim_winnings(&loser, &pool_id);
}

/// After a double-claim attempt the contract balance must remain correct.
/// Specifically, the winner's second call must not transfer extra tokens.
///
/// This test uses `try_claim_winnings` (the non-panicking variant) to inspect
/// the error without unwinding, then asserts the contract balance is unchanged.
#[test]
fn test_double_claim_does_not_drain_extra_funds() {
    let env = Env::default();
    let ctx = setup(&env);

    let winner = Address::generate(&env);
    let loser = Address::generate(&env);
    ctx.token_admin.mint(&winner, &500);
    ctx.token_admin.mint(&loser, &500);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &winner,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &two_outcome_config(&env),
    );

    ctx.client
        .place_prediction(&winner, &pool_id, &500, &1, &None, &None);
    ctx.client
        .place_prediction(&loser, &pool_id, &500, &0, &None, &None);

    resolve_at(&ctx, &env, pool_id, end_time, 1);

    // Legitimate first claim — winner takes the full pot.
    let w1 = ctx.client.claim_winnings(&winner, &pool_id);
    assert_eq!(w1, 1_000);

    // Loser claims (0) — sets their claimed flag.
    let wl = ctx.client.claim_winnings(&loser, &pool_id);
    assert_eq!(wl, 0);

    let balance_after_valid_claims = ctx.token.balance(&ctx.client.address);

    // Attempt a second claim from the winner — must fail with AlreadyClaimed.
    let result = ctx.client.try_claim_winnings(&winner, &pool_id);
    assert!(
        result.is_err(),
        "second claim must return an error"
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        PredifiError::AlreadyClaimed,
        "error must be AlreadyClaimed"
    );

    // Contract balance must not have changed.
    assert_eq!(
        ctx.token.balance(&ctx.client.address),
        balance_after_valid_claims,
        "double-claim must not alter contract balance"
    );

    // Winner balance must remain at the value set after the first (valid) claim.
    assert_eq!(ctx.token.balance(&winner), 1_000);
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. ALL PARTICIPANTS CHOSE THE SAME OUTCOME
// ═══════════════════════════════════════════════════════════════════════════

/// Every participant staked on outcome 1 and the pool resolves to outcome 1.
///
/// When `winning_stake == total_stake` the proportional payout collapses to
/// each user recovering exactly their own stake (with zero fee).
///
/// Key invariants checked:
/// - INV-1: `total_stake == sum(individual_stakes)`
/// - INV-4: `Σ(winnings) ≤ total_stake`
/// - INV-5: `Σ(claimed_winnings) == total_stake` (all funds distributed)
/// - Contract balance drains to exactly 0.
#[test]
fn test_all_participants_same_winning_outcome_each_gets_stake_back() {
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
        &two_outcome_config(&env),
    );

    // Five participants all bet on outcome 1.
    let stakes = [100i128, 200, 300, 400, 500];
    let total_stake: i128 = stakes.iter().sum(); // 1_500

    let mut bettors: alloc::vec::Vec<Address> = alloc::vec::Vec::new();
    for &amount in &stakes {
        let user = Address::generate(&env);
        ctx.token_admin.mint(&user, &amount);
        ctx.client
            .place_prediction(&user, &pool_id, &amount, &1, &None, &None);
        bettors.push(user);
    }

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.total_stake, total_stake, "INV-1: total_stake mismatch");

    resolve_at(&ctx, &env, pool_id, end_time, 1);

    // Each winner gets back their exact stake (winning_stake == total_stake → ratio = 1).
    let mut total_paid_out: i128 = 0;
    for (i, user) in bettors.iter().enumerate() {
        let winnings = ctx.client.claim_winnings(user, &pool_id);
        assert_eq!(
            winnings, stakes[i],
            "user {i} expected {} but received {}",
            stakes[i], winnings
        );
        total_paid_out += winnings;
    }

    // INV-5: all stake is returned.
    assert_eq!(
        total_paid_out, total_stake,
        "INV-5: total paid out must equal total staked"
    );
    assert_eq!(
        ctx.token.balance(&ctx.client.address),
        0,
        "contract must hold zero residual balance"
    );
}

/// All participants stake identical amounts on the same winning outcome.
/// Ensures there are no rounding artifacts when shares are perfectly equal.
#[test]
fn test_all_equal_stakes_same_outcome_no_rounding_loss() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &two_outcome_config(&env),
    );

    let n = 8usize;
    let stake_each = 250i128;
    let total_stake = stake_each * n as i128; // 2_000

    let mut bettors: alloc::vec::Vec<Address> = alloc::vec::Vec::new();
    for _ in 0..n {
        let user = Address::generate(&env);
        ctx.token_admin.mint(&user, &stake_each);
        ctx.client
            .place_prediction(&user, &pool_id, &stake_each, &0, &None, &None);
        bettors.push(user);
    }

    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, total_stake);

    resolve_at(&ctx, &env, pool_id, end_time, 0);

    let mut total_paid: i128 = 0;
    for user in &bettors {
        let w = ctx.client.claim_winnings(user, &pool_id);
        // Each user holds exactly 1/n of the winning stake, so gets back stake_each.
        assert_eq!(w, stake_each, "equal-stake winner should recover exact stake");
        total_paid += w;
    }

    assert_eq!(total_paid, total_stake, "all funds must be distributed");
    assert_eq!(ctx.token.balance(&ctx.client.address), 0);
}

/// All participants stake on the same outcome but the operator resolves to the
/// *other* outcome (the one nobody bet on).
///
/// Because `winning_stake == 0`, every claimant receives 0 — no one can
/// extract value from an unbet outcome, and the contract retains all funds.
#[test]
fn test_all_same_outcome_but_resolves_to_other_side_everyone_gets_zero() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &two_outcome_config(&env),
    );

    let stakes = [500i128, 300, 200];
    let total_stake: i128 = stakes.iter().sum(); // 1_000

    let mut bettors: alloc::vec::Vec<Address> = alloc::vec::Vec::new();
    for &amount in &stakes {
        let user = Address::generate(&env);
        ctx.token_admin.mint(&user, &amount);
        // All bet on outcome 0.
        ctx.client
            .place_prediction(&user, &pool_id, &amount, &0, &None, &None);
        bettors.push(user);
    }

    // Resolve to outcome 1 — which has zero stake.
    resolve_at(&ctx, &env, pool_id, end_time, 1);

    let mut total_paid: i128 = 0;
    for user in &bettors {
        let w = ctx.client.claim_winnings(user, &pool_id);
        assert_eq!(w, 0, "all participants must receive 0 (zero winning stake)");
        total_paid += w;
    }

    assert_eq!(total_paid, 0, "no funds should be distributed");
    // All funds remain in the contract.
    assert_eq!(ctx.token.balance(&ctx.client.address), total_stake);
}

/// All participants stake on the same outcome with a non-zero protocol fee.
/// Each winner's payout should be `stake * (1 - fee_bps/10000)` because
/// `winning_stake == total_stake` simplifies the formula to a straight
/// fee deduction.
#[test]
fn test_all_same_outcome_with_fee_payout_reduced_proportionally() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.protocol_version = 23;
        li.timestamp = 1_000;
    });

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);

    let ac_id = env.register(dummy_ac::DummyAC, ());
    let ac_client = dummy_ac::DummyACClient::new(&env, &ac_id);
    ac_client.grant_role(&admin, &0u32);
    ac_client.grant_role(&operator, &1u32);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);
    // 10% protocol fee (1_000 bps).
    client.init(&ac_id, &treasury, &1_000u32, &0u64, &3600u64, &0u32);

    let token_deployer = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_deployer);
    let token_address = token_contract.address();
    let token = token::Client::new(&env, &token_address);
    let token_admin = token::StellarAssetClient::new(&env, &token_address);
    client.add_token_to_whitelist(&admin, &token_address);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Fee same-outcome pool"),
            metadata_url: String::from_str(&env, "ipfs://fee-test"),
            min_stake: 1i128,
            max_stake: 0i128,
            min_total_stake: 0i128,
            max_total_stake: 0i128,
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

    // Two users stake equal amounts on outcome 1.
    let stake = 1_000i128;
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    token_admin.mint(&user_a, &stake);
    token_admin.mint(&user_b, &stake);
    client.place_prediction(&user_a, &pool_id, &stake, &1, &None, &None);
    client.place_prediction(&user_b, &pool_id, &stake, &1, &None, &None);

    let total_stake = 2 * stake; // 2_000
    env.ledger().with_mut(|li| li.timestamp = end_time);
    client.resolve_pool(&operator, &pool_id, &1u32);

    // fee_bps is locked in pool at resolution time.
    // protocol_fee = floor(2000 * 1000 / 10000) = 200  (ProtocolFavor rounding)
    // payout_pool = 2000 - 200 = 1800
    // each user's share = (1000 / 2000) * 1800 = 900
    let expected_each = 900i128;

    let wa = client.claim_winnings(&user_a, &pool_id);
    let wb = client.claim_winnings(&user_b, &pool_id);

    assert_eq!(wa, expected_each, "user_a payout should be 900 after 10% fee");
    assert_eq!(wb, expected_each, "user_b payout should be 900 after 10% fee");
    assert_eq!(
        wa + wb,
        2 * expected_each,
        "total payout must equal payout_pool"
    );

    // Treasury keeps the fee; contract should hold only the residual.
    let contract_balance = token.balance(&contract_id);
    // Due to integer rounding the contract may hold 0 or 1 stroop.
    assert!(
        contract_balance <= 1,
        "contract should hold at most 1 stroop residual, got {contract_balance}"
    );
    let _ = total_stake; // suppress unused warning
}

extern crate alloc;
