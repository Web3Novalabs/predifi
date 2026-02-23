#![cfg(test)]
#![allow(deprecated)]

use super::*;
// bring safe math helpers into test scope
use crate::safe_math::{RoundingMode, SafeMath};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

use proptest::prelude::*;

mod dummy_access_control {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    // ── Property‑based tests ─────────────────────────────────────────────────────

    prop_compose! {
        fn stakes_and_outcomes()
            (len in 1..6usize)
            (pairs in prop::collection::vec((1i128..1000, 1u32..3), len)) -> Vec<(i128, u32)> {
                pairs
            }
    }

    proptest! {
        #[test]
        fn prop_payout_fee_consistency(pairs in stakes_and_outcomes(), fee_bps in 0u32..10001, winning_outcome in 1u32..3) {
            // compute totals
            let total_stake: i128 = pairs.iter().map(|(s, _)| *s).sum();
            let winning_stake: i128 = pairs.iter().filter(|(_, o)| *o == winning_outcome).map(|(s, _)| *s).sum();

            // calculate expected shares/fees using SafeMath
            let mut expected_payouts = vec![];
            let mut expected_fees = 0i128;
            for (stake, outcome) in &pairs {
                if *outcome == winning_outcome && winning_stake > 0 {
                    let share = SafeMath::proportion(*stake, winning_stake, total_stake, RoundingMode::ProtocolFavor)?;
                    let fee = SafeMath::percentage(share, fee_bps as i128, RoundingMode::ProtocolFavor)?;
                    expected_fees = expected_fees.checked_add(fee).unwrap();
                    expected_payouts.push(share.checked_sub(fee).unwrap());
                } else {
                    expected_payouts.push(0);
                }
            }
            let expected_total_payout: i128 = expected_payouts.iter().sum();

            // now drive contract simulation
            let env = Env::default();
            env.mock_all_auths();
            let (ac_client, client, token_address, token, token_admin_client, treasury, operator) = setup(&env);
            let admin = Address::generate(&env);
            ac_client.grant_role(&admin, &ROLE_ADMIN);
            // make admin and set fee
            client
                .set_fee_bps(&admin, &fee_bps)
                .unwrap_or_else(|_| panic!("failed setting fee"));

            // create users and predictions
            let mut user_addrs = vec![];
            for _ in 0..pairs.len() {
                user_addrs.push(Address::generate(&env));
            }
            // mint tokens for each user
            for (i, (stake, _)) in pairs.iter().enumerate() {
                token_admin_client.mint(&user_addrs[i], stake);
            }
            let pool_id = client.create_pool(
                &100u64,
                &token_address,
                &String::from_str(&env, "Prop Pool"),
                &String::from_str(&env, "meta"),
            );
            for (i, (stake, outcome)) in pairs.iter().enumerate() {
                client.place_prediction(&user_addrs[i], &pool_id, stake, outcome);
            }
            // resolve
            env.ledger().with_mut(|li| li.timestamp = 101);
            client.resolve_pool(&operator, &pool_id, &winning_outcome).unwrap();

            let treasury_before = token.balance(&treasury);
            let mut actual_payouts = vec![];
            for addr in &user_addrs {
                let r = client.claim_winnings(addr, &pool_id);
                actual_payouts.push(r);
            }
            let treasury_after = token.balance(&treasury);
            let actual_total_payout: i128 = actual_payouts.iter().sum();
            let actual_total_fees = treasury_after.checked_sub(treasury_before).unwrap();

            // compare
            prop_assert_eq!(actual_total_payout, expected_total_payout);
            prop_assert_eq!(actual_total_fees, expected_fees);
            prop_assert!(actual_total_payout + actual_total_fees <= total_stake);
        }
    }



    // ── Pool Cancelation & State Guard Tests ────────────────────────────────────────

    #[test]
    fn test_admin_can_cancel_pool() {
        let env = Env::default();
        env.mock_all_auths();

        let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
        let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract(token_admin.clone());
        let token_address = token_contract;

        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        ac_client.grant_role(&admin, &ROLE_OPERATOR);
        client.init(&ac_id, &treasury, &0u32, &0u64);

        let pool_id = client.create_pool(
            &100000u64,
            &token_address,
            &3u32,
            &String::from_str(&env, "Test Pool"),
            &String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
        );

        // Admin should be able to cancel
        client.cancel_pool(&admin, &pool_id);
    }

    #[test]
    fn test_pool_creator_can_cancel_unresolved_pool() {
        let env = Env::default();
        env.mock_all_auths();

        let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
        let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract(token_admin.clone());
        let token_address = token_contract;

        let creator = Address::generate(&env);
        let treasury = Address::generate(&env);
        ac_client.grant_role(&creator, &ROLE_OPERATOR);
        client.init(&ac_id, &treasury, &0u32, &0u64);

        let pool_id = client.create_pool(
            &100000u64,
            &token_address,
            &3u32,
            &String::from_str(&env, "Test Pool"),
            &String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
        );

        // Admin should be able to cancel their pool
        client.cancel_pool(&creator, &pool_id);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")]
    fn test_non_admin_non_creator_cannot_cancel() {
        let env = Env::default();
        env.mock_all_auths();

        let (_, client, token_address, _, _, _, _) = setup(&env);

        let pool_id = client.create_pool(
            &100000u64,
            &token_address,
            &3u32,
            &String::from_str(&env, "Test Pool"),
            &String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
        );

        let unauthorized = Address::generate(&env);
        // This should fail - user is not admin
        client.cancel_pool(&unauthorized, &pool_id);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #22)")]
    fn test_cannot_cancel_resolved_pool() {
        let env = Env::default();
        env.mock_all_auths();

        let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
        let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract(token_admin.clone());
        let token_address = token_contract;

        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        let treasury = Address::generate(&env);
        ac_client.grant_role(&admin, &ROLE_OPERATOR);
        ac_client.grant_role(&operator, &ROLE_OPERATOR);
        client.init(&ac_id, &treasury, &0u32, &0u64);

        let pool_id = client.create_pool(
            &100000u64,
            &token_address,
            &3u32,
            &String::from_str(&env, "Test Pool"),
            &String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
        );

        env.ledger().with_mut(|li| li.timestamp = 100001);
        client.resolve_pool(&operator, &pool_id, &1u32);

        // Now try to cancel - should fail
        client.cancel_pool(&admin, &pool_id);
    }

    #[test]
    #[should_panic(expected = "Cannot place prediction on canceled pool")]
    fn test_cannot_place_prediction_on_canceled_pool() {
        let env = Env::default();
        env.mock_all_auths();

        let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
        let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract(token_admin.clone());
        let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
        let token_address = token_contract;

        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        ac_client.grant_role(&admin, &ROLE_OPERATOR);
        client.init(&ac_id, &treasury, &0u32, &0u64);

        let user = Address::generate(&env);
        token_admin_client.mint(&user, &1000);

        // Create and cancel pool
        let pool_id = client.create_pool(
            &100000u64,
            &token_address,
            &3u32,
            &String::from_str(&env, "Test Pool"),
            &String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
        );

        // Cancel the pool
        client.cancel_pool(&admin, &pool_id);

        // Try to place prediction on canceled pool - should panic
        client.place_prediction(&user, &pool_id, &100, &1);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")]
    fn test_pool_creator_cannot_cancel_after_admin_cancels() {
        let env = Env::default();
        env.mock_all_auths();

        let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
        let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract(token_admin.clone());
        let token_address = token_contract;

        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        ac_client.grant_role(&admin, &ROLE_OPERATOR);
        client.init(&ac_id, &treasury, &0u32, &0u64);

        let pool_id = client.create_pool(
            &100000u64,
            &token_address,
            &3u32,
            &String::from_str(&env, "Test Pool"),
            &String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
        );

        // Admin cancels the pool
        client.cancel_pool(&admin, &pool_id);

        // Attempt to cancel again should fail (already canceled)
        let non_admin = Address::generate(&env);
        client.cancel_pool(&non_admin, &pool_id);
    }

    #[test]
    #[should_panic(expected = "Cannot place prediction on canceled pool")]
    fn test_admin_can_cancel_pool_with_predictions() {
        let env = Env::default();
        env.mock_all_auths();

        let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
        let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract(token_admin.clone());
        let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
        let token_address = token_contract;

        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        ac_client.grant_role(&admin, &ROLE_OPERATOR);
        client.init(&ac_id, &treasury, &0u32, &0u64);

        let user = Address::generate(&env);
        token_admin_client.mint(&user, &1000);

        let pool_id = client.create_pool(
            &100000u64,
            &token_address,
            &3u32,
            &String::from_str(&env, "Test Pool"),
            &String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
        );

        // User places a prediction
        client.place_prediction(&user, &pool_id, &100, &1);

        // Admin cancels the pool - this freezes betting
        client.cancel_pool(&admin, &pool_id);

        // Verify no more predictions can be placed - should panic
        client.place_prediction(&user, &pool_id, &50, &2);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")]
    fn test_unauthorized_oracle_resolve() {
        let env = Env::default();
        env.mock_all_auths();

        let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
        let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract(token_admin.clone());
        let token_address = token_contract;

        let treasury = Address::generate(&env);
        let not_oracle = Address::generate(&env);

        // Give them OPERATOR instead of ORACLE, they still shouldn't be able to call oracle_resolve
        ac_client.grant_role(&not_oracle, &ROLE_OPERATOR);
        client.init(&ac_id, &treasury, &0u32, &0u64);

        let pool_id = client.create_pool(
            &100000u64,
            &token_address,
            &3u32,
            &String::from_str(&env, "Test Pool"),
            &String::from_str(&env, "ipfs://metadata"),
        );

        env.ledger().with_mut(|li| li.timestamp = 100001);

        client.oracle_resolve(
            &not_oracle,
            &pool_id,
            &1u32,
            &String::from_str(&env, "proof_123"),
        );
    }

#[test]
fn test_admin_can_set_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.set_fee_bps(&admin, &500u32);
}

#[test]
fn test_admin_can_set_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let new_treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.set_treasury(&admin, &new_treasury);
}

// ── Pause tests ───────────────────────────────────────────────────────────────

#[test]
fn test_admin_can_pause_and_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.pause(&admin);
    client.unpause(&admin);
}

#[test]
#[should_panic(expected = "Unauthorized: missing required role")]
fn test_non_admin_cannot_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let not_admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.pause(&not_admin);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_set_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.pause(&admin);
    client.set_fee_bps(&admin, &100u32);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_set_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.pause(&admin);
    client.set_treasury(&admin, &Address::generate(&env));
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_create_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.pause(&admin);
    client.create_pool(
        &100000u64,
        &token,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_place_prediction() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.pause(&admin);
    client.place_prediction(&user, &0u64, &10, &1);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_resolve_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.pause(&admin);
    client.resolve_pool(&operator, &0u64, &1u32);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_claim_winnings() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.pause(&admin);
    client.claim_winnings(&user, &0u64);
}

#[test]
fn test_unpause_restores_functionality() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64);
    token_admin_client.mint(&user, &1000);

    client.pause(&admin);
    client.unpause(&admin);

    let pool_id = client.create_pool(
        &100000u64,
        &token_contract,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );
    client.place_prediction(&user, &pool_id, &10, &1);
}

// ── Pagination tests ──────────────────────────────────────────────────────────

#[test]
fn test_get_user_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let pool0 = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );
    let pool1 = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );
    let pool2 = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );

    client.place_prediction(&user, &pool0, &10, &1);
    client.place_prediction(&user, &pool1, &20, &2);
    client.place_prediction(&user, &pool2, &30, &1);

    let first_two = client.get_user_predictions(&user, &0, &2);
    assert_eq!(first_two.len(), 2);
    assert_eq!(first_two.get(0).unwrap().pool_id, pool0);
    assert_eq!(first_two.get(1).unwrap().pool_id, pool1);

    let last_two = client.get_user_predictions(&user, &1, &2);
    assert_eq!(last_two.len(), 2);
    assert_eq!(last_two.get(0).unwrap().pool_id, pool1);
    assert_eq!(last_two.get(1).unwrap().pool_id, pool2);

    let last_one = client.get_user_predictions(&user, &2, &1);
    assert_eq!(last_one.len(), 1);
    assert_eq!(last_one.get(0).unwrap().pool_id, pool2);

    let empty = client.get_user_predictions(&user, &3, &1);
    assert_eq!(empty.len(), 0);
}
// ── Pool cancellation tests ───────────────────────────────────────────────────
// ── Property‑based tests ─────────────────────────────────────────────────────

use proptest::prelude::*;

prop_compose! {
    fn stakes_and_outcomes()
        (len in 1..6usize)
        (pairs in prop::collection::vec((1i128..1000, 1u32..3), len)) -> Vec<(i128, u32)> {
            pairs
        }
}

proptest! {
    #[test]
    fn prop_payout_fee_consistency(pairs in stakes_and_outcomes(), fee_bps in 0u32..10001, winning_outcome in 1u32..3) {
        // compute totals
        let total_stake: i128 = pairs.iter().map(|(s, _)| *s).sum();
        let winning_stake: i128 = pairs.iter().filter(|(_, o)| *o == winning_outcome).map(|(s, _)| *s).sum();

        // calculate expected shares/fees using SafeMath
        let mut expected_payouts = vec![];
        let mut expected_fees = 0i128;
        for (stake, outcome) in &pairs {
            if *outcome == winning_outcome && winning_stake > 0 {
                let share = SafeMath::proportion(*stake, winning_stake, total_stake, RoundingMode::ProtocolFavor)?;
                let fee = SafeMath::percentage(share, fee_bps as i128, RoundingMode::ProtocolFavor)?;
                expected_fees = expected_fees.checked_add(fee).unwrap();
                expected_payouts.push(share.checked_sub(fee).unwrap());
            } else {
                expected_payouts.push(0);
            }
        }
        let expected_total_payout: i128 = expected_payouts.iter().sum();

        // now drive contract simulation
        let env = Env::default();
        env.mock_all_auths();
        let (ac_client, client, token_address, token, token_admin_client, treasury, operator) = setup(&env);
        let admin = Address::generate(&env);
        ac_client.grant_role(&admin, &ROLE_ADMIN);
        // make admin and set fee
        client
            .set_fee_bps(&admin, &fee_bps)
            .unwrap_or_else(|_| panic!("failed setting fee"));

        // create users and predictions
        let mut user_addrs = vec![];
        for _ in 0..pairs.len() {
            user_addrs.push(Address::generate(&env));
        }
        // mint tokens for each user
        for (i, (stake, _)) in pairs.iter().enumerate() {
            token_admin_client.mint(&user_addrs[i], stake);
        }
        let pool_id = client.create_pool(
            &100u64,
            &token_address,
            &String::from_str(&env, "Prop Pool"),
            &String::from_str(&env, "meta"),
        );
        for (i, (stake, outcome)) in pairs.iter().enumerate() {
            client.place_prediction(&user_addrs[i], &pool_id, stake, outcome);
        }
        // resolve
        env.ledger().with_mut(|li| li.timestamp = 101);
        client.resolve_pool(&operator, &pool_id, &winning_outcome).unwrap();

        let treasury_before = token.balance(&treasury);
        let mut actual_payouts = vec![];
        for addr in &user_addrs {
            let r = client.claim_winnings(addr, &pool_id);
            actual_payouts.push(r);
        }
        let treasury_after = token.balance(&treasury);
        let actual_total_payout: i128 = actual_payouts.iter().sum();
        let actual_total_fees = treasury_after.checked_sub(treasury_before).unwrap();

        // compare
        prop_assert_eq!(actual_total_payout, expected_total_payout);
        prop_assert_eq!(actual_total_fees, expected_fees);
        prop_assert!(actual_total_payout + actual_total_fees <= total_stake);
    }
}


// ── Pool Cancelation & State Guard Tests ────────────────────────────────────────
=======
#[test]
fn test_admin_can_cancel_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    let pool_id = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );

    // Admin should be able to cancel
    client.cancel_pool(&admin, &pool_id);
}

#[test]
fn test_pool_creator_can_cancel_unresolved_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let creator = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&creator, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    let pool_id = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );

    // Admin should be able to cancel their pool
    client.cancel_pool(&creator, &pool_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_non_admin_non_creator_cannot_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _) = setup(&env);

    let pool_id = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );

    let unauthorized = Address::generate(&env);
    // This should fail - user is not admin
    client.cancel_pool(&unauthorized, &pool_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_cannot_cancel_resolved_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    let pool_id = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );

    env.ledger().with_mut(|li| li.timestamp = 100001);
    client.resolve_pool(&operator, &pool_id, &1u32);

    // Now try to cancel - should fail
    client.cancel_pool(&admin, &pool_id);
}

#[test]
#[should_panic(expected = "Cannot place prediction on canceled pool")]
fn test_cannot_place_prediction_on_canceled_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    // Create and cancel pool
    let pool_id = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );

    // Cancel the pool
    client.cancel_pool(&admin, &pool_id);

    // Try to place prediction on canceled pool - should panic
    client.place_prediction(&user, &pool_id, &100, &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_pool_creator_cannot_cancel_after_admin_cancels() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    let pool_id = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );

    // Admin cancels the pool
    client.cancel_pool(&admin, &pool_id);

    // Attempt to cancel again should fail (already canceled)
    let non_admin = Address::generate(&env);
    client.cancel_pool(&non_admin, &pool_id);
}

#[test]
#[should_panic(expected = "Cannot place prediction on canceled pool")]
fn test_admin_can_cancel_pool_with_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let pool_id = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );

    // User places a prediction
    client.place_prediction(&user, &pool_id, &100, &1);

    // Admin cancels the pool - this freezes betting
    client.cancel_pool(&admin, &pool_id);

    // Verify no more predictions can be placed - should panic
    client.place_prediction(&user, &pool_id, &50, &2);
}
>>>>>>> 83d8c3331bdec8b6cfd33dc82b3bf301eeb9db57

#[test]
fn test_cancel_pool_refunds_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    let contract_addr = client.address.clone();
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
    );

    // User places a prediction
    client.place_prediction(&user1, &pool_id, &100, &1);
    assert_eq!(token_admin_client.balance(&contract_addr), 100);
    assert_eq!(token_admin_client.balance(&user1), 900);

    // Admin cancels the pool - this should enable refund of predictions
    client.cancel_pool(&admin, &pool_id);

    // Verify predictions are refunded (get_user_predictions should show the prediction still exists for potential refund claim)
    let predictions = client.get_user_predictions(&user1, &0u32, &10u32);
    assert_eq!(predictions.len(), 1);
}

#[test]
#[should_panic(expected = "Cannot resolve a canceled pool")]
fn test_cannot_resolve_canceled_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    let pool_id = client.create_pool(
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "ipfs://metadata"),
    );

    client.cancel_pool(&admin, &pool_id);
    // Should panic because pool is not active (canceled)
    client.resolve_pool(&operator, &pool_id, &1u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #81)")]
fn test_resolve_pool_before_delay() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    // Init with 3600s delay
    client.init(&ac_id, &treasury, &0u32, &3600u64);

    let end_time = 10000;
    let pool_id = client.create_pool(
        &end_time,
        &token,
        &2u32,
        &String::from_str(&env, "Delay Test"),
        &String::from_str(&env, "ipfs://metadata"),
    );

    // Set time to end_time + MIN_POOL_DURATION (to allow creation)
    // Wait, create_pool checks end_time > current_time + MIN_POOL_DURATION.
    // In setup, current_time is 0. So 10000 is fine.

    // Set time to end_time + 10s (less than delay)
    env.ledger().with_mut(|li| li.timestamp = end_time + 10);

    // Should panic with ResolutionDelayNotMet (81)
    client.resolve_pool(&operator, &pool_id, &1u32);
}

#[test]
fn test_resolve_pool_after_delay() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    // Init with 3600s delay
    client.init(&ac_id, &treasury, &0u32, &3600u64);

    let end_time = 10000;
    let pool_id = client.create_pool(
        &end_time,
        &token,
        &2u32,
        &String::from_str(&env, "Delay Test"),
        &String::from_str(&env, "ipfs://metadata"),
    );

    // Set time to end_time + 3601s (more than delay)
    env.ledger().with_mut(|li| li.timestamp = end_time + 3601);

    // Should succeed
    client.resolve_pool(&operator, &pool_id, &1u32);
}

#[test]
fn test_mark_pool_ready() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let treasury = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&ac_id, &treasury, &0u32, &3600u64);

    let end_time = 10000;
    let pool_id = client.create_pool(
        &end_time,
        &token,
        &2u32,
        &String::from_str(&env, "Ready Test"),
        &String::from_str(&env, "ipfs://metadata"),
    );

    // Test before delay
    env.ledger().with_mut(|li| li.timestamp = end_time + 10);
    let res = client.try_mark_pool_ready(&pool_id);
    assert!(res.is_err());

    // Test after delay
    env.ledger().with_mut(|li| li.timestamp = end_time + 3600);
    let res = client.try_mark_pool_ready(&pool_id);
    assert!(res.is_ok());
}
