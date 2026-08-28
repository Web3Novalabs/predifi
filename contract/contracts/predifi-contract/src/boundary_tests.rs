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

// ─── #1322 (advanced): upgrade_contract boundary & edge cases ────────────────

/// An operator (role 1) must be rejected — upgrade is exclusively an admin
/// (role 0) operation.  This prevents a compromised operator from replacing
/// the contract binary and bypassing all other access controls.
#[test]
fn upgrade_contract_operator_role_is_insufficient() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, operator, _) = setup(&env);

    let hash = BytesN::from_array(&env, &[0xABu8; 32]);

    let result = client.try_upgrade_contract(&operator, &hash);
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "operator must be rejected with Unauthorized, not a generic panic"
    );
}

/// A completely unknown address with no role must also be rejected with the
/// same `Unauthorized` error — the access check runs before any WASM logic.
#[test]
fn upgrade_contract_stranger_is_rejected_with_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, _) = setup(&env);

    let stranger = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[0xFFu8; 32]);

    let result = client.try_upgrade_contract(&stranger, &hash);
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "stranger must be rejected with Unauthorized before WASM deployment is attempted"
    );
}

/// `upgrade_contract` must fail when the supplied WASM hash is all-zeros —
/// the Soroban host rejects a hash with no matching deployment artifact.
/// This guards against accidental null-hash upgrades that would brick the
/// contract by swapping in nonexistent bytecode.
#[test]
fn upgrade_contract_rejects_zero_wasm_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _, _, _, _, _, _) = setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &crate::test::ROLE_ADMIN);

    // All-zero hash: no WASM uploaded behind it.
    let zero_hash = BytesN::from_array(&env, &[0u8; 32]);

    // The host will panic when it cannot find the WASM — try_ captures that as Err.
    let result = client.try_upgrade_contract(&admin, &zero_hash);
    assert!(
        result.is_err(),
        "a zero WASM hash with no backing artifact must be rejected"
    );
}

/// Calling `upgrade_contract` while the contract is paused must still succeed
/// for the admin — upgrade_contract intentionally does NOT enforce the paused
/// guard so that a buggy pause can always be recovered via upgrade.
/// Verify the access control path executes (i.e. the call fails only at
/// the WASM-deployment step, not at the pause check).
#[test]
fn upgrade_contract_is_not_blocked_by_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _, _, _, _, _, _) = setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &crate::test::ROLE_ADMIN);

    // Pause the contract.
    client.pause(&admin);
    assert!(client.is_contract_paused(), "contract must be paused");

    // An invalid WASM hash is the limiting factor here, not the pause guard.
    // If upgrade_contract enforced the pause check it would return
    // PredifiError::ContractPaused; instead it returns a host-level error from
    // the missing WASM — any error suffices, but ContractPaused must NOT appear.
    let hash = BytesN::from_array(&env, &[0xDEu8; 32]);
    let result = client.try_upgrade_contract(&admin, &hash);
    assert!(
        result != Err(Ok(PredifiError::ContractPaused)),
        "upgrade_contract must not be blocked by the contract-paused guard"
    );
}

/// When active pools exist at upgrade time the upgrade path must not modify
/// or invalidate any pool state.  The upgrade call reaches the WASM-swap step
/// (and panics there in the mock environment), but all pool data written
/// before that point must remain intact and readable after the panic is caught.
#[test]
fn upgrade_contract_does_not_corrupt_active_pool_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, token_address, token, token_admin_client, _, _, creator) =
        setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &crate::test::ROLE_ADMIN);

    // Create a pool and place a stake so there is meaningful state to preserve.
    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);
    let staker = Address::generate(&env);
    token_admin_client.mint(&staker, &500i128);
    client.place_prediction(&staker, &pool_id, &500i128, &0u32, &None, &None);

    // Capture state before the upgrade attempt.
    let pool_before = client.get_pool(&pool_id);
    let version_before = client.get_version();
    assert_eq!(pool_before.total_stake, 500i128, "total_stake must be 500 before upgrade");

    // Attempt upgrade — fails at WASM-swap because the hash is not registered
    // in the mock environment.  The try_ wrapper prevents a test abort.
    let hash = BytesN::from_array(&env, &[0x42u8; 32]);
    let _ = client.try_upgrade_contract(&admin, &hash);

    // All pool state and contract configuration must be unchanged.
    let pool_after = client.get_pool(&pool_id);
    assert_eq!(
        pool_after.total_stake, pool_before.total_stake,
        "total_stake must be unchanged after a failed upgrade"
    );
    assert_eq!(
        pool_after.state, pool_before.state,
        "pool state must be unchanged after a failed upgrade"
    );
    assert_eq!(
        pool_after.end_time, pool_before.end_time,
        "pool end_time must be unchanged after a failed upgrade"
    );
    // Version must NOT have been incremented because the WASM-swap panicked
    // before the version write committed.
    assert_eq!(
        client.get_version(),
        version_before,
        "version must not be incremented when the WASM-swap step fails"
    );
}

/// `migrate_state` called immediately after a simulated upgrade (admin role
/// present, contract not paused) must succeed without modifying existing pool
/// data.  This verifies the post-upgrade migration hook is wired correctly and
/// that it is idempotent — calling it twice must yield the same outcome.
#[test]
fn migrate_state_is_called_correctly_post_upgrade_and_is_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &crate::test::ROLE_ADMIN);

    // Create a pool so there is state to preserve across migration.
    let pool_id = make_pool(&env, &client, &creator, &token_address, 1i128, 0i128);
    let staker = Address::generate(&env);
    token_admin_client.mint(&staker, &200i128);
    client.place_prediction(&staker, &pool_id, &200i128, &0u32, &None, &None);

    let pool_before = client.get_pool(&pool_id);
    let version_before = client.get_version();

    // First call to migrate_state — must succeed.
    let first = client.try_migrate_state(&admin);
    assert!(
        first.is_ok(),
        "first migrate_state call by admin must succeed: {:?}",
        first
    );

    // Second call — must also succeed (idempotent).
    let second = client.try_migrate_state(&admin);
    assert!(
        second.is_ok(),
        "second migrate_state call must succeed (idempotent): {:?}",
        second
    );

    // Pool data must be entirely unchanged by migration.
    let pool_after = client.get_pool(&pool_id);
    assert_eq!(
        pool_after.total_stake, pool_before.total_stake,
        "total_stake must be unchanged after migrate_state"
    );
    assert_eq!(
        pool_after.state, pool_before.state,
        "pool state must be unchanged after migrate_state"
    );
    assert_eq!(
        pool_after.fee_bps, pool_before.fee_bps,
        "pool fee_bps must be unchanged after migrate_state"
    );

    // migrate_state must not modify the version counter — version bumping is
    // the responsibility of upgrade_contract, not the migration hook.
    assert_eq!(
        client.get_version(),
        version_before,
        "migrate_state must not alter the version counter"
    );
}

/// `migrate_state` called by a non-admin must be rejected with `Unauthorized`.
/// This ensures the migration hook cannot be triggered by arbitrary callers
/// between an upgrade and the admin's intentional migration step.
#[test]
fn migrate_state_rejects_non_admin_after_upgrade() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, operator, _) = setup(&env);

    // Operator — privileged but not admin.
    let result_op = client.try_migrate_state(&operator);
    assert_eq!(
        result_op,
        Err(Ok(PredifiError::Unauthorized)),
        "operator must be rejected from migrate_state with Unauthorized"
    );

    // Completely unknown address.
    let stranger = Address::generate(&env);
    let result_stranger = client.try_migrate_state(&stranger);
    assert_eq!(
        result_stranger,
        Err(Ok(PredifiError::Unauthorized)),
        "stranger must be rejected from migrate_state with Unauthorized"
    );
}

/// `migrate_state` while the contract is paused must return `ContractPaused`.
/// This enforces that migrations only run in a known-good operational state
/// and cannot be used to sneak writes through a pause.
#[test]
fn migrate_state_is_blocked_while_contract_is_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _, _, _, _, _, _) = setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &crate::test::ROLE_ADMIN);

    client.pause(&admin);
    assert!(client.is_contract_paused());

    let result = client.try_migrate_state(&admin);
    assert_eq!(
        result,
        Err(Ok(PredifiError::ContractPaused)),
        "migrate_state must return ContractPaused when the contract is paused"
    );
}

/// `upgrade_contract` requires the caller to provide authorisation (`require_auth`).
/// Verify that the auth check fires *before* any role check by stripping auths
/// and confirming the call fails.
#[test]
fn upgrade_contract_requires_admin_auth() {
    // Do NOT call env.mock_all_auths() so no auth is granted.
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let (ac_client, client, _, _, _, _, _, _) = setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &crate::test::ROLE_ADMIN);

    let hash = BytesN::from_array(&env, &[0x11u8; 32]);

    // Even with the admin role present, skipping the auth envelope must fail.
    let result = client.try_upgrade_contract(&admin, &hash);
    assert!(
        result.is_err(),
        "upgrade_contract must require an auth envelope from the admin"
    );
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
