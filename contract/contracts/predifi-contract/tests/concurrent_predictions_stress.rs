#![cfg(test)]

use predifi_contract::{Contract, Pool, Prediction, PredictionOutcome};
use predifi_errors::PrediFiError;

#[test]
fn test_1000_concurrent_predictions_single_pool() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("BTC/USD".to_string(), 1000000, 100000, 100).expect("Should create pool");

    // Place 1000 predictions
    let mut users = Vec::new();
    for i in 0..1000 {
        let user = format!("user_{}", i);
        users.push(user.clone());
        let result = contract.place_prediction(user, 100, if i % 2 == 0 { "UP" } else { "DOWN" }.to_string());
        assert!(result.is_ok(), "Prediction {} should succeed", i);
    }

    // Verify all predictions were recorded
    let total_predictions = contract.get_pool_prediction_count("BTC/USD").expect("Should get count");
    assert_eq!(total_predictions, 1000);
}

#[test]
fn test_gas_consumption_scales_linearly_with_predictions() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("ETH/USD".to_string(), 5000000, 500000, 100).expect("Should create pool");

    let mut gas_measurements = Vec::new();

    for batch_size in [100, 250, 500, 1000] {
        let initial_gas = contract.get_gas_used();

        for i in 0..batch_size {
            let user = format!("user_batch_{}", i);
            contract.place_prediction(user, 50, "UP".to_string()).expect("Should place");
        }

        let final_gas = contract.get_gas_used();
        let gas_consumed = final_gas - initial_gas;
        gas_measurements.push((batch_size, gas_consumed));
    }

    // Verify roughly linear scaling (allow 10% variance)
    for i in 1..gas_measurements.len() {
        let (prev_count, prev_gas) = gas_measurements[i - 1];
        let (curr_count, curr_gas) = gas_measurements[i];

        let prev_avg_gas = prev_gas / prev_count;
        let curr_avg_gas = curr_gas / curr_count;
        let variance = ((curr_avg_gas - prev_avg_gas).abs() as f64) / prev_avg_gas as f64;

        assert!(variance < 0.1, "Gas usage should scale linearly, variance: {}", variance);
    }
}

#[test]
fn test_payout_accuracy_with_many_participants() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("EUR/USD".to_string(), 1000000, 100000, 200).expect("Should create pool");

    // 500 users betting UP, 500 betting DOWN with varying amounts
    let mut total_up_stake = 0u128;
    let mut total_down_stake = 0u128;

    for i in 0..500 {
        let amount = 100 + (i % 50) * 10; // Varying amounts

        let user_up = format!("up_user_{}", i);
        contract.place_prediction(user_up, amount, "UP".to_string()).expect("Should place UP");
        total_up_stake += amount;

        let user_down = format!("down_user_{}", i);
        contract.place_prediction(user_down, amount, "DOWN".to_string()).expect("Should place DOWN");
        total_down_stake += amount;
    }

    // Resolve pool with UP outcome
    let result = contract.resolve_pool("EUR/USD".to_string(), "UP".to_string()).expect("Should resolve");

    // Verify payout accuracy
    let expected_payout_per_up_user = (total_up_stake + total_down_stake) / 500;
    let actual_payouts = contract.get_pool_payouts("EUR/USD").expect("Should get payouts");

    for (i, payout) in actual_payouts.iter().enumerate() {
        let tolerance = expected_payout_per_up_user / 20; // 5% tolerance
        assert!(((*payout as i128) - (expected_payout_per_up_user as i128)).abs() <= (tolerance as i128));
    }
}

#[test]
fn test_concurrent_deposits_with_predictions() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("GBP/USD".to_string(), 10000000, 1000000, 100).expect("Should create pool");

    let initial_balance = pool.get_balance();

    // Concurrent deposits and predictions
    for i in 0..200 {
        let user = format!("user_{}", i);

        // Deposit
        contract.deposit(user.clone(), 5000).expect("Should deposit");

        // Place prediction
        contract.place_prediction(user, 1000, if i % 2 == 0 { "UP" } else { "DOWN" }.to_string()).expect("Should place");
    }

    let final_balance = pool.get_balance();
    let expected_deposits = 200 * 5000;
    assert!(final_balance >= initial_balance + expected_deposits);
}

#[test]
fn test_1000_predictions_state_consistency() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("GOLD/USD".to_string(), 2000000, 200000, 100).expect("Should create pool");

    // Place 1000 predictions
    for i in 0..1000 {
        let user = format!("user_{}", i);
        contract.place_prediction(user, 100, "UP".to_string()).expect("Should place");
    }

    // Verify state consistency
    let pool_state = contract.get_pool_state("GOLD/USD").expect("Should get state");
    assert_eq!(pool_state.prediction_count, 1000);
    assert_eq!(pool_state.total_staked, 100000); // 1000 * 100
    assert!(!pool_state.has_errors);
}

#[test]
fn test_prediction_ordering_with_high_volume() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("SPX/USD".to_string(), 5000000, 500000, 100).expect("Should create pool");

    let mut prediction_ids = Vec::new();

    // Place 500 predictions rapidly
    for i in 0..500 {
        let user = format!("rapid_user_{}", i);
        let pred_result = contract.place_prediction(user, 50, "DOWN".to_string()).expect("Should place");
        prediction_ids.push(pred_result.id);
    }

    // Verify ordering matches insertion order
    for (i, id) in prediction_ids.iter().enumerate() {
        let prediction = contract.get_prediction(*id).expect("Should get prediction");
        assert_eq!(prediction.sequence_number, i as u64);
    }
}

#[test]
fn test_refund_processing_with_many_predictions() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("OIL/USD".to_string(), 3000000, 300000, 100).expect("Should create pool");

    let mut prediction_ids = Vec::new();

    // Place 100 predictions
    for i in 0..100 {
        let user = format!("refund_user_{}", i);
        let pred = contract.place_prediction(user, 200, "UP".to_string()).expect("Should place");
        prediction_ids.push(pred.id);
    }

    // Request refunds for all
    for id in prediction_ids {
        let result = contract.request_refund("admin".to_string(), id);
        assert!(result.is_ok(), "Refund for {} should succeed", id);
    }

    // Verify all refunded
    let remaining = contract.get_active_predictions().len();
    assert_eq!(remaining, 0);
}

#[test]
fn test_quadratic_complexity_check() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("CRYPTO/USD".to_string(), 10000000, 1000000, 100).expect("Should create pool");

    let mut timings = Vec::new();

    for size in [100, 250, 500] {
        let start_time = std::time::Instant::now();

        for i in 0..size {
            let user = format!("perf_user_{}", i);
            contract.place_prediction(user, 100, "UP".to_string()).expect("Should place");
        }

        let elapsed = start_time.elapsed().as_micros();
        timings.push((size, elapsed));
    }

    // Check for O(n) scaling, not O(n²)
    // If O(n²), time would quadruple when input doubles
    let (size1, time1) = timings[0];
    let (size2, time2) = timings[1];

    let time_ratio = time2 as f64 / time1 as f64;
    let size_ratio = size2 as f64 / size1 as f64;

    // For O(n), time_ratio should be close to size_ratio
    // For O(n²), time_ratio would be size_ratio²
    assert!(time_ratio < size_ratio * 2.0, "Potential O(n²) complexity detected");
}

#[test]
fn test_large_batch_within_gas_limits() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("INDICES/USD".to_string(), 20000000, 2000000, 100).expect("Should create pool");

    let gas_limit = 30_000_000u64; // Typical block gas limit
    let initial_gas = contract.get_gas_used();

    // Place as many predictions as possible within gas limit
    let mut count = 0;
    for i in 0..2000 {
        let user = format!("gas_user_{}", i);
        match contract.place_prediction(user, 100, "UP".to_string()) {
            Ok(_) => {
                count += 1;
                let current_gas = contract.get_gas_used() - initial_gas;
                if current_gas > gas_limit {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    assert!(count >= 100, "Should be able to process at least 100 predictions within gas limit");
}
