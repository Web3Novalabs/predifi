//! Integration tests for the `update_referrer` entry point.
//!
//! Verifies both the state transitions and the exact `referrer_updated` event
//! payload (topics + fields) relied upon by off-chain indexers.

#![cfg(test)]

mod common;

use common::{event_count, event_data, field, setup, test_env};
use predifi_contract::PredifiError;
use soroban_sdk::{testutils::Address as _, Address, Env, Map, Symbol, TryFromVal, Val};

/// Decode an event payload field into `T`.
fn decode<T>(env: &Env, data: &Map<Symbol, Val>, name: &str) -> T
where
    T: TryFromVal<Env, Val>,
{
    let val = field(env, data, name);
    T::try_from_val(env, &val).unwrap()
}

#[test]
fn update_referrer_sets_referrer_and_emits_event() {
    let env = test_env();
    let h = setup(&env);

    let user = h.creator.clone();
    let referrer = Address::generate(&env);
    let pool_id = 7u64;

    h.client
        .update_referrer(&user, &pool_id, &Some(referrer.clone()));

    assert_eq!(event_count(&env, "referrer_updated"), 1);
    let data = event_data(&env, "referrer_updated").expect("referrer_updated event");

    let got_user: Address = decode(&env, &data, "user");
    let got_pool_id: u64 = decode(&env, &data, "pool_id");
    let got_referrer: Option<Address> = decode(&env, &data, "new_referrer");

    assert_eq!(got_user, user);
    assert_eq!(got_pool_id, pool_id);
    assert_eq!(got_referrer, Some(referrer));
}

#[test]
fn update_referrer_with_none_removes_referrer() {
    let env = test_env();
    let h = setup(&env);

    let user = h.creator.clone();
    let referrer = Address::generate(&env);
    let pool_id = 3u64;

    h.client
        .update_referrer(&user, &pool_id, &Some(referrer.clone()));
    h.client.update_referrer(&user, &pool_id, &None);

    // Events reflect the most recent invocation only, so this observes the
    // `None` update.
    assert_eq!(event_count(&env, "referrer_updated"), 1);
    let data = event_data(&env, "referrer_updated").expect("referrer_updated event");
    let got_referrer: Option<Address> = decode(&env, &data, "new_referrer");
    assert_eq!(got_referrer, None);
}

#[test]
fn self_referral_is_rejected() {
    let env = test_env();
    let h = setup(&env);

    let user = h.creator.clone();

    let result = h.client.try_update_referrer(&user, &1u64, &Some(user.clone()));

    assert_eq!(result.unwrap_err().unwrap(), PredifiError::Unauthorized);
    // No event should be emitted for a rejected update.
    assert_eq!(event_count(&env, "referrer_updated"), 0);
}

#[test]
fn contract_address_referral_is_rejected() {
    let env = test_env();
    let h = setup(&env);

    let user = h.creator.clone();
    let contract_addr = h.contract_id.clone();

    let result = h
        .client
        .try_update_referrer(&user, &1u64, &Some(contract_addr));

    assert_eq!(result.unwrap_err().unwrap(), PredifiError::Unauthorized);
    assert_eq!(event_count(&env, "referrer_updated"), 0);
}

#[test]
fn update_referrer_while_paused_is_rejected() {
    let env = test_env();
    let h = setup(&env);

    let user = h.creator.clone();
    let referrer = Address::generate(&env);

    h.client.pause(&h.admin);

    let result = h
        .client
        .try_update_referrer(&user, &1u64, &Some(referrer));

    assert_eq!(result.unwrap_err().unwrap(), PredifiError::ContractPaused);
}

#[test]
fn update_referrer_can_overwrite_previous_referrer() {
    let env = test_env();
    let h = setup(&env);

    let user = h.creator.clone();
    let first = Address::generate(&env);
    let second = Address::generate(&env);

    h.client.update_referrer(&user, &5u64, &Some(first));
    h.client
        .update_referrer(&user, &5u64, &Some(second.clone()));

    let data = event_data(&env, "referrer_updated").expect("referrer_updated event");
    let got_referrer: Option<Address> = decode(&env, &data, "new_referrer");
    assert_eq!(got_referrer, Some(second));
}
