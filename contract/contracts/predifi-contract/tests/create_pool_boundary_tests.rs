//! Advanced boundary & edge-case integration tests for `create_pool`.
//!
//! These tests exercise the entry point through the real
//! [`PredifiContractClient`] (not the in-crate `TestEnv`), so they cover the
//! same validation ladder an on-chain caller hits. They focus on four groups
//! called out by the issue:
//!
//! 1. **Timestamps** — zero duration, `end_time == now`, `u64::MAX`, and the
//!    exact `MAX_POOL_DURATION` boundary.
//! 2. **Descriptions** — empty, exactly 256 bytes, and 257 bytes.
//! 3. **Identity / token** — duplicate descriptions (allowed, distinct ids) and
//!    a token that is not on the betting whitelist.
//! 4. **Fees** — pool creation with `fee_bps` at `0` and `10_000`, the fee math
//!    at both extremes, and rejection of `10_001`.
//!
//! Every rejection also asserts the **typed** error where the contract emits
//! one, and that **no partial state is persisted** (active-pool index stays
//! empty and the pool-id counter does not advance).

#![cfg(test)]

mod common;

use common::{setup, setup_with_fee, test_env, Harness};
use predifi_contract::{
    calculate_protocol_fee, PoolConfig, PredifiError, MAX_POOL_DURATION,
};
use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env, String};

// ── helpers ─────────────────────────────────────────────────────────────────

/// A minimal valid pool configuration with exactly two outcomes.
fn config(env: &Env) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "Boundary pool"),
        metadata_url: String::from_str(env, "https://predifi.example/pool"),
        min_stake: 1,
        max_stake: 0,
        min_total_stake: 1,
        max_total_stake: 0,
        initial_liquidity: 0,
        required_resolutions: 1,
        private: false,
        whitelist_key: None,
        outcome_descriptions: vec![
            env,
            String::from_str(env, "Yes"),
            String::from_str(env, "No"),
        ],
    }
}

fn create_pool(env: &Env, h: &Harness<'_>, end_time: u64) -> u64 {
    h.client.create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config(env),
    )
}

/// A rejected `create_pool` must not advance the active-pool index, and the
/// next successful creation must still receive pool id `0`.
fn assert_no_partial_state(env: &Env, h: &Harness<'_>) {
    assert_eq!(
        h.client.get_active_pools_count(),
        0,
        "rejected create_pool must leave the active-pool index empty"
    );
    let id = create_pool(env, h, env.ledger().timestamp() + 1_000);
    assert_eq!(id, 0, "pool-id counter must not advance on a rejected create");
}

// ── 1. timestamp boundaries ─────────────────────────────────────────────────

/// `end_time == start_time` (zero-length window) is rejected with the typed
/// `InvalidTimestamp` error.
#[test]
fn zero_duration_rejected_with_invalid_timestamp() {
    let env = test_env();
    let h = setup(&env);
    let now = env.ledger().timestamp();

    let mut cfg = config(&env);
    cfg.start_time = now;

    let result = h.client.try_create_pool(
        &h.creator,
        &now,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &cfg,
    );

    assert_eq!(result.unwrap_err().unwrap(), PredifiError::InvalidTimestamp);
    assert_no_partial_state(&env, &h);
}

/// `end_time == now` with the default (open-immediately) `start_time` is not
/// strictly in the future, so the contract aborts before persisting anything.
///
/// The contract intentionally raises the legacy diagnostic `assert!` here
/// rather than a typed error (kept for backwards-compatible test
/// expectations), so this test pins a clean revert + no partial state instead
/// of an error enum.
#[test]
fn end_time_equal_to_now_reverts_without_persisting() {
    let env = test_env();
    let h = setup(&env);
    let now = env.ledger().timestamp();

    let result =
        h.client
            .try_create_pool(&h.creator, &now, &h.token_address, &2u32, &symbol_short!("Crypto"), &config(&env));

    assert!(result.is_err(), "end_time == now must be rejected");
    assert_no_partial_state(&env, &h);
}

/// `end_time = u64::MAX` far exceeds `MAX_POOL_DURATION` from now and is
/// rejected with `InvalidTimestamp`.
#[test]
fn max_u64_end_time_rejected_with_invalid_timestamp() {
    let env = test_env();
    let h = setup(&env);

    let result = h.client.try_create_pool(
        &h.creator,
        &u64::MAX,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config(&env),
    );

    assert_eq!(result.unwrap_err().unwrap(), PredifiError::InvalidTimestamp);
    assert_no_partial_state(&env, &h);
}

/// `end_time == now + MAX_POOL_DURATION` is the largest accepted value.
#[test]
fn exact_max_pool_duration_boundary_is_accepted() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + MAX_POOL_DURATION;

    let pool_id = create_pool(&env, &h, end_time);

    assert_eq!(h.client.get_pool(&pool_id).end_time, end_time);
    assert_eq!(h.client.get_active_pools_count(), 1);
}

/// One second past the maximum duration is rejected with `InvalidTimestamp`.
#[test]
fn one_second_beyond_max_pool_duration_is_rejected() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + MAX_POOL_DURATION + 1;

    let result = h.client.try_create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config(&env),
    );

    assert_eq!(result.unwrap_err().unwrap(), PredifiError::InvalidTimestamp);
    assert_no_partial_state(&env, &h);
}

// ── 2. description boundaries ───────────────────────────────────────────────

/// An empty description is accepted — only the *upper* bound (256 bytes) is
/// enforced by `create_pool`.
#[test]
fn empty_description_is_accepted() {
    let env = test_env();
    let h = setup(&env);

    let mut cfg = config(&env);
    cfg.description = String::from_str(&env, "");

    let pool_id = h.client.create_pool(
        &h.creator,
        &(env.ledger().timestamp() + 1_000),
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &cfg,
    );

    assert_eq!(h.client.get_pool(&pool_id).description.len(), 0);
}

/// A description of exactly 256 bytes is the accepted upper boundary.
#[test]
fn description_of_exactly_256_bytes_is_accepted() {
    let env = test_env();
    let h = setup(&env);

    let mut cfg = config(&env);
    cfg.description = String::from_str(&env, &"a".repeat(256));

    let pool_id = h.client.create_pool(
        &h.creator,
        &(env.ledger().timestamp() + 1_000),
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &cfg,
    );

    assert_eq!(h.client.get_pool(&pool_id).description.len(), 256);
}

/// A 257-byte description trips the contract's `assert!` guard and must revert
/// without persisting a pool.
#[test]
fn description_of_257_bytes_reverts_without_persisting() {
    let env = test_env();
    let h = setup(&env);

    let mut cfg = config(&env);
    cfg.description = String::from_str(&env, &"a".repeat(257));

    let result = h.client.try_create_pool(
        &h.creator,
        &(env.ledger().timestamp() + 1_000),
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &cfg,
    );

    assert!(result.is_err(), "description > 256 bytes must be rejected");
    assert_no_partial_state(&env, &h);
}

// ── 3. identity & token ─────────────────────────────────────────────────────

/// Pools are identified by their numeric id, not by description, so two pools
/// may share an identical description and still receive distinct ids.
#[test]
fn duplicate_descriptions_create_distinct_pools() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;

    let id_a = create_pool(&env, &h, end_time);
    let id_b = create_pool(&env, &h, end_time);

    assert_ne!(id_a, id_b, "duplicate descriptions must get distinct ids");
    assert_eq!(h.client.get_active_pools_count(), 2);
    assert_eq!(
        h.client.get_pool(&id_a).description,
        h.client.get_pool(&id_b).description
    );
}

/// A token that is not on the betting whitelist is rejected with the typed
/// `TokenNotWhitelisted` error. Soroban has no zero-address sentinel, so a
/// freshly generated (never whitelisted) address is the equivalent "invalid"
/// token identity.
#[test]
fn non_whitelisted_token_rejected() {
    let env = test_env();
    let h = setup(&env);
    let bogus_token = Address::generate(&env);

    let result = h.client.try_create_pool(
        &h.creator,
        &(env.ledger().timestamp() + 1_000),
        &bogus_token,
        &2u32,
        &symbol_short!("Crypto"),
        &config(&env),
    );

    assert_eq!(
        result.unwrap_err().unwrap(),
        PredifiError::TokenNotWhitelisted
    );
    assert_no_partial_state(&env, &h);
}

/// The contract's own address is never on the token whitelist and must be
/// rejected the same way.
#[test]
fn contract_address_as_token_rejected() {
    let env = test_env();
    let h = setup(&env);

    let result = h.client.try_create_pool(
        &h.creator,
        &(env.ledger().timestamp() + 1_000),
        &h.contract_id,
        &2u32,
        &symbol_short!("Crypto"),
        &config(&env),
    );

    assert_eq!(
        result.unwrap_err().unwrap(),
        PredifiError::TokenNotWhitelisted
    );
    assert_no_partial_state(&env, &h);
}

// ── 4. fee boundaries ───────────────────────────────────────────────────────

/// With a `0` bps fee the protocol takes nothing, so the payout pool is the
/// full stake.
#[test]
fn zero_fee_bps_takes_nothing() {
    let env = test_env();
    let h = setup_with_fee(&env, 0);

    let fee = calculate_protocol_fee(1_000, 0).unwrap();
    assert_eq!(fee, 0);
    assert_eq!(1_000 - fee, 1_000, "payout pool keeps the full stake");

    // Pool creation still works at the 0 bps extreme.
    let pool_id = create_pool(&env, &h, env.ledger().timestamp() + 1_000);
    assert_eq!(h.client.get_active_pools_count(), 1);
    assert_eq!(h.client.get_pool(&pool_id).fee_bps, 0);
    assert_eq!(h.client.get_pool(&pool_id).options_count, 2);
}

/// With a `10_000` bps (100%) fee the protocol takes the entire stake, leaving
/// an empty payout pool.
#[test]
fn full_fee_bps_takes_everything() {
    let env = test_env();
    let h = setup_with_fee(&env, 10_000);

    let fee = calculate_protocol_fee(1_000, 10_000).unwrap();
    assert_eq!(fee, 1_000);
    assert_eq!(1_000 - fee, 0, "payout pool is empty at 100% fee");

    // Pool creation still works at the 10_000 bps extreme.
    let pool_id = create_pool(&env, &h, env.ledger().timestamp() + 1_000);
    assert_eq!(h.client.get_active_pools_count(), 1);
    assert_eq!(h.client.get_pool(&pool_id).fee_bps, 10_000);
    assert_eq!(h.client.get_pool(&pool_id).options_count, 2);
}

/// `set_fee_bps` accepts the `10_000` boundary.
#[test]
fn set_fee_bps_accepts_upper_boundary() {
    let env = test_env();
    let h = setup(&env);

    assert!(h.client.try_set_fee_bps(&h.admin, &10_000u32).is_ok());
}

/// `set_fee_bps` rejects `10_001` with the typed `InvalidFeeBps` error.
#[test]
fn set_fee_bps_rejects_above_upper_boundary() {
    let env = test_env();
    let h = setup(&env);

    let result = h.client.try_set_fee_bps(&h.admin, &10_001u32);

    assert_eq!(result.unwrap_err().unwrap(), PredifiError::InvalidFeeBps);
}

/// `calculate_protocol_fee` rejects a fee above 10_000 bps rather than
/// silently over-charging.
#[test]
fn calculate_protocol_fee_rejects_fee_above_10000_bps() {
    use predifi_errors::PrediFiError;

    assert_eq!(
        calculate_protocol_fee(1_000, 10_001),
        Err(PrediFiError::InvalidFeeBps)
    );
}
