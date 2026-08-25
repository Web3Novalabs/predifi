#![cfg(test)]
#![allow(deprecated)]

//! Boundary and edge-case coverage for four entry points
//! (issues #1317, #1318, #1322, #1326).
//!
//! Reuses `test::setup` so the harness matches the existing suite rather than
//! standing up a second, subtly different one.

extern crate std;

use super::*;
use crate::test::setup;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, BytesN, Env, String,
};

/// Create an active pool with the given stake bounds.
fn make_pool(
    env: &Env,
    client: &PredifiContractClient,
    creator: &Address,
    token: &Address,
    min_stake: i128,
    max_stake: i128,
) -> u64 {
    client.create_pool(
        creator,
        &100_000u64,
        token,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(env, "Boundary Pool"),
            metadata_url: String::from_str(env, "ipfs://boundary"),
            min_stake,
            max_stake,
            max_total_stake: 1_000_000i128,
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

// ─── #1317: batch_claim_winnings ─────────────────────────────────────────────

#[test]
fn batch_claim_empty_pool_ids_returns_empty_map() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, _) = setup(&env);

    let user = Address::generate(&env);
    let result = client.batch_claim_winnings(&user, &vec![&env]);

    // An empty batch is a no-op, not an error — a caller filtering a list down
    // to nothing should not have to special-case the empty result.
    assert_eq!(result.len(), 0);
}

#[test]
fn batch_claim_invalid_pool_ids_yield_zero_not_failure() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, _) = setup(&env);

    let user = Address::generate(&env);
    let result = client.batch_claim_winnings(&user, &vec![&env, 9_999u64, 10_000u64]);

    // claim_winnings_internal is wrapped in unwrap_or(0), so unknown pools are
    // reported as a zero claim rather than aborting the whole batch. This is
    // the partial-failure contract: one bad id must not strand the others.
    assert_eq!(result.len(), 2);
    assert_eq!(result.get(9_999u64).unwrap(), 0);
    assert_eq!(result.get(10_000u64).unwrap(), 0);
}

#[test]
fn batch_claim_duplicate_pool_ids_collapse_to_one_entry() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);
    let user = Address::generate(&env);

    let result = client.batch_claim_winnings(&user, &vec![&env, pool_id, pool_id, pool_id]);

    // Results are keyed by pool_id in a Map, so a duplicated id cannot produce
    // a duplicated payout entry. Worth pinning: if this ever became a Vec, the
    // same pool could appear — and be read as — multiple claims.
    assert_eq!(result.len(), 1);
}

#[test]
fn batch_claim_mixed_valid_and_invalid_ids_returns_an_entry_for_each() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);
    let user = Address::generate(&env);

    let result = client.batch_claim_winnings(&user, &vec![&env, pool_id, 9_999u64]);

    // Every requested id is accounted for, so a caller can reconcile the
    // response against the request without inferring which ones were dropped.
    assert_eq!(result.len(), 2);
    assert!(result.get(pool_id).is_some());
    assert!(result.get(9_999u64).is_some());
}

#[test]
fn batch_claim_large_batch_is_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, _) = setup(&env);

    let user = Address::generate(&env);
    let mut ids = vec![&env];
    for i in 0..50u64 {
        ids.push_back(i);
    }

    // There is no explicit cap on the batch size; this documents that, so a
    // future limit is a deliberate change rather than a silent regression.
    let result = client.batch_claim_winnings(&user, &ids);
    assert_eq!(result.len(), 50);
}

// ─── #1318: set_stake_limits ─────────────────────────────────────────────────

#[test]
fn set_stake_limits_rejects_min_greater_than_max() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 100i128);

    let res = client.try_set_stake_limits(&operator, &pool_id, &500i128, &100i128);
    assert!(res.is_err(), "min_stake above max_stake must be rejected");
}

#[test]
fn set_stake_limits_rejects_zero_min() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 100i128);

    // A zero minimum would let a prediction be placed with no stake at all.
    assert!(client.try_set_stake_limits(&operator, &pool_id, &0i128, &0i128).is_err());
    assert!(client.try_set_stake_limits(&operator, &pool_id, &-1i128, &100i128).is_err());
}

#[test]
fn set_stake_limits_treats_zero_max_as_unbounded() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 100i128);

    // validate_stake_limits only compares against max when it is > 0, so zero
    // is the sentinel for "no ceiling" rather than an invalid bound.
    client.set_stake_limits(&operator, &pool_id, &5i128, &0i128);
}

#[test]
fn set_stake_limits_accepts_i128_max_ceiling() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 100i128);

    // The upper bound must not overflow any comparison inside validation.
    client.set_stake_limits(&operator, &pool_id, &1i128, &i128::MAX);
}

#[test]
fn set_stake_limits_requires_operator_role() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 100i128);

    let stranger = Address::generate(&env);
    assert!(
        client.try_set_stake_limits(&stranger, &pool_id, &5i128, &50i128).is_err(),
        "only an operator may change stake limits"
    );
}

// ─── #1322: upgrade_contract ─────────────────────────────────────────────────

#[test]
fn upgrade_contract_rejects_non_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, operator, _) = setup(&env);

    let hash = BytesN::from_array(&env, &[0u8; 32]);

    // The operator role is privileged but must not reach the upgrade path —
    // an upgrade replaces every other control in the contract.
    assert_eq!(
        client.try_upgrade_contract(&operator, &hash),
        Err(Ok(PredifiError::Unauthorized))
    );

    let stranger = Address::generate(&env);
    assert_eq!(
        client.try_upgrade_contract(&stranger, &hash),
        Err(Ok(PredifiError::Unauthorized))
    );
}

#[test]
fn upgrade_contract_rejects_unknown_wasm_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _, _, _, _, _, _) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &0u32);

    // A hash with no uploaded WASM behind it must fail after authorization.
    let hash = BytesN::from_array(&env, &[7u8; 32]);
    assert!(client.try_upgrade_contract(&admin, &hash).is_err());
    assert_eq!(client.get_version(), 1);
}

#[test]
fn upgrade_with_active_pool_preserves_state_and_allows_migration() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, token_address, _, _, _, _, creator) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &0u32);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1, 100);
    let before = client.get_pool(&pool_id);
    let invalid_hash = BytesN::from_array(&env, &[9u8; 32]);

    // An upgrade attempt must not be able to discard an active pool.
    assert!(client.try_upgrade_contract(&admin, &invalid_hash).is_err());
    let after = client.get_pool(&pool_id);
    assert_eq!(after, before);
    assert_eq!(client.get_version(), 1);

    // Migration is a separate, authorized post-upgrade operation and must be
    // safe to invoke with the preserved active state.
    client.migrate_state(&admin);
    assert_eq!(client.get_pool(&pool_id), before);
}

// ─── #1326: update_pool_description ──────────────────────────────────────────

#[test]
fn update_description_rejects_empty_string() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);

    let res = client.try_update_pool_description(&creator, &pool_id, &String::from_str(&env, ""));
    assert!(res.is_err(), "an empty description must be rejected");
}

#[test]
fn update_description_accepts_the_maximum_length() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);

    // The guard is `len() > 256`, so exactly 256 is the last accepted value.
    // An off-by-one here would silently cap descriptions one byte short.
    let at_limit = core::str::from_utf8(&[b'a'; 256]).unwrap();
    client.update_pool_description(&creator, &pool_id, &String::from_str(&env, at_limit));
}

#[test]
fn update_description_rejects_one_byte_over_the_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);

    let over = core::str::from_utf8(&[b'a'; 257]).unwrap();
    let res =
        client.try_update_pool_description(&creator, &pool_id, &String::from_str(&env, over));
    assert!(res.is_err(), "257 bytes must be rejected");
}

#[test]
fn update_description_measures_unicode_in_bytes_not_characters() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);

    // Emoji and accents are accepted...
    client.update_pool_description(
        &creator,
        &pool_id,
        &String::from_str(&env, "Prédiction 🎯 — «marché» 日本語"),
    );

    // ...but the 256 limit is a byte limit, so 100 four-byte emoji (400 bytes)
    // exceed it even though that is well under 256 characters.
    let mut many = std::string::String::new();
    for _ in 0..100 {
        many.push('🎯');
    }
    let res = client.try_update_pool_description(
        &creator,
        &pool_id,
        &String::from_str(&env, many.as_str()),
    );
    assert!(res.is_err(), "byte length, not character count, bounds the description");
}

#[test]
fn update_description_rejects_non_creator() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);

    let stranger = Address::generate(&env);
    let res = client.try_update_pool_description(
        &stranger,
        &pool_id,
        &String::from_str(&env, "hijacked"),
    );
    assert!(res.is_err(), "only the creator or an admin may edit the description");
}

#[test]
fn update_description_rejects_after_the_pool_has_ended() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);

    // Past end_time the pool is no longer editable, so a resolved or expired
    // pool cannot have its terms rewritten after the fact.
    env.ledger().with_mut(|l| l.timestamp = 200_000);

    let res = client.try_update_pool_description(
        &creator,
        &pool_id,
        &String::from_str(&env, "too late"),
    );
    assert!(res.is_err(), "an ended pool must not accept a description change");
}
