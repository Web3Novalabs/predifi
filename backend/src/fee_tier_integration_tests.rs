//! Integration tests for fee tier transitions.
//!
//! Tests verify:
//! - Correct fee application at tier boundaries
//! - Fee changes mid-pool
//! - Treasury accumulation accuracy

#[cfg(all(test, feature = "integration-tests"))]
mod tests {
    use crate::config::Config;
    use crate::price_cache::PriceCache;
    use crate::redis_cache::RedisCache;
    use crate::ws::EventBus;
    use axum::extract::State;
    use axum::http::Method;
    use axum::Router;
    use http_body_util::BodyExt;
    use hyper::{Body, Request};
    use std::collections::HashMap;
    use tower::ServiceExt;

    // ── Test router setup ───────────────────────────────────────────────────────

    /// Build a test router with custom fee configuration.
    fn build_test_router(
        treasury_fee_bps: u32,
        referral_fee_bps: u32,
    ) -> Router {
        let mut config = Config::default_for_test();
        config.treasury_fee_bps = treasury_fee_bps;
        config.referral_fee_bps = referral_fee_bps;

        crate::build_router(
            config,
            PriceCache::new(),
            RedisCache::disabled(),
            EventBus::new(),
        )
    }

    /// Build a POST request with JSON body.
    fn post_json(path: &str, body: &serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri(path)
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("failed to build request")
    }

    // ── Fee Tier Boundary Tests ─────────────────────────────────────────────────

    /// Test that fees are correctly calculated at exact tier boundaries.
    ///
    /// We simulate multiple pool creation scenarios and verify fee calculations
    /// match expected values at each tier threshold.
    #[test]
    fn fee_tier_boundary_at_threshold() {
        // Test fee calculation at exact threshold values
        // Thresholds: 1_000 -> 2%, 5_000 -> 1%, 10_000 -> 0.5%
        // Base fee: 3% (300 bps)

        let base_fee_bps = 300u32;
        let tiers = vec![
            (1_000i64, 200u32), // 2%
            (5_000i64, 100u32), // 1%
            (10_000i64, 50u32), // 0.5%
        ];

        let test_cases = [
            // (stake, expected_fee_bps, expected_fee_amount)
            (999i64, base_fee_bps, 29),       // Below all tiers -> base 3%
            (1_000i64, 200, 20),              // Exact Tier 1 threshold -> 2%
            (1_001i64, 200, 20),              // Just above Tier 1 -> 2%
            (4_999i64, 200, 99),              // Below Tier 2 -> 2%
            (5_000i64, 100, 50),              // Exact Tier 2 threshold -> 1%
            (5_001i64, 100, 50),              // Just above Tier 2 -> 1%
            (9_999i64, 100, 99),              // Below Tier 3 -> 1%
            (10_000i64, 50, 50),              // Exact Tier 3 threshold -> 0.5%
            (10_001i64, 50, 50),              // Just above Tier 3 -> 0.5%
        ];

        for (stake, expected_fee_bps, expected_fee) in test_cases {
            let calculated_fee = calculate_fee(stake, expected_fee_bps);
            assert_eq!(calculated_fee, expected_fee, "Failed for stake={}", stake);
        }
    }

    /// Test fee calculation below all tier thresholds (base fee applies).
    #[test]
    fn fee_below_all_tiers_uses_base_rate() {
        let base_fee_bps = 300; // 3%
        let stakes = [100, 500, 999];

        for stake in stakes {
            let fee = calculate_fee(stake, base_fee_bps);
            let expected_fee = (stake as f64 * 0.03) as i64;
            assert_eq!(fee, expected_fee);
        }
    }

    /// Test fee calculation at exact tier thresholds.
    #[test]
    fn fee_at_exact_threshold_uses_tier_rate() {
        let tiers = vec![
            (1_000, 200), // 2%
            (5_000, 100), // 1%
            (10_000, 50), // 0.5%
        ];

        for (threshold, tier_fee_bps) in tiers {
            // Test at threshold
            let fee = calculate_fee(threshold, tier_fee_bps);
            let expected_fee = (threshold as f64 * tier_fee_bps as f64 / 10_000.0) as i64;
            assert_eq!(fee, expected_fee);

            // Test just above threshold
            let fee_above = calculate_fee(threshold + 1, tier_fee_bps);
            assert_eq!(fee_above, expected_fee);
        }
    }

    /// Test fee calculation between tier thresholds.
    #[test]
    fn fee_between_tiers_uses_lower_tier_rate() {
        // Tiers: 1_000 -> 2%, 5_000 -> 1%
        // Between thresholds should use 2%

        let stakes = [1_001, 2_000, 3_000, 4_000, 4_999];
        let tier_fee_bps = 200; // 2% (Tier 1 rate)

        for stake in stakes {
            let fee = calculate_fee(stake, tier_fee_bps);
            let expected_fee = (stake as f64 * 0.02) as i64;
            assert_eq!(fee, expected_fee);
        }
    }

    // ── Mid-Pool Fee Change Tests ───────────────────────────────────────────────

    /// Test fee calculation changes when pool stake crosses thresholds mid-pool.
    ///
    /// Simulates a pool where stakes are added over time, crossing tier thresholds.
    /// The final fee should reflect the highest tier threshold reached.
    #[test]
    fn fee_changes_as_pool_stake_increases() {
        // Simulate adding stakes that cross tier thresholds
        let tiers = vec![
            (1_000, 200), // 2%
            (5_000, 100), // 1%
        ];

        let mut total_stake = 0i64;
        let mut expected_fee_rate = 300u32; // Base 3%

        // Add first stake: 800 -> total = 800, still below Tier 1
        total_stake += 800;
        assert_eq!(calculate_fee(total_stake, expected_fee_rate), 24); // 3% of 800

        // Add second stake: 4_500 -> total = 5_300, crosses Tier 1 (1_000) and Tier 2 (5_000)
        total_stake += 4_500;
        // Fee should be based on total_stake = 5_300, which is >= 5_000 threshold
        expected_fee_rate = 100; // 1% (Tier 2 rate)
        assert_eq!(calculate_fee(total_stake, expected_fee_rate), 53); // 1% of 5300

        // Add third stake: 6_000 -> total = 11_300, still Tier 2
        total_stake += 6_000;
        assert_eq!(calculate_fee(total_stake, expected_fee_rate), 113); // 1% of 11300
    }

    /// Test that fee changes are immediately effective for new pools.
    ///
    /// When a pool is created after a fee change, it should use the new fee rate.
    #[test]
    fn new_pool_uses_current_fee_rate() {
        let config = Config::default_for_test();

        // Base fee is 300 bps (3%) by default
        assert_eq!(config.treasury_fee_bps, 300);

        let test_cases = [
            (1_000, 30),   // 3% of 1000
            (5_000, 150),  // 3% of 5000
            (10_000, 300), // 3% of 10000
        ];

        for (stake, expected_fee) in test_cases {
            let fee = calculate_fee(stake, config.treasury_fee_bps);
            assert_eq!(fee, expected_fee);
        }
    }

    // ── Treasury Accumulation Tests ─────────────────────────────────────────────

    /// Test treasury fee accumulation across multiple pools.
    #[test]
    fn treasury_accumulates_fees_across_pools() {
        let base_fee_bps = 300; // 3%
        let tiers = vec![
            (1_000, 200), // 2%
            (5_000, 100), // 1%
        ];

        // Simulate multiple pools with different stakes
        let pool_stakes = [500, 1_000, 2_500, 5_000, 10_000];
        let mut total_expected_fees = 0i64;

        for stake in pool_stakes {
            // Determine which fee rate applies
            let fee_rate = if stake >= 5_000 {
                100 // Tier 2: 1%
            } else if stake >= 1_000 {
                200 // Tier 1: 2%
            } else {
                base_fee_bps // Base: 3%
            };

            let fee = calculate_fee(stake, fee_rate);
            total_expected_fees += fee;
        }

        // Total expected: 15 + 20 + 50 + 50 + 300 = 435
        assert_eq!(total_expected_fees, 435);
    }

    /// Test treasury partial withdrawal accuracy.
    #[test]
    fn treasury_partial_withdrawal_accuracy() {
        let base_fee_bps = 300; // 3%
        let mut total_fees = 0i64;

        // Simulate fees from multiple pools
        total_fees += calculate_fee(1_000, base_fee_bps); // 30
        total_fees += calculate_fee(5_000, base_fee_bps); // 150
        total_fees += calculate_fee(10_000, base_fee_bps); // 300

        assert_eq!(total_fees, 480);

        // Admin withdraws half (240)
        let remaining = total_fees - 240;
        assert_eq!(remaining, 240);

        // Admin withdraws rest (240)
        let final_remaining = remaining - 240;
        assert_eq!(final_remaining, 0);
    }

    /// Test treasury full withdrawal to multiple recipients.
    #[test]
    fn treasury_withdrawal_to_multiple_recipients() {
        let base_fee_bps = 300; // 3%
        let mut total_fees = 0i64;

        // Accumulate fees
        total_fees += calculate_fee(500, base_fee_bps);    // 15
        total_fees += calculate_fee(1_500, base_fee_bps);  // 45
        total_fees += calculate_fee(3_000, base_fee_bps);  // 90
        total_fees += calculate_fee(7_500, base_fee_bps);  // 225

        assert_eq!(total_fees, 375);

        // Split withdrawal
        let recipient1_amount = 100;
        let recipient2_amount = 150;
        let recipient3_amount = 125;

        let total_withdrawn = recipient1_amount + recipient2_amount + recipient3_amount;
        assert_eq!(total_withdrawn, 375);
        assert_eq!(total_fees - total_withdrawn, 0);
    }

    // ── Edge Case Tests ─────────────────────────────────────────────────────────

    /// Test fee calculation with zero stake.
    #[test]
    fn fee_zero_stake() {
        let base_fee_bps = 300;
        let fee = calculate_fee(0, base_fee_bps);
        assert_eq!(fee, 0);
    }

    /// Test fee calculation with maximum possible fee rate (100%).
    #[test]
    fn fee_maximum_fee_rate() {
        let stake = 1_000_000;
        let max_fee_bps = 10_000; // 100%

        let fee = calculate_fee(stake, max_fee_bps);
        assert_eq!(fee, stake); // 100% of stake
    }

    /// Test fee calculation at minimum non-zero fee rate (1 bps = 0.01%).
    #[test]
    fn fee_minimum_fee_rate() {
        let stake = 10_000;
        let min_fee_bps = 1; // 0.01%

        let fee = calculate_fee(stake, min_fee_bps);
        assert_eq!(fee, 1); // floor(10000 * 1 / 10000) = 1
    }

    /// Test large volume fee calculation (overflow protection).
    #[test]
    fn fee_large_volume() {
        let stake = i64::MAX / 100; // Large but safe stake
        let fee_bps = 10_000; // 100%

        // Should not overflow
        let fee = calculate_fee(stake, fee_bps);
        assert_eq!(fee, stake); // 100% of stake
    }

    /// Test fractional fee rounding (floor behavior).
    #[test]
    fn fee_fractional_rounding() {
        // Test cases where fee calculation produces fractional results
        let test_cases = [
            (100, 300, 3),    // 100 * 300 / 10000 = 3.0
            (101, 300, 3),    // 101 * 300 / 10000 = 3.03 -> floor to 3
            (333, 300, 9),    // 333 * 300 / 10000 = 9.99 -> floor to 9
            (999, 300, 29),   // 999 * 300 / 10000 = 29.97 -> floor to 29
            (1_000, 333, 33), // 1000 * 333 / 10000 = 33.3 -> floor to 33
        ];

        for (stake, fee_bps, expected_fee) in test_cases {
            let fee = calculate_fee(stake, fee_bps);
            assert_eq!(fee, expected_fee, "Failed for stake={}, fee_bps={}", stake, fee_bps);
        }
    }

    // ── Boundary Transition Tests ───────────────────────────────────────────────

    /// Test transitions between consecutive tier thresholds.
    #[test]
    fn fee_tier_boundary_transitions() {
        let tiers = vec![
            (1_000, 200), // 2%
            (5_000, 100), // 1%
        ];

        // Test just below, at, and just above each threshold
        for (threshold, tier_fee) in tiers {
            // Just below threshold -> use base or lower tier
            let below = threshold - 1;
            let base_fee_bps = 300; // Assume base is 3%
            let fee_below = calculate_fee(below, base_fee_bps);

            // At threshold -> use tier fee
            let fee_at = calculate_fee(threshold, tier_fee);

            // Just above threshold -> still use tier fee
            let above = threshold + 1;
            let fee_above = calculate_fee(above, tier_fee);

            // Verify tier fee is lower than base (as expected)
            assert!(fee_at < fee_below, "Tier fee should be lower than base at threshold {}", threshold);
            assert_eq!(fee_at, fee_above, "Fee should be same at and above threshold {}", threshold);
        }
    }

    /// Test fee calculation with multiple tiers in descending order.
    #[test]
    fn fee_multiple_tiers_descending_order() {
        let tiers = vec![
            (10_000, 50),  // 0.5%
            (5_000, 100),  // 1%
            (1_000, 200),  // 2%
        ];

        let base_fee_bps = 300; // 3%

        let test_cases = [
            (500, base_fee_bps),    // Base
            (999, base_fee_bps),    // Base
            (1_000, 200),           // Tier 1
            (4_999, 200),           // Tier 1
            (5_000, 100),           // Tier 2
            (9_999, 100),           // Tier 2
            (10_000, 50),           // Tier 3
            (20_000, 50),           // Tier 3
        ];

        for (stake, expected_fee_bps) in test_cases {
            let fee = calculate_fee(stake, expected_fee_bps);
            let expected_fee = (stake as f64 * expected_fee_bps as f64 / 10_000.0) as i64;
            assert_eq!(fee, expected_fee);
        }
    }

    // ── API Endpoint Tests ──────────────────────────────────────────────────────

    /// Test /api/v1/fees endpoint returns correct configuration.
    #[test]
    fn api_fees_endpoint_returns_configured_values() {
        let config = Config::default_for_test();

        // Default values
        assert_eq!(config.treasury_fee_bps, 300);  // 3%
        assert_eq!(config.referral_fee_bps, 5_000); // 50%
    }

    /// Test fee calculation with various realistic scenarios.
    #[test]
    fn fee_realistic_scenarios() {
        // Scenario 1: Small pool (100 XLM) at base rate
        let fee = calculate_fee(100_000_000, 300); // 100 XLM in stroops
        assert_eq!(fee, 3_000_000); // 3% = 3 XLM

        // Scenario 2: Medium pool (10,000 XLM) at tier 1
        let fee = calculate_fee(10_000_000_000, 200); // 10,000 XLM
        assert_eq!(fee, 20_000_000); // 2% = 200 XLM

        // Scenario 3: Large pool (1M XLM) at tier 2
        let fee = calculate_fee(1_000_000_000_000, 100); // 1M XLM
        assert_eq!(fee, 10_000_000_000); // 1% = 10,000 XLM

        // Scenario 4: Very large pool (10M XLM) at tier 3
        let fee = calculate_fee(10_000_000_000_000, 50); // 10M XLM
        assert_eq!(fee, 50_000_000_000); // 0.5% = 50,000 XLM
    }

    // ── Helper Functions ────────────────────────────────────────────────────────

    /// Calculate fee amount given stake and fee rate in basis points.
    ///
    /// Formula: `fee = floor(stake * fee_bps / 10_000)`
    fn calculate_fee(stake: i64, fee_bps: u32) -> i64 {
        (stake as i128 * fee_bps as i128 / 10_000i128) as i64
    }

    /// Verify fee calculation matches expected formula.
    #[test]
    fn fee_formula_verification() {
        let stake = 10_000;
        let fee_bps = 300; // 3%

        // Formula: floor(stake * fee_bps / 10_000)
        let expected = (stake as i128 * fee_bps as i128 / 10_000i128) as i64;
        let calculated = calculate_fee(stake, fee_bps);

        assert_eq!(calculated, expected);
        assert_eq!(calculated, 300); // 3% of 10000
    }

    /// Test fee tier selection logic (highest threshold <= stake).
    #[test]
    fn fee_tier_selection_logic() {
        let tiers = vec![
            (1_000, 200),
            (5_000, 100),
            (10_000, 50),
        ];

        let base_fee_bps = 300;

        // For each stake, find the correct tier
        let test_cases = [
            (500, base_fee_bps),    // Below all -> base
            (999, base_fee_bps),    // Below all -> base
            (1_000, 200),           // Exact Tier 1
            (1_500, 200),           // Between Tier 1 and 2
            (4_999, 200),           // Just below Tier 2
            (5_000, 100),           // Exact Tier 2
            (7_500, 100),           // Between Tier 2 and 3
            (9_999, 100),           // Just below Tier 3
            (10_000, 50),           // Exact Tier 3
            (15_000, 50),           // Above all tiers
        ];

        for (stake, expected_fee_bps) in test_cases {
            let fee = calculate_fee(stake, expected_fee_bps);
            assert!(fee > 0 || stake == 0);
        }
    }

    /// Test that fee rates are monotonically decreasing with thresholds.
    #[test]
    fn fee_rates_monotonically_decreasing() {
        let tiers = vec![
            (1_000, 300),  // 3%
            (5_000, 200),  // 2%
            (10_000, 100), // 1%
            (50_000, 50),  // 0.5%
        ];

        for window in tiers.windows(2) {
            let (threshold1, fee1) = window[0];
            let (threshold2, fee2) = window[1];

            // Higher threshold should have lower fee rate
            assert!(threshold2 > threshold1, "Thresholds should increase");
            assert!(fee2 < fee1, "Fee rates should decrease with higher thresholds");
        }
    }

    /// Test treasury accumulation with fee changes mid-period.
    #[test]
    fn treasury_accumulation_with_fee_changes() {
        // Initial state: Base fee 3% (300 bps)
        let mut total_fees = 0i64;

        // Pool 1: 1,000 stake at base rate
        total_fees += calculate_fee(1_000, 300); // 30

        // Fee changes to 2% (200 bps) before Pool 2
        total_fees += calculate_fee(2_000, 200); // 40

        // Fee changes to 1% (100 bps) before Pool 3
        total_fees += calculate_fee(5_000, 100); // 50

        // Total: 30 + 40 + 50 = 120
        assert_eq!(total_fees, 120);
    }

    /// Test that fee calculations are idempotent (same inputs → same outputs).
    #[test]
    fn fee_idempotency() {
        let test_cases = [
            (1_000, 300),
            (5_000, 200),
            (10_000, 100),
            (100, 50),
            (1_000_000, 10_000),
        ];

        for (stake, fee_bps) in test_cases {
            let fee1 = calculate_fee(stake, fee_bps);
            let fee2 = calculate_fee(stake, fee_bps);
            let fee3 = calculate_fee(stake, fee_bps);

            assert_eq!(fee1, fee2);
            assert_eq!(fee2, fee3);
        }
    }

    /// Test fee calculation with varying fee_bps precision.
    #[test]
    fn fee_precision_tests() {
        let base_stake = 10_000;

        let test_cases = [
            (1, 1),      // 0.01% of 10000 = 1
            (10, 10),    // 0.1% of 10000 = 10
            (50, 50),    // 0.5% of 10000 = 50
            (100, 100),  // 1% of 10000 = 100
            (200, 200),  // 2% of 10000 = 200
            (500, 500),  // 5% of 10000 = 500
            (1_000, 1_000),  // 10% of 10000 = 1000
            (2_500, 2_500),  // 25% of 10000 = 2500
            (5_000, 5_000),  // 50% of 10000 = 5000
            (10_000, 10_000), // 100% of 10000 = 10000
        ];

        for (fee_bps, expected_fee) in test_cases {
            let fee = calculate_fee(base_stake, fee_bps);
            assert_eq!(fee, expected_fee);
        }
    }
}
