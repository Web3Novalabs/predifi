//! Event assertion integration tests for `predifi-contract`.
//!
//! Every state-changing entry point that publishes an event is exercised through
//! the real [`PredifiContractClient`], and the emitted event is asserted on
//! **both** its exact topic (the value in `topics[0]`) and its exact payload
//! field structure. This guards the contract's indexer-facing contract: backend
//! indexers depend on these topics and payload shapes remaining stable.
//!
//! > NOTE: `Env::events().all()` only reflects the events published by the most
//! > recent top-level contract invocation (any subsequent client call clears the
//! > buffer). Each test therefore inspects events **immediately** after the
//! > emitting entry point, before making any other contract call.

#![cfg(test)]

mod common;

use common::{event_count, event_data, field, setup, test_env, Harness};
use predifi_contract::PoolConfig;
use soroban_sdk::{
    symbol_short, testutils::Address as _, vec, Address, Env, Map, String, Symbol, TryFromVal, Val,
};

// ── helpers ─────────────────────────────────────────────────────────────────

/// Decode a named payload field into a concrete Soroban type.
fn decode<T>(env: &Env, data: &Map<Symbol, Val>, name: &str) -> T
where
    T: TryFromVal<Env, Val>,
{
    T::try_from_val(env, &field(env, data, name)).expect("event field type mismatch")
}

/// A minimal valid pool configuration with exactly two outcomes.
fn base_config(env: &Env) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "Event assertion pool"),
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
        &base_config(env),
    )
}

// ── create_pool ─────────────────────────────────────────────────────────────

#[test]
fn create_pool_emits_pool_created_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;

    let pool_id = create_pool(&env, &h, end_time);

    assert_eq!(event_count(&env, "pool_created"), 1);
    let data = event_data(&env, "pool_created").expect("pool_created event missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "creator"), h.creator);
    assert_eq!(decode::<u64>(&env, &data, "end_time"), end_time);
    assert_eq!(decode::<Address>(&env, &data, "token"), h.token_address);
    assert_eq!(decode::<u32>(&env, &data, "options_count"), 2);
    assert_eq!(decode::<u32>(&env, &data, "required_resolutions"), 1);
    assert_eq!(decode::<i128>(&env, &data, "max_total_stake"), 0);
    assert_eq!(decode::<i128>(&env, &data, "initial_liquidity"), 0);
    assert_eq!(
        decode::<String>(&env, &data, "metadata_url"),
        String::from_str(&env, "https://predifi.example/pool")
    );
    assert_eq!(
        decode::<Symbol>(&env, &data, "category"),
        symbol_short!("Crypto")
    );
    assert_eq!(
        decode::<soroban_sdk::Vec<String>>(&env, &data, "outcome_descriptions"),
        vec![
            &env,
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ]
    );
}

#[test]
fn create_pool_with_liquidity_emits_both_events() {
    let env = test_env();
    let h = setup(&env);
    h.mint(&h.creator, 1_000);

    let end_time = env.ledger().timestamp() + 1_000;
    let mut config = base_config(&env);
    config.initial_liquidity = 250;

    let pool_id = h.client.create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config,
    );

    assert_eq!(event_count(&env, "pool_created"), 1);
    assert_eq!(event_count(&env, "initial_liquidity_provided"), 1);

    let data =
        event_data(&env, "initial_liquidity_provided").expect("initial liquidity event missing");
    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "creator"), h.creator);
    assert_eq!(decode::<i128>(&env, &data, "amount"), 250);
}

// ── place_prediction ────────────────────────────────────────────────────────

#[test]
fn place_prediction_emits_prediction_placed_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    let user = Address::generate(&env);
    h.mint(&user, 1_000);
    h.client
        .place_prediction(&user, &pool_id, &100i128, &1u32, &None, &None);

    assert_eq!(event_count(&env, "prediction_placed"), 1);
    let data = event_data(&env, "prediction_placed").expect("prediction_placed event missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "user"), user);
    assert_eq!(decode::<i128>(&env, &data, "amount"), 100);
    assert_eq!(decode::<u32>(&env, &data, "outcome"), 1);
}

// ── close_staking ───────────────────────────────────────────────────────────

#[test]
fn close_staking_emits_staking_closed_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);
    h.client.close_staking(&pool_id);

    assert_eq!(event_count(&env, "staking_closed"), 1);
    let data = event_data(&env, "staking_closed").expect("staking_closed event missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<u64>(&env, &data, "end_time"), end_time);
    assert_eq!(decode::<i128>(&env, &data, "total_stake"), 0);
    assert_eq!(decode::<u64>(&env, &data, "timestamp"), end_time + 1);
}

// ── resolve_pool ────────────────────────────────────────────────────────────

#[test]
fn resolve_pool_emits_pool_resolved_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.set_time(end_time + 1);
    h.client.resolve_pool(&h.operator, &pool_id, &1u32);

    assert_eq!(event_count(&env, "pool_resolved"), 1);
    let data = event_data(&env, "pool_resolved").expect("pool_resolved event missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "operator"), h.operator);
    assert_eq!(decode::<u32>(&env, &data, "outcome"), 1);
}

// ── claim_winnings ──────────────────────────────────────────────────────────

#[test]
fn claim_winnings_emits_winnings_claimed_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    let user = Address::generate(&env);
    h.mint(&user, 1_000);
    h.client
        .place_prediction(&user, &pool_id, &100i128, &1u32, &None, &None);

    h.set_time(end_time + 1);
    h.client.resolve_pool(&h.operator, &pool_id, &1u32);

    let paid = h.client.claim_winnings(&user, &pool_id);
    assert_eq!(paid, 100);

    assert_eq!(event_count(&env, "winnings_claimed"), 1);
    let data = event_data(&env, "winnings_claimed").expect("winnings_claimed event missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "user"), user);
    assert_eq!(decode::<i128>(&env, &data, "amount"), 100);
}

// ── cancel_pool + claim_refund ──────────────────────────────────────────────

#[test]
fn cancel_pool_emits_pool_canceled_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.client
        .cancel_pool(&h.operator, &pool_id, &String::from_str(&env, "oracle outage"));

    assert_eq!(event_count(&env, "pool_canceled"), 1);
    let data = event_data(&env, "pool_canceled").expect("pool_canceled event missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "caller"), h.operator);
    assert_eq!(decode::<Address>(&env, &data, "operator"), h.operator);
    assert_eq!(
        decode::<String>(&env, &data, "reason"),
        String::from_str(&env, "oracle outage")
    );
}

#[test]
fn claim_refund_emits_refund_claimed_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    let user = Address::generate(&env);
    h.mint(&user, 1_000);
    h.client
        .place_prediction(&user, &pool_id, &100i128, &0u32, &None, &None);

    h.client
        .cancel_pool(&h.operator, &pool_id, &String::from_str(&env, "canceled"));

    let refund = h.client.claim_refund(&user, &pool_id);
    assert_eq!(refund, 100);

    assert_eq!(event_count(&env, "refund_claimed"), 1);
    let data = event_data(&env, "refund_claimed").expect("refund_claimed event missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "user"), user);
    assert_eq!(decode::<i128>(&env, &data, "amount"), 100);
}

// ── update_pool_description ─────────────────────────────────────────────────

#[test]
fn update_pool_description_emits_event_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    let new_desc = String::from_str(&env, "clarified description");
    h.client
        .update_pool_description(&h.creator, &pool_id, &new_desc);

    assert_eq!(event_count(&env, "pool_description_updated"), 1);
    let data =
        event_data(&env, "pool_description_updated").expect("pool_description_updated missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "caller"), h.creator);
    assert_eq!(decode::<String>(&env, &data, "new_description"), new_desc);
}

// ── set_stake_limits ────────────────────────────────────────────────────────

#[test]
fn set_stake_limits_emits_event_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.client
        .set_stake_limits(&h.operator, &pool_id, &5i128, &100i128);

    assert_eq!(event_count(&env, "stake_limits_updated"), 1);
    let data = event_data(&env, "stake_limits_updated").expect("stake_limits_updated missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "operator"), h.operator);
    assert_eq!(decode::<i128>(&env, &data, "min_stake"), 5);
    assert_eq!(decode::<i128>(&env, &data, "max_stake"), 100);
}

// ── increase_max_total_stake ────────────────────────────────────────────────

#[test]
fn increase_max_total_stake_emits_event_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;
    let pool_id = create_pool(&env, &h, end_time);

    h.client
        .increase_max_total_stake(&h.creator, &pool_id, &500i128);

    assert_eq!(event_count(&env, "max_stake_increased"), 1);
    let data = event_data(&env, "max_stake_increased").expect("max_stake_increased missing");

    assert_eq!(decode::<u64>(&env, &data, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &data, "creator"), h.creator);
    assert_eq!(decode::<i128>(&env, &data, "new_max_total_stake"), 500);
}

// ── token whitelist ─────────────────────────────────────────────────────────

#[test]
fn add_token_to_whitelist_emits_event_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let token = Address::generate(&env);

    h.client.add_token_to_whitelist(&h.admin, &token);

    assert_eq!(event_count(&env, "token_whitelist_added"), 1);
    let data = event_data(&env, "token_whitelist_added").expect("token_whitelist_added missing");

    assert_eq!(decode::<Address>(&env, &data, "admin"), h.admin);
    assert_eq!(decode::<Address>(&env, &data, "token"), token);
}

#[test]
fn remove_token_from_whitelist_emits_event_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let token = Address::generate(&env);
    h.client.add_token_to_whitelist(&h.admin, &token);

    h.client.remove_token_from_whitelist(&h.admin, &token);

    assert_eq!(event_count(&env, "token_whitelist_removed"), 1);
    let data =
        event_data(&env, "token_whitelist_removed").expect("token_whitelist_removed missing");

    assert_eq!(decode::<Address>(&env, &data, "admin"), h.admin);
    assert_eq!(decode::<Address>(&env, &data, "token"), token);
}

// ── set_treasury ────────────────────────────────────────────────────────────

#[test]
fn set_treasury_emits_treasury_update_with_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let new_treasury = Address::generate(&env);

    h.client.set_treasury(&h.admin, &new_treasury);

    assert_eq!(event_count(&env, "treasury_update"), 1);
    let data = event_data(&env, "treasury_update").expect("treasury_update missing");

    assert_eq!(decode::<Address>(&env, &data, "admin"), h.admin);
    assert_eq!(decode::<Address>(&env, &data, "treasury"), new_treasury);
}

// ── private-pool whitelist ──────────────────────────────────────────────────

#[test]
fn private_pool_whitelist_events_have_exact_fields() {
    let env = test_env();
    let h = setup(&env);
    let end_time = env.ledger().timestamp() + 1_000;

    let mut config = base_config(&env);
    config.private = true;
    let pool_id = h.client.create_pool(
        &h.creator,
        &end_time,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config,
    );

    let user = Address::generate(&env);
    let now = env.ledger().timestamp();
    h.client.add_to_whitelist(&h.creator, &pool_id, &user);

    assert_eq!(event_count(&env, "added_to_whitelist"), 1);
    let added = event_data(&env, "added_to_whitelist").expect("added_to_whitelist missing");
    assert_eq!(decode::<u64>(&env, &added, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &added, "user"), user);
    assert_eq!(decode::<Address>(&env, &added, "added_by"), h.creator);
    assert_eq!(decode::<u64>(&env, &added, "timestamp"), now);

    h.client.remove_from_whitelist(&h.creator, &pool_id, &user);

    assert_eq!(event_count(&env, "removed_from_whitelist"), 1);
    let removed =
        event_data(&env, "removed_from_whitelist").expect("removed_from_whitelist missing");
    assert_eq!(decode::<u64>(&env, &removed, "pool_id"), pool_id);
    assert_eq!(decode::<Address>(&env, &removed, "user"), user);
    assert_eq!(decode::<Address>(&env, &removed, "removed_by"), h.creator);
    assert_eq!(decode::<u64>(&env, &removed, "timestamp"), now);
}

// ── negative path ───────────────────────────────────────────────────────────

#[test]
fn failed_entry_point_emits_no_event() {
    let env = test_env();
    let h = setup(&env);

    // `end_time` in the past is rejected, so no `pool_created` event is published.
    let result = h.client.try_create_pool(
        &h.creator,
        &1u64,
        &h.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &base_config(&env),
    );

    assert!(result.is_err());
    assert_eq!(event_count(&env, "pool_created"), 0);
}
