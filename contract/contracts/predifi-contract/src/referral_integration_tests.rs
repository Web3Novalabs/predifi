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
