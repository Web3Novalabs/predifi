use crate::{PoolConfig, PredifiContract, PredifiContractClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

extern crate alloc;

mod dummy_access_control {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct DummyAccessControl;

    #[contractimpl]
    impl DummyAccessControl {
        pub fn grant_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().set(&key, &true);
        }

        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }
    }
}

const ROLE_ADMIN: u32 = 0;
const ROLE_OPERATOR: u32 = 1;

/// Helper to setup a test environment for stress testing.
fn stress_setup(
    env: &Env,
) -> (
    PredifiContractClient<'_>,
    Address,
    token::Client<'_>,
    token::StellarAssetClient<'_>,
) {
    env.mock_all_auths();

    // Set protocol version and info
    env.ledger().with_mut(|li| {
        li.protocol_version = 23;
        li.timestamp = 1000;
    });

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(env, &ac_id);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let treasury = Address::generate(env);

    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&admin, &ROLE_OPERATOR); // Grant operator too for convenience
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    client.init(&ac_id, &treasury, &500, &3600);

    // Setup Token
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let token_client = token::Client::new(env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    // Whitelist the token
    client.add_token_to_whitelist(&admin, &token_id);

    (client, admin, token_client, token_admin_client)
}

#[test]
fn test_high_volume_predictions_single_pool() {
    let env = Env::default();
    let (client, _admin, token_client, token_admin_client) = stress_setup(&env);

    let creator = Address::generate(&env);
    let num_users = 100;
    let stake_per_user = 1000;

    // Create pool with min_stake=10, max_stake=10000
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        // 10000 > 1000 + 3600
        &token_client.address,
        &2,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "High Volume Stress Test"),
            metadata_url: String::from_str(&env, "ipfs://stress"),
            min_stake: 10i128,
            max_stake: 10000i128,
            initial_liquidity: 0,
            required_resolutions: 1u32,
        },
    );

    // Place 100 predictions from 100 unique users
    for i in 0..num_users {
        let user = Address::generate(&env);
        token_admin_client.mint(&user, &stake_per_user);

        // Split users between outcome 0 and 1
        let outcome = i % 2;
        client.place_prediction(&user, &pool_id, &stake_per_user, &outcome, &None);
    }

    // Use client to get details instead of direct storage access
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.total_stake, (num_users as i128) * stake_per_user);
}

#[test]
fn test_bulk_claim_winnings() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client) = stress_setup(&env);

    let creator = Address::generate(&env);
    let num_users = 48; // Using 48 to avoid rounding dust (48 * 1000 = 480000)
    let stake_per_user = 1000;

    let pool_id = client.create_pool(
        &creator,
        &5000u64,
        // 5000 > 1000 + 3600
        &token_client.address,
        &2,
        &symbol_short!("Finance"),
        &PoolConfig {
            description: String::from_str(&env, "Bulk Claim Test"),
            metadata_url: String::from_str(&env, "ipfs://bulk"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0,
            required_resolutions: 1u32,
        },
    );

    let mut users = alloc::vec::Vec::new();
    for _ in 0..num_users {
        let user = Address::generate(&env);
        token_admin_client.mint(&user, &stake_per_user);
        client.place_prediction(&user, &pool_id, &stake_per_user, &0, &None); // All on outcome 0
        users.push(user);
    }

    // Advance time to allow resolution
    env.ledger().with_mut(|li| li.timestamp = 10000); // 10000 > 5000 + 3600

    // Resolve as outcome 0
    client.resolve_pool(&admin, &pool_id, &0);

    // Bulk claim
    for user in users {
        let winnings = client.claim_winnings(&user, &pool_id);
        assert!(winnings > 0);
    }
}

#[test]
fn test_sequential_pool_creation_stress() {
    let env = Env::default();
    let (client, _, token_client, _) = stress_setup(&env);

    let creator = Address::generate(&env);
    let num_pools = 50;

    for i in 0..num_pools {
        let pool_id = client.create_pool(
            &creator,
            &200000u64,
            &token_client.address,
            &2,
            &symbol_short!("Other"),
            &PoolConfig {
                description: String::from_str(&env, "Stress Pool"),
                metadata_url: String::from_str(&env, "ipfs://meta"),
                min_stake: 1i128,
                max_stake: 0i128,
                initial_liquidity: 0,
                required_resolutions: 1u32,
            },
        );
        assert_eq!(pool_id, i as u64);
    }
}

#[test]
fn test_max_outcomes_high_volume() {
    let env = Env::default();
    let (client, _, token_client, token_admin_client) = stress_setup(&env);

    let creator = Address::generate(&env);
    let max_options = 16; // Using 16 as a reasonable "high" value that is common

    let pool_id = client.create_pool(
        &creator,
        &200000u64,
        &token_client.address,
        &max_options,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "High Options Test"),
            metadata_url: String::from_str(&env, "ipfs://meta"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0,
            required_resolutions: 1u32,
        },
    );

    // Place predictions on each outcome
    for i in 0..max_options {
        let user = Address::generate(&env);
        token_admin_client.mint(&user, &1000);
        client.place_prediction(&user, &pool_id, &1000, &i, &None);
    }

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.options_count, max_options);
}

#[test]
fn test_prediction_throughput_measurement() {
    let env = Env::default();
    let (client, _, token_client, token_admin_client) = stress_setup(&env);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &200000u64,
        &token_client.address,
        &2,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Throughput Test"),
            metadata_url: String::from_str(&env, "ipfs://meta"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0,
            required_resolutions: 1u32,
        },
    );

    let start_ledger = env.ledger().timestamp();
    let num_predictions = 50;

    for _ in 0..num_predictions {
        let user = Address::generate(&env);
        token_admin_client.mint(&user, &100);
        client.place_prediction(&user, &pool_id, &100, &0, &None);
    }

    let end_ledger = env.ledger().timestamp();
    // In mock env, timestamp doesn't advance unless we do it.
    // This test primarily verifies that 50 consecutive predictions succeed.
    assert!(end_ledger >= start_ledger);
}

#[test]
fn test_resolution_under_load() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client) = stress_setup(&env);

    let creator = Address::generate(&env);
    let num_pools = 10;
    let mut pool_ids = alloc::vec::Vec::new();

    for _ in 0..num_pools {
        let pid = client.create_pool(
            &creator,
            &20000u64,
            &token_client.address,
            &2,
            &symbol_short!("Other"),
            &PoolConfig {
                description: String::from_str(&env, "Load Pool"),
                metadata_url: String::from_str(&env, "ipfs://load"),
                min_stake: 1i128,
                max_stake: 0i128,
                initial_liquidity: 0,
                required_resolutions: 1u32,
            },
        );
        pool_ids.push(pid);
    }

    // Place predictions on all pools
    for &pid in &pool_ids {
        let user = Address::generate(&env);
        token_admin_client.mint(&user, &1000);
        client.place_prediction(&user, &pid, &1000, &0, &None);
    }

    // Advance time
    env.ledger().with_mut(|li| li.timestamp = 30000);

    // Resolve all pools
    for pid in pool_ids {
        client.resolve_pool(&admin, &pid, &0);
        let pool = client.get_pool(&pid);
        assert!(pool.resolved);
    }
}
