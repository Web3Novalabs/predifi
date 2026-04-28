#![cfg(test)]

use crate::test::ROLE_ADMIN;
use crate::{FeeTier, PoolConfig};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, Env, String, Vec,
};

#[test]
fn test_dynamic_fee_tiers_application() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        ac_client,
        client,
        token_address,
        _token,
        token_admin_client,
        _treasury,
        operator,
        creator,
    ) = crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Set global fee to 3%
    client.set_fee_bps(&admin, &300u32);

    // Set up fee tiers
    // Threshold 1M (1,000,000 * 10^7) -> 1% (100 bps)
    // Threshold 5M (5,000,000 * 10^7) -> 0.5% (50 bps)
    let tiers = Vec::from_array(
        &env,
        [
            FeeTier {
                stake_threshold: 1_000_000 * 10_000_000,
                fee_bps: 100,
            },
            FeeTier {
                stake_threshold: 5_000_000 * 10_000_000,
                fee_bps: 50,
            },
        ],
    );

    client.set_fee_tiers(&admin, &tiers);

    // 1. Create a pool with low volume (below 1M)
    let end_time_1 = env.ledger().timestamp() + 10000;
    let pool_id = client.create_pool(
        &creator,
        &end_time_1,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Low Volume Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "A"),
                String::from_str(&env, "B"),
            ],
        },
    );

    // Resolve it
    env.ledger().set_timestamp(end_time_1 + 1000);
    client.resolve_pool(&operator, &pool_id, &0);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.fee_bps, 300); // Should use global default (300)

    // 2. Create a pool with medium volume (2M)
    let end_time_2 = env.ledger().timestamp() + 10000;
    let pool_id_2 = client.create_pool(
        &creator,
        &end_time_2,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Med Volume Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "A"),
                String::from_str(&env, "B"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &(2_000_000 * 10_000_000));
    client.place_prediction(
        &user,
        &pool_id_2,
        &(2_000_000 * 10_000_000),
        &0,
        &None,
        &None,
    );

    env.ledger().set_timestamp(end_time_2 + 1000);
    client.resolve_pool(&operator, &pool_id_2, &0);

    let pool2 = client.get_pool(&pool_id_2);
    assert_eq!(pool2.fee_bps, 100); // Should apply 1% tier

    // 3. Create a pool with high volume (6M)
    let end_time_3 = env.ledger().timestamp() + 10000;
    let pool_id_3 = client.create_pool(
        &creator,
        &end_time_3,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "High Volume Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "A"),
                String::from_str(&env, "B"),
            ],
        },
    );
    token_admin_client.mint(&user, &(6_000_000 * 10_000_000));
    client.place_prediction(
        &user,
        &pool_id_3,
        &(6_000_000 * 10_000_000),
        &0,
        &None,
        &None,
    );

    env.ledger().set_timestamp(end_time_3 + 1000);
    client.resolve_pool(&operator, &pool_id_3, &0);

    let pool3 = client.get_pool(&pool_id_3);
    assert_eq!(pool3.fee_bps, 50); // Should apply 0.5% tier
}

#[test]
fn test_set_fee_tiers_unsorted_thresholds() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _, _, _, _, _, _) = crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let tiers = Vec::from_array(
        &env,
        [
            FeeTier { stake_threshold: 5_000_000, fee_bps: 50 },
            FeeTier { stake_threshold: 1_000_000, fee_bps: 100 }, // out of order
        ],
    );

    let result = client.try_set_fee_tiers(&admin, &tiers);
    assert_eq!(
        result.err().unwrap().unwrap(),
        crate::PredifiError::InvalidFeeBps
    );
}

#[test]
fn test_set_fee_tiers_duplicate_thresholds() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, _, _, _, _, _, _) = crate::test::setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let tiers = Vec::from_array(
        &env,
        [
            FeeTier { stake_threshold: 1_000_000, fee_bps: 100 },
            FeeTier { stake_threshold: 1_000_000, fee_bps: 50 }, // duplicate threshold
        ],
    );

    let result = client.try_set_fee_tiers(&admin, &tiers);
    assert_eq!(
        result.err().unwrap().unwrap(),
        crate::PredifiError::InvalidFeeBps
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_set_fee_tiers_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _, _, _) = crate::test::setup(&env);
    let user = Address::generate(&env);
    let tiers = Vec::new(&env);

    client.set_fee_tiers(&user, &tiers);
}
