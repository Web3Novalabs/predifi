//! Referral system end-to-end integration tests (Issue #1334).
//!
//! Covers: referrer registration via place_prediction, volume tracking,
//! referral cut calculation, and self-referral rejection.

#![cfg(test)]

use crate::test::ROLE_ADMIN;
use crate::PoolConfig;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

fn make_pool(
    env: &Env,
    client: &crate::PredifiContractClient,
    creator: &Address,
    token_address: &Address,
    end_time: u64,
) -> u64 {
    client.create_pool(
        creator,
        &end_time,
        token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(env, "Referral test pool"),
            metadata_url: String::from_str(env, "ipfs://referral"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                env,
                String::from_str(env, "No"),
                String::from_str(env, "Yes"),
            ],
        },
    )
}

/// Placing predictions with a referrer accumulates volume for that referrer.
#[test]
fn test_referral_volume_tracking() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, token_address, _token, token_admin_client, _treasury, _operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let referrer = Address::generate(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    token_admin_client.mint(&user_a, &300);
    token_admin_client.mint(&user_b, &200);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 5_000);

    client.place_prediction(&user_a, &pool_id, &300, &0, &Some(referrer.clone()), &None);
    assert_eq!(client.get_referred_volume(&referrer, &pool_id), 300);

    client.place_prediction(&user_b, &pool_id, &200, &1, &Some(referrer.clone()), &None);
    assert_eq!(client.get_referred_volume(&referrer, &pool_id), 500);
}

/// Referral cut is paid to the referrer on claim_winnings.
#[test]
fn test_referral_cut_paid_on_claim() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let ac_id = env.register(
        crate::test::dummy_access_control::DummyAccessControl,
        (),
    );
    let ac_client =
        crate::test::dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);

    let contract_id = env.register(crate::PredifiContract, ());
    let client = crate::PredifiContractClient::new(&env, &contract_id);

    let token_admin_addr = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin_addr.clone());
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let treasury = Address::generate(&env);
    let operator = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);

    ac_client.grant_role(&operator, &crate::test::ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // 2% protocol fee; 50% of that goes to the referrer.
    client.init(&ac_id, &treasury, &200u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_address);
    client.set_referral_cut_bps(&admin, &5000u32);

    let referrer = Address::generate(&env);
    let referred = Address::generate(&env);

    token_admin_client.mint(&referred, &1_000);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 5_000);

    client.place_prediction(&referred, &pool_id, &1_000, &0, &Some(referrer.clone()), &None);

    assert_eq!(client.get_referred_volume(&referrer, &pool_id), 1_000);

    env.ledger().with_mut(|li| li.timestamp = 5_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    // Protocol fee = 2% of 1000 = 20. Referrer cut = 50% of 20 = 10. Payout = 980.
    let winnings = client.claim_winnings(&referred, &pool_id);
    assert_eq!(winnings, 980);
    assert_eq!(token.balance(&referred), 980);
    assert_eq!(token.balance(&referrer), 10);
}

/// A user must not be able to refer themselves.
#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_update_referrer_self_referral_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (_ac_client, client, token_address, _token, token_admin_client, _treasury, _operator, creator) =
        crate::test::setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &100);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 5_000);

    client.update_referrer(&user, &pool_id, &Some(user.clone()));
}

/// Referred volume is scoped per-pool; different pools have independent counts.
#[test]
fn test_referral_volume_is_per_pool() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, token_address, _token, token_admin_client, _treasury, _operator, creator) =
        crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let referrer = Address::generate(&env);
    let user = Address::generate(&env);
    token_admin_client.mint(&user, &600);

    let pool_a = make_pool(&env, &client, &creator, &token_address, 5_000);
    let pool_b = make_pool(&env, &client, &creator, &token_address, 5_000);

    client.place_prediction(&user, &pool_a, &400, &0, &Some(referrer.clone()), &None);
    client.place_prediction(&user, &pool_b, &200, &0, &Some(referrer.clone()), &None);

    assert_eq!(client.get_referred_volume(&referrer, &pool_a), 400);
    assert_eq!(client.get_referred_volume(&referrer, &pool_b), 200);
}

/// Registration before the first prediction is used when no referrer is
/// supplied at placement time, and later updates route only new volume.
#[test]
fn test_referrer_registration_and_updates_route_new_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (_ac_client, client, token_address, _token, token_admin_client, _treasury, _operator, creator) =
        crate::test::setup(&env);

    let user = Address::generate(&env);
    let first_referrer = Address::generate(&env);
    let second_referrer = Address::generate(&env);
    token_admin_client.mint(&user, &600);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 5_000);

    // Register before placement; the explicit placement referrer is omitted.
    client.update_referrer(&user, &pool_id, &Some(first_referrer.clone()));
    client.place_prediction(&user, &pool_id, &100, &0, &None, &None);
    assert_eq!(client.get_referred_volume(&first_referrer, &pool_id), 100);

    // Changing the referrer does not rewrite historical volume.
    client.update_referrer(&user, &pool_id, &Some(second_referrer.clone()));
    client.place_prediction(&user, &pool_id, &200, &0, &None, &None);
    assert_eq!(client.get_referred_volume(&first_referrer, &pool_id), 100);
    assert_eq!(client.get_referred_volume(&second_referrer, &pool_id), 200);

    // Clearing the registration stops referral tracking for subsequent stake.
    client.update_referrer(&user, &pool_id, &None);
    client.place_prediction(&user, &pool_id, &300, &0, &None, &None);
    assert_eq!(client.get_referred_volume(&second_referrer, &pool_id), 200);
}

/// Referral cuts from referred winners accumulate across independent pools.
#[test]
fn test_referral_cuts_and_volume_accumulate_across_pools() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let ac_id = env.register(
        crate::test::dummy_access_control::DummyAccessControl,
        (),
    );
    let ac_client =
        crate::test::dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(crate::PredifiContract, ());
    let client = crate::PredifiContractClient::new(&env, &contract_id);

    let token_admin_addr = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin_addr.clone());
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let treasury = Address::generate(&env);
    let operator = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);

    ac_client.grant_role(&operator, &crate::test::ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &200u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_contract);
    client.set_referral_cut_bps(&admin, &5000u32);

    let referrer = Address::generate(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    token_admin_client.mint(&user_a, &500);
    token_admin_client.mint(&user_b, &300);

    let pool_a = make_pool(&env, &client, &creator, &token_contract, 5_000);
    let pool_b = make_pool(&env, &client, &creator, &token_contract, 5_000);
    client.place_prediction(&user_a, &pool_a, &500, &0, &Some(referrer.clone()), &None);
    client.place_prediction(&user_b, &pool_b, &300, &0, &Some(referrer.clone()), &None);

    assert_eq!(client.get_referred_volume(&referrer, &pool_a), 500);
    assert_eq!(client.get_referred_volume(&referrer, &pool_b), 300);

    env.ledger().with_mut(|li| li.timestamp = 5_001);
    client.resolve_pool(&operator, &pool_a, &0u32);
    client.resolve_pool(&operator, &pool_b, &0u32);

    // Each sole winner pays 2% protocol fee, half of which goes to the referrer.
    assert_eq!(client.claim_winnings(&user_a, &pool_a), 490);
    assert_eq!(client.claim_winnings(&user_b, &pool_b), 294);
    assert_eq!(token.balance(&referrer), 8);
}

/// Full referral end-to-end: two users referred by the same referrer each win,
/// both pay a referral cut, and the referrer accumulates cuts from both payouts.
///
/// Covers issue #1463 — Integration Tests: Referral system end-to-end.
///
/// Scenario:
/// - referrer refers user_a (outcome 0) and user_b (outcome 0) on the same pool.
/// - Both users stake on the winning outcome.
/// - Each payout sends 50% of the referral_bps share (default 50% of protocol fee,
///   which is 0% in test setup) to the referrer.
/// - Total referrer earnings = sum of cuts from both payouts.
#[test]
fn test_referral_end_to_end_two_winners_same_referrer() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (_ac_client, client, token_address, token, token_admin_client, _treasury, operator, creator) =
        crate::test::setup(&env);

    let referrer = Address::generate(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    token_admin_client.mint(&user_a, &400);
    token_admin_client.mint(&user_b, &600);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 5_000);

    // Both users bet on outcome 0 via the same referrer.
    client.place_prediction(&user_a, &pool_id, &400, &0, &Some(referrer.clone()), &None);
    client.place_prediction(&user_b, &pool_id, &600, &0, &Some(referrer.clone()), &None);

    // Verify referrer accumulated volume from both predictions.
    assert_eq!(client.get_referred_volume(&referrer, &pool_id), 1_000);

    // Advance and resolve outcome 0 as the winner.
    env.ledger().with_mut(|li| li.timestamp = 5_001);
    client.close_staking(&pool_id);
    client.mark_pool_ready(&pool_id);
    client.resolve_pool(&operator, &pool_id, &0u32);

    // user_a and user_b both win; their shares are 40% and 60% of the 1000-token pool.
    let payout_a = client.claim_winnings(&user_a, &pool_id);
    let payout_b = client.claim_winnings(&user_b, &pool_id);

    assert_eq!(payout_a, 400); // 400/1000 * 1000 = 400
    assert_eq!(payout_b, 600); // 600/1000 * 1000 = 600

    // Both winners received their funds.
    assert_eq!(token.balance(&user_a), 400);
    assert_eq!(token.balance(&user_b), 600);

    // Contract is empty (0% protocol fee → 0 referral cut in this setup).
    assert_eq!(token.balance(&client.address), 0);
}

/// Referral system end-to-end with TWO referred winners sharing the same referrer:
/// both payouts carry a referral cut and the referrer accumulates both.
///
/// Covers issue #1463 — Referral system end-to-end.
///
/// Setup: 2% protocol fee; referral cut = 50% of that fee.
/// user_a stakes 400 on outcome 0; user_b stakes 600 on outcome 0 — both win.
/// Each payout = stake (no competing side), minus 2% fee, with 50% going to referrer.
///   user_a payout = 400 - 8 = 392; referrer cut from user_a = 4.
///   user_b payout = 600 - 12 = 588; referrer cut from user_b = 6.
///   total referrer earnings = 10.
#[test]
fn test_referral_end_to_end_two_winners_same_referrer_with_fee() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let ac_id = env.register(crate::test::dummy_access_control::DummyAccessControl, ());
    let ac_client =
        crate::test::dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);

    let contract_id = env.register(crate::PredifiContract, ());
    let client = crate::PredifiContractClient::new(&env, &contract_id);

    let token_admin_addr = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin_addr.clone());
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let treasury = Address::generate(&env);
    let operator = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);

    ac_client.grant_role(&operator, &crate::test::ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // 2% protocol fee; 50% of that goes to the referrer.
    client.init(&ac_id, &treasury, &200u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_address);
    client.set_referral_cut_bps(&admin, &5000u32);

    let referrer = Address::generate(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    token_admin_client.mint(&user_a, &400);
    token_admin_client.mint(&user_b, &600);

    let pool_id = make_pool(&env, &client, &creator, &token_address, 5_000);

    // Both users bet on outcome 0 via the same referrer.
    client.place_prediction(&user_a, &pool_id, &400, &0, &Some(referrer.clone()), &None);
    client.place_prediction(&user_b, &pool_id, &600, &0, &Some(referrer.clone()), &None);

    assert_eq!(client.get_referred_volume(&referrer, &pool_id), 1_000);

    env.ledger().with_mut(|li| li.timestamp = 5_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    // user_a: wins 400 (sole winner on their side with 400 stake out of 1000 total)
    // But wait — both users are on outcome 0, total winning stake = 1000 = total pool.
    // So user_a payout = (400/1000) * 1000 = 400 gross.
    // Protocol fee = 2% of 400 = 8. Net = 392. Referrer cut = 50% of 8 = 4.
    let payout_a = client.claim_winnings(&user_a, &pool_id);
    assert_eq!(payout_a, 392);
    assert_eq!(token.balance(&user_a), 392);

    // user_b: (600/1000) * 1000 = 600 gross. Fee = 12. Net = 588. Referrer cut = 6.
    let payout_b = client.claim_winnings(&user_b, &pool_id);
    assert_eq!(payout_b, 588);
    assert_eq!(token.balance(&user_b), 588);

    // Referrer accumulates cuts from both: 4 + 6 = 10.
    assert_eq!(token.balance(&referrer), 10);
}
