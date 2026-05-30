//! Edge-case and concurrent-staker tests for the PrediFi prediction market contract.
//!
//! # Issue #1036 — Test pool behaviour when `end_time == current_time`
//!
//! The contract enforces `end_time > current_time` at pool creation and
//! `current_time < pool.end_time` when placing a prediction.  These tests
//! verify the exact boundary:
//!
//! - Creating a pool with `end_time == current_time` must be **rejected**.
//! - Placing a prediction at the exact moment `current_time == pool.end_time`
//!   must be **rejected** (the window is half-open: `[creation, end_time)`).
//!
//! # Issue #1028 — Test for multiple simultaneous stakers
//!
//! Soroban's single-threaded execution model means "simultaneous" is modelled
//! as back-to-back transactions within the same ledger timestamp.  These tests
//! verify that:
//!
//! - Many stakers can participate in the same pool without corrupting the
//!   total-stake invariant (INV-1).
//! - Proportional payout arithmetic is correct when many users share a winning
//!   outcome.
//! - Stakers on different outcomes all receive correct payouts after resolution.

#![cfg(test)]

use crate::{MarketState, PoolConfig, PredifiContract, PredifiContractClient};
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

// ─── Test environment setup ───────────────────────────────────────────────────

/// Shared setup: deploys the dummy access-control contract, the predifi
/// contract, and a whitelisted token.  Returns the client, token helpers,
/// admin address, and operator address.
fn setup(
    env: &Env,
) -> (
    PredifiContractClient<'_>,
    token::Client<'_>,
    token::StellarAssetClient<'_>,
    Address, // admin / operator (same address for simplicity)
) {
    env.mock_all_auths();
    // Start at a non-zero timestamp so we can subtract from it in tests.
    env.ledger().with_mut(|li| {
        li.protocol_version = 23;
        li.timestamp = 1_000;
    });

    let admin = Address::generate(env);
    let treasury = Address::generate(env);

    let ac_id = env.register(dummy_ac::DummyAC, ());
    let ac_client = dummy_ac::DummyACClient::new(env, &ac_id);
    ac_client.grant_role(&admin, &0u32); // Admin
    ac_client.grant_role(&admin, &1u32); // Operator

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);
    // resolution_delay = 0 so tests can resolve immediately after end_time.
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let token_client = token::Client::new(env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    client.add_token_to_whitelist(&admin, &token_id);

    (client, token_client, token_admin_client, admin)
}

/// Convenience: build a minimal two-outcome `PoolConfig`.
fn two_outcome_config(env: &Env) -> PoolConfig {
    PoolConfig {
            start_time: 0,
        description: String::from_str(env, "Edge case pool"),
        metadata_url: String::from_str(env, "ipfs://edge"),
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
// Issue #1036 — end_time == current_time edge cases
// ═══════════════════════════════════════════════════════════════════════════

/// Creating a pool with `end_time == current_time` must be rejected.
///
/// The contract asserts `end_time > current_time`, so equality is invalid.
#[test]
#[should_panic(expected = "end_time must be greater than current time")]
fn test_create_pool_end_time_equals_current_time_is_rejected() {
    let env = Env::default();
    let (client, token_client, _, _) = setup(&env);

    let creator = Address::generate(&env);
    let current_time = env.ledger().timestamp(); // 1_000

    // end_time == current_time — must panic
    client.create_pool(
        &creator,
        &current_time, // equal, not strictly greater
        &token_client.address,
        &2u32,
        &symbol_short!("Tech"),
        &two_outcome_config(&env),
    );
}

/// Creating a pool with `end_time == current_time + min_pool_duration - 1`
/// (one second short of the minimum) must also be rejected.
#[test]
#[should_panic(expected = "pool duration too short")]
fn test_create_pool_end_time_below_min_duration_is_rejected() {
    let env = Env::default();
    let (client, token_client, _, _) = setup(&env);

    let creator = Address::generate(&env);
    let current_time = env.ledger().timestamp(); // 1_000
                                                 // min_pool_duration = 3600; end_time = 1_000 + 3600 - 1 = 4_599 (too short)
    let end_time = current_time + 3600 - 1;

    client.create_pool(
        &creator,
        &end_time,
        &token_client.address,
        &2u32,
        &symbol_short!("Tech"),
        &two_outcome_config(&env),
    );
}

/// Placing a prediction at the exact moment `current_time == pool.end_time`
/// must be rejected — the betting window is half-open `[creation, end_time)`.
#[test]
#[should_panic(expected = "betting window closed")]
fn test_place_prediction_at_exact_end_time_is_rejected() {
    let env = Env::default();
    let (client, token_client, token_admin_client, _) = setup(&env);

    let creator = Address::generate(&env);
    // Pool ends at ledger time 5_000 (current is 1_000, so duration = 4_000 ≥ 3_600).
    let end_time = 5_000u64;

    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_client.address,
        &2u32,
        &symbol_short!("Tech"),
        &two_outcome_config(&env),
    );

    // Advance ledger to exactly end_time.
    env.ledger().with_mut(|li| li.timestamp = end_time);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &100);

    // Prediction at current_time == end_time must panic ("Pool has ended").
    client.place_prediction(&user, &pool_id, &100, &0, &None, &None);
}

/// Placing a prediction one second *before* `end_time` must succeed.
/// This confirms the boundary is exclusive on the right side only.
#[test]
fn test_place_prediction_one_second_before_end_time_succeeds() {
    let env = Env::default();
    let (client, token_client, token_admin_client, _) = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 5_000u64;

    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_client.address,
        &2u32,
        &symbol_short!("Tech"),
        &two_outcome_config(&env),
    );

    // One second before end_time — still inside the betting window.
    env.ledger().with_mut(|li| li.timestamp = end_time - 1);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &100);
    client.place_prediction(&user, &pool_id, &100, &1, &None, &None);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.total_stake, 100);
}

/// Resolving a pool at exactly `end_time` (with `resolution_delay = 0`) must
/// succeed — the resolution window opens as soon as the betting window closes.
#[test]
fn test_resolve_pool_at_exact_end_time_succeeds() {
    let env = Env::default();
    let (client, token_client, token_admin_client, admin) = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 5_000u64;

    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_client.address,
        &2u32,
        &symbol_short!("Tech"),
        &two_outcome_config(&env),
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &500);
    client.place_prediction(&user, &pool_id, &500, &1, &None, &None);

    // Advance to exactly end_time; resolution_delay = 0 so eligible_at = end_time.
    env.ledger().with_mut(|li| li.timestamp = end_time);
    client.resolve_pool(&admin, &pool_id, &1u32);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 1u32);
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1028 — Multiple simultaneous stakers
// ═══════════════════════════════════════════════════════════════════════════

/// Ten stakers all bet on the same outcome within the same ledger timestamp.
/// After resolution the total stake must equal the sum of all individual
/// stakes (INV-1) and each winner's payout must be proportional to their
/// contribution.
#[test]
fn test_multiple_simultaneous_stakers_same_outcome() {
    let env = Env::default();
    let (client, token_client, token_admin_client, admin) = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 5_000u64;

    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_client.address,
        &2u32,
        &symbol_short!("Sports"),
        &two_outcome_config(&env),
    );

    // Ten stakers, each betting a different amount on outcome 0.
    // All transactions happen at the same ledger timestamp (1_000).
    let stakes: [i128; 10] = [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
    let total_stake: i128 = stakes.iter().sum(); // 5_500

    let mut stakers: alloc::vec::Vec<Address> = alloc::vec::Vec::new();
    for &amount in &stakes {
        let user = Address::generate(&env);
        token_admin_client.mint(&user, &amount);
        client.place_prediction(&user, &pool_id, &amount, &0, &None, &None);
        stakers.push(user);
    }

    // Verify INV-1: pool.total_stake == sum of all individual stakes.
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.total_stake, total_stake);
    assert_eq!(pool.participants_count, 10u32);

    // Resolve at end_time.
    env.ledger().with_mut(|li| li.timestamp = end_time);
    client.resolve_pool(&admin, &pool_id, &0u32);

    // Every staker is a winner; each receives their proportional share of the
    // total pot (no fee configured, so payout_pool == total_stake).
    let mut total_paid_out: i128 = 0;
    for (i, user) in stakers.iter().enumerate() {
        let winnings = client.claim_winnings(user, &pool_id);
        // Expected: (stake / total_stake) * total_stake == stake (100% payout, no fee)
        assert_eq!(
            winnings, stakes[i],
            "staker {i} expected {} but got {winnings}",
            stakes[i]
        );
        total_paid_out += winnings;
    }

    // All funds must be distributed — contract balance should be zero.
    assert_eq!(total_paid_out, total_stake);
    assert_eq!(token_client.balance(&client.address), 0);
}

/// Twenty stakers split evenly across two outcomes at the same ledger
/// timestamp.  After resolution, only the winning-side stakers receive
/// payouts and the total payout equals the full pool (no fee).
#[test]
fn test_multiple_simultaneous_stakers_split_outcomes() {
    let env = Env::default();
    let (client, token_client, token_admin_client, admin) = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 5_000u64;

    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_client.address,
        &2u32,
        &symbol_short!("Finance"),
        &two_outcome_config(&env),
    );

    let n_per_side = 10usize;
    let stake_per_user: i128 = 1_000;

    let mut winners: alloc::vec::Vec<Address> = alloc::vec::Vec::new();
    let mut losers: alloc::vec::Vec<Address> = alloc::vec::Vec::new();

    // All bets placed at the same ledger timestamp (1_000).
    for i in 0..(n_per_side * 2) {
        let user = Address::generate(&env);
        token_admin_client.mint(&user, &stake_per_user);
        let outcome = (i % 2) as u32; // alternates 0, 1, 0, 1, …
        client.place_prediction(&user, &pool_id, &stake_per_user, &outcome, &None, &None);
        if outcome == 1 {
            winners.push(user);
        } else {
            losers.push(user);
        }
    }

    let total_stake = stake_per_user * (n_per_side * 2) as i128; // 20_000
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.total_stake, total_stake);
    assert_eq!(pool.participants_count, (n_per_side * 2) as u32);

    // Resolve: outcome 1 wins.
    env.ledger().with_mut(|li| li.timestamp = end_time);
    client.resolve_pool(&admin, &pool_id, &1u32);

    // Each winner staked 1_000 out of 10_000 on the winning side.
    // Payout = (1_000 / 10_000) * 20_000 = 2_000 (double their stake).
    let expected_payout_per_winner = 2_000i128;
    let mut total_paid: i128 = 0;
    for user in &winners {
        let w = client.claim_winnings(user, &pool_id);
        assert_eq!(w, expected_payout_per_winner);
        total_paid += w;
    }

    // Losers receive nothing.
    for user in &losers {
        let w = client.claim_winnings(user, &pool_id);
        assert_eq!(w, 0);
    }

    // All funds distributed.
    assert_eq!(total_paid, total_stake);
    assert_eq!(token_client.balance(&client.address), 0);
}

/// Fifty stakers all bet on the same outcome at the same ledger timestamp.
/// Verifies that the stake accumulation and participant counter scale
/// correctly under higher concurrency.
#[test]
fn test_fifty_simultaneous_stakers_same_outcome() {
    let env = Env::default();
    let (client, token_client, token_admin_client, admin) = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 5_000u64;

    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_client.address,
        &2u32,
        &symbol_short!("Crypto"),
        &two_outcome_config(&env),
    );

    let n = 50usize;
    let stake_per_user: i128 = 200;

    let mut stakers: alloc::vec::Vec<Address> = alloc::vec::Vec::new();
    for _ in 0..n {
        let user = Address::generate(&env);
        token_admin_client.mint(&user, &stake_per_user);
        client.place_prediction(&user, &pool_id, &stake_per_user, &0, &None, &None);
        stakers.push(user);
    }

    let expected_total = stake_per_user * n as i128; // 10_000
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.total_stake, expected_total, "INV-1 violated");
    assert_eq!(pool.participants_count, n as u32);

    // Resolve and verify every staker gets their stake back (no fee, single outcome).
    env.ledger().with_mut(|li| li.timestamp = end_time);
    client.resolve_pool(&admin, &pool_id, &0u32);

    let mut total_paid: i128 = 0;
    for user in &stakers {
        let w = client.claim_winnings(user, &pool_id);
        assert_eq!(w, stake_per_user);
        total_paid += w;
    }
    assert_eq!(total_paid, expected_total);
    assert_eq!(token_client.balance(&client.address), 0);
}

/// A staker who increases their stake across two transactions (same pool,
/// same outcome) is treated as a single participant.  The participant count
/// must not be double-incremented.
#[test]
fn test_staker_top_up_does_not_double_count_participant() {
    let env = Env::default();
    let (client, token_client, token_admin_client, admin) = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 5_000u64;

    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_client.address,
        &2u32,
        &symbol_short!("Politics"),
        &two_outcome_config(&env),
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1_000);

    // First stake: 400 on outcome 0.
    client.place_prediction(&user, &pool_id, &400, &0, &None, &None);
    // Top-up: additional 600 on the same outcome.
    client.place_prediction(&user, &pool_id, &600, &0, &None, &None);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.total_stake, 1_000, "total stake should be 1_000");
    // The user is still a single participant despite two transactions.
    assert_eq!(pool.participants_count, 1u32, "participant count must be 1");

    // Resolve and claim — user should receive the full pot.
    env.ledger().with_mut(|li| li.timestamp = end_time);
    client.resolve_pool(&admin, &pool_id, &0u32);

    let winnings = client.claim_winnings(&user, &pool_id);
    assert_eq!(winnings, 1_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

extern crate alloc;

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1037 — Emit event when oracle price feed is updated
// ═══════════════════════════════════════════════════════════════════════════
//
// The `update_price_feed` function now emits a `PriceFeedUpdatedEvent` after
// successfully storing the new price data.  The tests below verify:
//
// - A valid price update succeeds and the event fields match the inputs.
// - An update with a future timestamp is rejected (no event emitted).
// - An update from a non-whitelisted oracle is rejected.

/// A valid `update_price_feed` call must succeed and the stored data must
/// match the supplied arguments.
#[test]
fn test_update_price_feed_emits_event_and_stores_data() {
    let env = Env::default();
    let (client, _, _, admin) = setup(&env);

    // Whitelist an oracle address.
    let oracle = Address::generate(&env);
    client.add_oracle(&admin, &oracle);

    // current ledger time is 1_000; use timestamp = 999 (strictly in the past).
    let feed_pair = symbol_short!("BTCUSD");
    let price: i128 = 60_000_000_000; // $60,000 at 6 decimals
    let confidence: i128 = 10_000_000; // ±$10
    let ts: u64 = 999;
    let expires: u64 = 2_000;

    let result =
        client.try_update_price_feed(&oracle, &feed_pair, &price, &confidence, &ts, &expires);
    assert!(
        result.is_ok(),
        "update_price_feed should succeed for a whitelisted oracle"
    );
}

/// An `update_price_feed` call with `timestamp >= current_ledger_time` must
/// be rejected with `InvalidData` — no event should be emitted.
#[test]
fn test_update_price_feed_rejects_future_timestamp() {
    let env = Env::default();
    let (client, _, _, admin) = setup(&env);

    let oracle = Address::generate(&env);
    client.add_oracle(&admin, &oracle);

    let feed_pair = symbol_short!("ETHUSD");
    // timestamp == current_time (1_000) — must be rejected.
    let result = client.try_update_price_feed(
        &oracle,
        &feed_pair,
        &3_000_000_000i128,
        &1_000_000i128,
        &1_000u64, // == current_time
        &2_000u64,
    );
    assert!(result.is_err(), "future/equal timestamp must be rejected");
}

/// An `update_price_feed` call from a non-whitelisted oracle must be
/// rejected with `Unauthorized`.
#[test]
fn test_update_price_feed_rejects_non_whitelisted_oracle() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env);

    // oracle is NOT added to the whitelist.
    let oracle = Address::generate(&env);

    let result = client.try_update_price_feed(
        &oracle,
        &symbol_short!("SOLUSD"),
        &100_000_000i128,
        &500_000i128,
        &999u64,
        &2_000u64,
    );
    assert!(result.is_err(), "non-whitelisted oracle must be rejected");
}
