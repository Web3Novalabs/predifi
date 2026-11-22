//! Tests for issues #2017 (oracle confidence ratio), #2021 (treasury must not be the
//! contract itself) and #2022 (a referral rate of 0 disables referral payouts).

#![cfg(test)]

use crate::test::{ROLE_ADMIN, ROLE_OPERATOR};
use crate::{PoolConfig, PredifiError};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

// ── #2017: MIN_CONFIDENCE_RATIO_BPS is enforced in update_price_feed ───────

const PRICE: i128 = 1_000_000;

/// Submit a feed with the given confidence against a fixed price; the ratio in
/// bps is `confidence * 10_000 / PRICE`, so confidence 50_000 is exactly 500 bps.
fn submit_with_confidence(confidence: i128) -> Result<(), PredifiError> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    let oracle = Address::generate(&env);
    client.add_oracle(&admin, &oracle);

    match client.try_update_price_feed(
        &oracle,
        &symbol_short!("BTCUSD"),
        &PRICE,
        &confidence,
        &999u64,
        &2_000u64,
    ) {
        Ok(Ok(())) => Ok(()),
        Err(Ok(e)) => Err(e),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn test_confidence_ratio_just_below_limit_is_accepted() {
    // 49_999 * 10_000 / 1_000_000 = 499 bps
    assert_eq!(submit_with_confidence(49_999), Ok(()));
}

#[test]
fn test_confidence_ratio_exactly_at_limit_is_accepted() {
    // 50_000 * 10_000 / 1_000_000 = 500 bps
    assert_eq!(submit_with_confidence(50_000), Ok(()));
}

#[test]
fn test_confidence_ratio_just_above_limit_is_rejected() {
    // 50_100 * 10_000 / 1_000_000 = 501 bps
    assert_eq!(
        submit_with_confidence(50_100),
        Err(PredifiError::InvalidData)
    );
}

#[test]
fn test_confidence_larger_than_price_is_rejected() {
    assert_eq!(
        submit_with_confidence(PRICE * 2),
        Err(PredifiError::InvalidData)
    );
}

#[test]
fn test_zero_confidence_is_accepted() {
    assert_eq!(submit_with_confidence(0), Ok(()));
}

#[test]
fn test_confidence_multiplication_overflow_is_rejected_not_panicking() {
    assert_eq!(
        submit_with_confidence(i128::MAX),
        Err(PredifiError::InvalidData)
    );
}

// ── #2021: set_treasury rejects the contract's own address ─────────────────

#[test]
fn test_set_treasury_rejects_contract_own_address() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, treasury, _operator, _creator) =
        crate::test::setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let result = client.try_set_treasury(&admin, &client.address);

    assert_eq!(result, Err(Ok(PredifiError::InvalidAddressOrToken)));
    // The previously configured treasury is untouched.
    assert_eq!(client.get_fee_config().treasury, treasury);
}

#[test]
fn test_set_treasury_accepts_a_valid_address() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    let new_treasury = Address::generate(&env);

    client.set_treasury(&admin, &new_treasury);

    assert_eq!(client.get_fee_config().treasury, new_treasury);
}

// ── #2022: set_referral_rate(0) disables referral payouts ──────────────────

fn make_pool(
    env: &Env,
    client: &crate::PredifiContractClient,
    creator: &Address,
    token_address: &Address,
) -> u64 {
    client.create_pool(
        creator,
        &5_000u64,
        token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(env, "Referral disable pool"),
            metadata_url: String::from_str(env, "ipfs://referral-disable"),
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

#[test]
fn test_referral_rate_zero_overrides_legacy_cut_and_pays_no_referrer() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let ac_id = env.register(crate::test::dummy_access_control::DummyAccessControl, ());
    let ac_client = crate::test::dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(crate::PredifiContract, ());
    let client = crate::PredifiContractClient::new(&env, &contract_id);

    let token_admin_addr = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin_addr);
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);

    let treasury = Address::generate(&env);
    let operator = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // 2% protocol fee so a non-zero referral cut would be observable.
    client.init(&ac_id, &treasury, &200u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_contract);

    // A season starts with a 50% cut, then the admin ends it with rate 0.
    client.set_referral_cut_bps(&admin, &5000u32);
    assert_eq!(client.get_referral_cut_bps(), 5000);
    client.set_referral_rate(&admin, &0u32);
    assert_eq!(client.get_referral_cut_bps(), 0);

    let referrer = Address::generate(&env);
    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1_000);

    let pool_id = make_pool(&env, &client, &creator, &token_contract);
    client.place_prediction(&user, &pool_id, &1_000, &0, &Some(referrer.clone()), &None);

    env.ledger().with_mut(|li| li.timestamp = 5_001);
    client.resolve_pool(&operator, &pool_id, &0u32);
    client.claim_winnings(&user, &pool_id);

    assert_eq!(token.balance(&referrer), 0);
}

#[test]
fn test_referral_rate_nonzero_still_applies() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    client.set_referral_rate(&admin, &1_200u32);

    assert_eq!(client.get_referral_cut_bps(), 1_200);
}
