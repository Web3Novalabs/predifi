#![cfg(test)]

extern crate std;

use crate::test::ROLE_ADMIN;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, BytesN, Env, String, Symbol,
};

#[test]
fn test_set_fee_bps_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_fee_bps(&attacker, &500);
    assert!(result.is_err());
}

#[test]
fn test_set_treasury_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_treasury(&attacker, &Address::generate(&env));
    assert!(result.is_err());
}

#[test]
fn test_add_oracle_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_add_oracle(&attacker, &Address::generate(&env));
    assert!(result.is_err());
}

#[test]
fn test_remove_oracle_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.add_oracle(&Address::generate(&env), &oracle);
    let result = client.try_remove_oracle(&attacker, &oracle);
    assert!(result.is_err());
}

#[test]
fn test_upgrade_contract_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_upgrade_contract(&attacker, &BytesN::from_array(&env, &[0u8; 32]));
    assert!(result.is_err());
}

#[test]
fn test_withdraw_treasury_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_withdraw_treasury(
        &attacker,
        &_token_address,
        &1000,
        &Address::generate(&env),
    );
    assert!(result.is_err());
}

#[test]
fn test_pause_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.pause(&attacker);
    }));
    assert!(result.is_err());
}

#[test]
fn test_unpause_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.unpause(&attacker);
    }));
    assert!(result.is_err());
}

#[test]
fn test_set_resolution_delay_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_resolution_delay(&attacker, &3600);
    assert!(result.is_err());
}

#[test]
fn test_set_min_stake_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_min_stake(&attacker, &100);
    assert!(result.is_err());
}

#[test]
fn test_add_token_to_whitelist_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_add_token_to_whitelist(&attacker, &Address::generate(&env));
    assert!(result.is_err());
}

#[test]
fn test_remove_token_from_whitelist_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_remove_token_from_whitelist(&attacker, &Address::generate(&env));
    assert!(result.is_err());
}

#[test]
fn test_set_prediction_cooldown_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_prediction_cooldown(&attacker, &60);
    assert!(result.is_err());
}

#[test]
fn test_set_max_predictions_per_user_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_max_predictions_per_user(&attacker, &10);
    assert!(result.is_err());
}

#[test]
fn test_set_claim_window_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_claim_window(&attacker, &86400);
    assert!(result.is_err());
}

#[test]
fn test_set_min_pool_duration_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_min_pool_duration(&attacker, &7200);
    assert!(result.is_err());
}

#[test]
fn test_init_oracle_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_init_oracle(&attacker, &Address::generate(&env), &300, &100);
    assert!(result.is_err());
}

#[test]
fn test_set_fee_tiers_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let tier = crate::FeeTier {
        stake_threshold: 1000,
        fee_bps: 100,
    };
    let result = client.try_set_fee_tiers(&attacker, &vec![&env, tier]);
    assert!(result.is_err());
}

#[test]
fn test_set_referral_rate_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_referral_rate(&attacker, &500);
    assert!(result.is_err());
}

#[test]
fn test_set_referral_volume_threshold_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_set_referral_volume_threshold(&attacker, &1000);
    assert!(result.is_err());
}

#[test]
fn test_emergency_withdraw_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_emergency_withdraw(
        &attacker,
        &_token_address,
        &Address::generate(&env),
        &1000,
    );
    assert!(result.is_err());
}

#[test]
fn test_migrate_state_rejects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        crate::test::setup(&env);
    let attacker = Address::generate(&env);
    let result = client.try_migrate_state(&attacker);
    assert!(result.is_err());
}
