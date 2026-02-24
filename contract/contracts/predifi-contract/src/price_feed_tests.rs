#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{symbol, testutils::Address as TestAddress, testutils::BytesN as TestBytesN};
    use crate::test_utils::{create_test_contract, create_test_pool, MockAccessControl};

    #[test]
    fn test_oracle_initialization() {
        let env = Env::default();
        let admin = TestAddress::generate(&env);
        let pyth_contract = TestAddress::generate(&env);
        let contract_id = create_test_contract(&env, &admin);

        // Test oracle initialization
        PredifiContract::init_oracle(
            env.clone(),
            admin.clone(),
            pyth_contract.clone(),
            300, // 5 minutes max price age
            100, // 1% min confidence ratio
        )
        .unwrap();

        // Verify oracle config
        let config = PredifiContract::get_oracle_config(env.clone()).unwrap();
        assert_eq!(config.pyth_contract, pyth_contract);
        assert_eq!(config.max_price_age, 300);
        assert_eq!(config.min_confidence_ratio, 100);
    }

    #[test]
    fn test_price_feed_update_and_retrieval() {
        let env = Env::default();
        let admin = TestAddress::generate(&env);
        let oracle = TestAddress::generate(&env);
        let contract_id = create_test_contract(&env, &admin);

        // Initialize oracle
        PredifiContract::init_oracle(env.clone(), admin.clone(), TestAddress::generate(&env), 300, 100).unwrap();

        // Set up oracle role
        let access_control = MockAccessControl::new(&env, &admin);
        access_control.grant_role(&oracle, 3); // Oracle role

        // Update price feed
        let feed_pair = symbol!("ETH/USD");
        let price = 3000_000000_i128; // $3000 with 6 decimals
        let confidence = 10_000_i128; // ±$0.01
        let timestamp = env.ledger().timestamp();
        let expires_at = timestamp + 60;

        PredifiContract::update_price_feed(
            env.clone(),
            oracle.clone(),
            feed_pair.clone(),
            price,
            confidence,
            timestamp,
            expires_at,
        )
        .unwrap();

        // Retrieve price feed
        let feed = PredifiContract::get_price_feed(env.clone(), feed_pair.clone()).unwrap();
        assert_eq!(feed.pair, feed_pair);
        assert_eq!(feed.price, price);
        assert_eq!(feed.confidence, confidence);
        assert_eq!(feed.timestamp, timestamp);
        assert_eq!(feed.expires_at, expires_at);
    }

    #[test]
    fn test_price_condition_setting_and_evaluation() {
        let env = Env::default();
        let admin = TestAddress::generate(&env);
        let operator = TestAddress::generate(&env);
        let contract_id = create_test_contract(&env, &admin);

        // Initialize oracle
        PredifiContract::init_oracle(env.clone(), admin.clone(), TestAddress::generate(&env), 300, 100).unwrap();

        // Set up operator role
        let access_control = MockAccessControl::new(&env, &admin);
        access_control.grant_role(&operator, 1); // Operator role

        // Create a test pool
        let pool_id = create_test_pool(&env, &admin);

        // Set price condition: ETH > $3000
        let feed_pair = symbol!("ETH/USD");
        let target_price = 3000_000000_i128;
        let operator_type = 1; // Greater than
        let tolerance_bps = 100; // 1%

        PredifiContract::set_price_condition(
            env.clone(),
            operator.clone(),
            pool_id,
            feed_pair.clone(),
            target_price,
            operator_type,
            tolerance_bps,
        )
        .unwrap();

        // Verify price condition
        let condition = PredifiContract::get_price_condition(env.clone(), pool_id).unwrap();
        assert_eq!(condition.feed_pair, feed_pair);
        assert_eq!(condition.target_price, target_price);
        assert_eq!(condition.operator, operator_type);
        assert_eq!(condition.tolerance_bps, tolerance_bps);
    }

    #[test]
    fn test_price_based_pool_resolution() {
        let env = Env::default();
        let admin = TestAddress::generate(&env);
        let operator = TestAddress::generate(&env);
        let oracle = TestAddress::generate(&env);
        let contract_id = create_test_contract(&env, &admin);

        // Initialize oracle
        PredifiContract::init_oracle(env.clone(), admin.clone(), TestAddress::generate(&env), 300, 100).unwrap();

        // Set up roles
        let access_control = MockAccessControl::new(&env, &admin);
        access_control.grant_role(&operator, 1); // Operator role
        access_control.grant_role(&oracle, 3); // Oracle role

        // Create a test pool
        let pool_id = create_test_pool(&env, &admin);

        // Set price condition: ETH > $3000
        let feed_pair = symbol!("ETH/USD");
        let target_price = 3000_000000_i128;
        PredifiContract::set_price_condition(
            env.clone(),
            operator.clone(),
            pool_id,
            feed_pair.clone(),
            target_price,
            1, // Greater than
            100, // 1% tolerance
        )
        .unwrap();

        // Update price feed with ETH at $3100 (condition should be met)
        let current_time = env.ledger().timestamp();
        PredifiContract::update_price_feed(
            env.clone(),
            oracle.clone(),
            feed_pair.clone(),
            3100_000000_i128, // $3100
            10_000_i128,
            current_time,
            current_time + 60,
        )
        .unwrap();

        // Fast forward time to meet resolution delay
        env.ledger().set_timestamp(current_time + 3600); // 1 hour later

        // Resolve pool from price
        PredifiContract::resolve_pool_from_price(env.clone(), oracle.clone(), pool_id).unwrap();

        // Verify pool is resolved with outcome 1 (condition met)
        let pool = PredifiContract::get_pool(env.clone(), pool_id).unwrap();
        assert_eq!(pool.state, MarketState::Resolved);
        assert_eq!(pool.outcome, 1);
    }

    #[test]
    fn test_batch_price_feed_updates() {
        let env = Env::default();
        let admin = TestAddress::generate(&env);
        let oracle = TestAddress::generate(&env);
        let contract_id = create_test_contract(&env, &admin);

        // Initialize oracle
        PredifiContract::init_oracle(env.clone(), admin.clone(), TestAddress::generate(&env), 300, 100).unwrap();

        // Set up oracle role
        let access_control = MockAccessControl::new(&env, &admin);
        access_control.grant_role(&oracle, 3); // Oracle role

        // Create batch updates
        let mut updates = Vec::new(&env);
        let current_time = env.ledger().timestamp();

        updates.push_back((
            symbol!("ETH/USD"),
            3000_000000_i128,
            10_000_i128,
            current_time,
            current_time + 60,
        ));
        updates.push_back((
            symbol!("BTC/USD"),
            60000_000000_i128,
            100_000_i128,
            current_time,
            current_time + 60,
        ));

        // Batch update
        PredifiContract::batch_update_price_feeds(env.clone(), oracle.clone(), updates).unwrap();

        // Verify both feeds are updated
        let eth_feed = PredifiContract::get_price_feed(env.clone(), symbol!("ETH/USD")).unwrap();
        assert_eq!(eth_feed.price, 3000_000000_i128);

        let btc_feed = PredifiContract::get_price_feed(env.clone(), symbol!("BTC/USD")).unwrap();
        assert_eq!(btc_feed.price, 60000_000000_i128);
    }

    #[test]
    fn test_price_validation() {
        let env = Env::default();
        let admin = TestAddress::generate(&env);
        let oracle = TestAddress::generate(&env);
        let contract_id = create_test_contract(&env, &admin);

        // Initialize oracle with strict validation
        PredifiContract::init_oracle(env.clone(), admin.clone(), TestAddress::generate(&env), 60, 50).unwrap();

        // Set up oracle role
        let access_control = MockAccessControl::new(&env, &admin);
        access_control.grant_role(&oracle, 3); // Oracle role

        let current_time = env.ledger().timestamp();

        // Test 1: Expired price data should be invalid
        PredifiContract::update_price_feed(
            env.clone(),
            oracle.clone(),
            symbol!("ETH/USD"),
            3000_000000_i128,
            10_000_i128,
            current_time,
            current_time + 30, // Expires in 30 seconds
        )
        .unwrap();

        // Fast forward past expiration
        env.ledger().set_timestamp(current_time + 120); // 2 minutes later

        // Try to resolve with expired data - should fail
        let pool_id = create_test_pool(&env, &admin);
        PredifiContract::set_price_condition(
            env.clone(),
            admin.clone(),
            pool_id,
            symbol!("ETH/USD"),
            3000_000000_i128,
            1,
            100,
        )
        .unwrap();

        let result = PredifiContract::resolve_pool_from_price(env.clone(), oracle.clone(), pool_id);
        assert!(result.is_err());
        assert_eq!(result.err(), Some(PredifiError::PriceDataInvalid));
    }

    #[test]
    fn test_authorization_checks() {
        let env = Env::default();
        let admin = TestAddress::generate(&env);
        let unauthorized_user = TestAddress::generate(&env);
        let contract_id = create_test_contract(&env, &admin);

        // Initialize oracle
        PredifiContract::init_oracle(env.clone(), admin.clone(), TestAddress::generate(&env), 300, 100).unwrap();

        // Test unauthorized oracle update
        let result = PredifiContract::update_price_feed(
            env.clone(),
            unauthorized_user.clone(),
            symbol!("ETH/USD"),
            3000_000000_i128,
            10_000_i128,
            env.ledger().timestamp(),
            env.ledger().timestamp() + 60,
        );
        assert!(result.is_err());
        assert_eq!(result.err(), Some(PredifiError::Unauthorized));

        // Test unauthorized oracle initialization
        let result = PredifiContract::init_oracle(
            env.clone(),
            unauthorized_user.clone(),
            TestAddress::generate(&env),
            300,
            100,
        );
        assert!(result.is_err());
        assert_eq!(result.err(), Some(PredifiError::Unauthorized));
    }

    #[test]
    fn test_price_condition_operators() {
        let env = Env::default();
        let admin = TestAddress::generate(&env);
        let oracle = TestAddress::generate(&env);
        let contract_id = create_test_contract(&env, &admin);

        // Initialize oracle
        PredifiContract::init_oracle(env.clone(), admin.clone(), TestAddress::generate(&env), 300, 100).unwrap();

        // Set up roles
        let access_control = MockAccessControl::new(&env, &admin);
        access_control.grant_role(&admin, 1); // Operator role
        access_control.grant_role(&oracle, 3); // Oracle role

        let current_time = env.ledger().timestamp();
        let feed_pair = symbol!("ETH/USD");
        let target_price = 3000_000000_i128;

        // Test equal operator (0) - should resolve to outcome 1 when price equals target
        let pool_id1 = create_test_pool(&env, &admin);
        PredifiContract::set_price_condition(
            env.clone(),
            admin.clone(),
            pool_id1,
            feed_pair.clone(),
            target_price,
            0, // Equal
            100, // 1% tolerance
        )
        .unwrap();

        PredifiContract::update_price_feed(
            env.clone(),
            oracle.clone(),
            feed_pair.clone(),
            3000_000000_i128, // Exactly equal
            10_000_i128,
            current_time,
            current_time + 60,
        )
        .unwrap();

        env.ledger().set_timestamp(current_time + 3600);
        PredifiContract::resolve_pool_from_price(env.clone(), oracle.clone(), pool_id1).unwrap();
        let pool1 = PredifiContract::get_pool(env.clone(), pool_id1).unwrap();
        assert_eq!(pool1.outcome, 1); // Condition met

        // Test less than operator (2) - should resolve to outcome 1 when price is less
        let pool_id2 = create_test_pool(&env, &admin);
        PredifiContract::set_price_condition(
            env.clone(),
            admin.clone(),
            pool_id2,
            feed_pair.clone(),
            target_price,
            2, // Less than
            100,
        )
        .unwrap();

        PredifiContract::update_price_feed(
            env.clone(),
            oracle.clone(),
            feed_pair.clone(),
            2900_000000_i128, // Less than target
            10_000_i128,
            current_time,
            current_time + 60,
        )
        .unwrap();

        PredifiContract::resolve_pool_from_price(env.clone(), oracle.clone(), pool_id2).unwrap();
        let pool2 = PredifiContract::get_pool(env.clone(), pool_id2).unwrap();
        assert_eq!(pool2.outcome, 1); // Condition met
    }

    #[test]
    fn test_error_conditions() {
        let env = Env::default();
        let admin = TestAddress::generate(&env);
        let contract_id = create_test_contract(&env, &admin);

        // Test getting oracle config before initialization
        let config = PredifiContract::get_oracle_config(env.clone());
        assert!(config.is_none());

        // Test getting price feed for non-existent pair
        let feed = PredifiContract::get_price_feed(env.clone(), symbol!("NONEXISTENT"));
        assert!(feed.is_none());

        // Test getting price condition for non-existent pool
        let condition = PredifiContract::get_price_condition(env.clone(), 999);
        assert!(condition.is_none());

        // Initialize oracle
        PredifiContract::init_oracle(env.clone(), admin.clone(), TestAddress::generate(&env), 300, 100).unwrap();

        // Test resolving pool without price condition
        let pool_id = create_test_pool(&env, &admin);
        let oracle = TestAddress::generate(&env);
        let access_control = MockAccessControl::new(&env, &admin);
        access_control.grant_role(&oracle, 3);

        let result = PredifiContract::resolve_pool_from_price(env.clone(), oracle.clone(), pool_id);
        assert!(result.is_err());
        assert_eq!(result.err(), Some(PredifiError::PriceConditionNotSet));
    }
}
