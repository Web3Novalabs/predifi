#![cfg(test)]

use predifi_contract::{MarketState, PoolConfig, PredifiContract, PredifiContractClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

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

#[test]
fn test_price_based_pool_mock_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    // 1. Setup Contracts & Identities
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let operator = Address::generate(&env);
    let creator = Address::generate(&env);
    let treasury = Address::generate(&env);

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    // Initializing the contract
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    // Setup Token and Whitelist Category/Token
    let token_address = Address::generate(&env);
    client.add_token_to_whitelist(&admin, &token_address);
    // Note: If validate_category checks storage, we might need to register it.
    // However, if the contract doesn't have an add_category method, we assume it's pre-registered or validation is mocked.
    // For now, let's assume the contract handles it via mock or it just works if we don't have an interface to add.

    // 2. Create a Prediction Pool
    let end_time = 4000u64; // > min_pool_duration (3600)
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_address,
        &2u32, // 2 outcomes: 0 (No), 1 (Yes)
        &symbol_short!("Crypto"),
        &PoolConfig {
            description: String::from_str(&env, "Will ETH > $4000?"),
            metadata_url: String::from_str(&env, "ipfs://..."),
            min_stake: 100,
            max_stake: 0,
            max_total_stake: 0,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
        },
    );

    // 3. Set Price Condition (ETH > $4000)
    let asset = symbol_short!("ETH_USD");
    let target_price = 4000_0000000i128; // 7 decimals

    client.set_price_condition(
        &operator,
        &pool_id,
        &asset,
        &target_price,
        &1u32,   // ComparisonOp::GreaterThan
        &100u32, // 1% tolerance
    );

    // 4. Mock the PriceFeed update
    let current_time = env.ledger().timestamp();
    let mock_price = 4100_0000000i128; // ETH is now $4100

    client.update_price_feed(
        &oracle,
        &asset,
        &mock_price,
        &100i128, // confidence
        &current_time,
        &(current_time + 10000), // expires at
    );

    // 5. Verify Resolution logic
    // Fast forward past end_time (4000) and resolution_delay (0)
    env.ledger().with_mut(|li| li.timestamp = 5000);

    client.resolve_pool_from_price(&pool_id);

    // 6. Assert Result
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 1); // "Yes" outcome wins as 4100 > 4000
}
