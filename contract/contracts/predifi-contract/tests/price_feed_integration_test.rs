#![cfg(test)]

use predifi_contract::{MarketState, PoolConfig, PredifiContract, PredifiContractClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    Address, Env, String, Symbol,
};

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

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    // Initializing the contract (assuming standard init from lib.rs)
    // Note: In a real integration test, we would also register/setup AccessControl
    client.init(&admin, &treasury, &0u32, &0u64, &3600u64);

    // 2. Create a Prediction Pool
    let end_time = 1000u64;
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &Address::generate(&env), // Mock token address
        &2u32,                    // 2 outcomes: 0 (No), 1 (Yes)
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
    // Field names: asset, target_price, compare_op
    let asset = symbol_short!("ETH-USD");
    let target_price = 4000_0000000i128; // 7 decimals

    // We use the available set_price_condition method
    client.set_price_condition(
        &operator,
        &pool_id,
        &asset,
        &target_price,
        &1u32,   // ComparisonOp::GreaterThan
        &100u32, // 1% tolerance
    );

    // 4. Mock the PriceFeed update (Setting a fixed price in Env)
    let current_time = env.ledger().timestamp();
    let mock_price = 4100_0000000i128; // ETH is now $4100

    client.update_price_feed(
        &oracle,
        &asset,
        &mock_price,
        &100i128, // confidence
        &current_time,
        &(current_time + 3600), // expires at
    );

    // 5. Verify Resolution logic
    // Fast forward past end_time and resolution_delay
    env.ledger().with_mut(|li| li.timestamp = 2000);

    client.resolve_pool_from_price(&pool_id);

    // 6. Assert Result
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 1); // "Yes" outcome wins as 4100 > 4000
}
