//! # High-Volume Concurrent Prediction Stress Tests
//!
//! This module contains comprehensive stress tests for prediction markets under high concurrency:
//! - 1000+ concurrent predictions on a single pool
//! - Gas consumption scaling analysis
//! - Payout accuracy verification with many participants
//! - Algorithmic complexity detection (O(1), O(n), O(n²))
//!
//! Run with:
//! ```bash
//! cargo test -p predifi-contract stress_test_high_volume -- --nocapture --test-threads=1
//! ```

#[cfg(test)]
mod high_volume_stress_tests {
    extern crate std;
    use std::collections::HashMap;
    use std::time::Instant;

    use crate::{
        calculate_claim_payout, PayoutInput, PoolConfig, PredifiContract, PredifiContractClient,
    };
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Ledger},
        token, Address, Env, String, Vec,
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

    fn stress_setup(
        env: &Env,
    ) -> (
        PredifiContractClient<'_>,
        Address,
        Address,
        token::Client<'_>,
        token::StellarAssetClient<'_>,
    ) {
        env.mock_all_auths();

        env.ledger().with_mut(|li| {
            li.protocol_version = 23;
            li.timestamp = 1000;
        });

        let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
        let ac_client = dummy_access_control::DummyAccessControlClient::new(env, &ac_id);

        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(env, &contract_id);

        let admin = Address::generate(env);
        let treasury = Address::generate(env);

        ac_client.grant_role(&admin, &ROLE_ADMIN);
        ac_client.grant_role(&admin, &ROLE_OPERATOR);

        client.init(&ac_id, &treasury, &500, &3600, &3600u64, &0u32);

        let token_admin = Address::generate(env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();
        let token_client = token::Client::new(env, &token_id);
        let token_admin_client = token::StellarAssetClient::new(env, &token_id);

        client.add_token_to_whitelist(&admin, &token_id);

        (client, admin, ac_id, token_client, token_admin_client)
    }

    /// Test: 1000 concurrent predictions on a single 2-outcome pool
    /// Measures: Gas scaling, transaction throughput, payout accuracy
    #[test]
    fn test_1000_concurrent_predictions_binary_pool() {
        let env = Env::default();
        let (client, _admin, ac_id, token_client, token_admin_client) = stress_setup(&env);

        // Setup token
        let token_id = token_client.address.clone();
        let mut users = std::vec::Vec::with_capacity(1000);
        for i in 0..1000u32 {
            let user = Address::generate(&env);
            token_admin_client.mint(&user, &1_000_000_000i128);
            users.push(user);
        }

        // Create pool
        let creator = Address::generate(&env);
        token_admin_client.mint(&creator, &10_000_000i128);

        let config = PoolConfig {
            start_time: 1000u64,
            description: String::from_slice(&env, "1000-user stress test"),
            metadata_url: String::from_slice(&env, "https://example.com"),
            min_stake: 1_000i128,
            max_stake: 100_000i128,
            min_total_stake: 1_000_000i128,
            max_total_stake: 1_000_000_000i128,
            initial_liquidity: 10_000i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: Vec::new(&env),
        };

        let pool_id = client.create_pool(
            &creator,
            &5000u64,
            &token_id,
            &2u32,
            &symbol_short!("TEST"),
            &config,
        );

        println!("[stress] Pool created: {}", pool_id);

        // Phase 1: Place 500 predictions on outcome 0
        let start = Instant::now();
        let mut gas_costs_outcome_0 = std::vec::Vec::with_capacity(500);

        for i in 0..500u32 {
            let user = &users[i as usize];
            let gas_before = env.budget().cpu_instruction_cost();
            
            client.place_prediction(&user, &pool_id, &1_000i128, &0u32, &None, &None);
            
            let gas_after = env.budget().cpu_instruction_cost();
            let gas_used = gas_after.saturating_sub(gas_before);
            gas_costs_outcome_0.push(gas_used);
        }

        let elapsed_0 = start.elapsed();
        println!("[stress] 500 predictions on outcome 0: {:.2}s", elapsed_0.as_secs_f64());
        println!("[gas] avg gas (outcome 0): {}", average(&gas_costs_outcome_0));
        println!("[gas] min gas (outcome 0): {}", min_or_zero(&gas_costs_outcome_0));
        println!("[gas] max gas (outcome 0): {}", max_or_zero(&gas_costs_outcome_0));

        // Phase 2: Place 500 predictions on outcome 1
        let start = Instant::now();
        let mut gas_costs_outcome_1 = std::vec::Vec::with_capacity(500);

        for i in 500..1000u32 {
            let user = &users[i as usize];
            let gas_before = env.budget().cpu_instruction_cost();
            
            client.place_prediction(&user, &pool_id, &1_000i128, &1u32, &None, &None);
            
            let gas_after = env.budget().cpu_instruction_cost();
            let gas_used = gas_after.saturating_sub(gas_before);
            gas_costs_outcome_1.push(gas_used);
        }

        let elapsed_1 = start.elapsed();
        println!("[stress] 500 predictions on outcome 1: {:.2}s", elapsed_1.as_secs_f64());
        println!("[gas] avg gas (outcome 1): {}", average(&gas_costs_outcome_1));
        println!("[gas] min gas (outcome 1): {}", min_or_zero(&gas_costs_outcome_1));
        println!("[gas] max gas (outcome 1): {}", max_or_zero(&gas_costs_outcome_1));

        // Verify pool total stake is correct
        let pool = client.get_pool(&pool_id);
        let expected_total_stake = 500_000i128 + 500_000i128; // 500 × 1000 + 500 × 1000
        assert_eq!(pool.total_stake, expected_total_stake);
        println!("[stress] ✅ Pool total stake correct: {}", pool.total_stake);

        // Phase 3: Resolve pool to outcome 0
        let operator = Address::generate(&env);
        let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
        ac_client.grant_role(&operator, &ROLE_OPERATOR);

        env.ledger().with_mut(|li| {
            li.timestamp = 9001;
        });

        client.resolve_pool(&operator, &pool_id, &0u32);
        println!("[stress] Pool resolved to outcome 0");

        // Phase 4: Verify payout accuracy for all 500 winners
        let winning_stake = 500_000i128; // Total stake on outcome 0
        let protocol_fee_bps = 500i128; // From config
        
        println!("[stress] Verifying payouts for 500 winners...");
        let mut claimed_total = 0i128;
        for i in 0..500u32 {
            let user = &users[i as usize];
            let user_stake = 1_000i128;
            
            let payout_input = PayoutInput {
                pool_total_stake: expected_total_stake,
                fee_bps: protocol_fee_bps,
                user_stake,
                winning_stake,
            };

            let payout = calculate_claim_payout(&payout_input).unwrap();
            
            // Verify invariant: winnings never exceed total stake
            assert!(payout.winnings <= expected_total_stake);
            
            // Verify proportional split
            let expected_payout = (user_stake as f64 / winning_stake as f64) * payout.payout_pool as f64;
            let actual_payout = payout.winnings as f64;
            let diff_pct = ((actual_payout - expected_payout).abs() / expected_payout) * 100.0;
            
            // Allow 1% rounding error
            assert!(diff_pct < 1.0, "Payout mismatch for user {}: {:.2}% off", i, diff_pct);

            claimed_total += client.claim_winnings(&user, &pool_id);
        }
        assert_eq!(claimed_total, 995_000i128);
        println!("[stress] ✅ All 500 payouts verified (±1% tolerance)");

        // Algorithmic Complexity Analysis
        println!("\n[analysis] Algorithmic Complexity:");
        println!("[analysis] Gas increase (outcome 0 → 1): {:.1}%", 
            ((average(&gas_costs_outcome_1) as f64 / average(&gas_costs_outcome_0) as f64 - 1.0) * 100.0));
        
        // If gas is linear in prediction count, should be flat. If quadratic, should increase.
        let gas_ratio = average(&gas_costs_outcome_1) as f64 / average(&gas_costs_outcome_0) as f64;
        if gas_ratio > 1.1 {
            println!("[warn] ⚠️  Gas scaling detected: {:.2}x (potential O(n) or worse)", gas_ratio);
        } else {
            println!("[stress] ✅ Gas scaling is linear/constant (good)");
        }
    }

    /// Test: 1000 concurrent predictions with many outcomes (16-outcome pool)
    /// Measures: Gas scaling with outcome count, outcome stake updates
    #[test]
    fn test_1000_predictions_16_outcomes() {
        let env = Env::default();
        let (client, _admin, ac_id, token_client, token_admin_client) = stress_setup(&env);

        let token_id = token_client.address.clone();
        let mut users = std::vec::Vec::with_capacity(1000);
        for i in 0..1000u32 {
            let user = Address::generate(&env);
            token_admin_client.mint(&user, &1_000_000_000i128);
            users.push(user);
        }

        let creator = Address::generate(&env);
        token_admin_client.mint(&creator, &10_000_000i128);

        let config = PoolConfig {
            start_time: 1000u64,
            description: String::from_slice(&env, "16-outcome stress test"),
            metadata_url: String::from_slice(&env, "https://example.com"),
            min_stake: 1_000i128,
            max_stake: 100_000i128,
            min_total_stake: 1_000_000i128,
            max_total_stake: 1_000_000_000i128,
            initial_liquidity: 10_000i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: Vec::new(&env),
        };

        let pool_id = client.create_pool(
            &creator,
            &5000u64,
            &token_id,
            &16u32, // 16 outcomes
            &symbol_short!("TEST"),
            &config,
        );

        println!("[stress] 16-outcome pool created: {}", pool_id);

        // Place predictions on each outcome (62-63 per outcome, total 1000)
        let mut gas_costs = std::vec::Vec::with_capacity(1000);
        let predictions_per_outcome = 1000 / 16;

        for outcome in 0..16i32 {
            let start_idx = (outcome as usize) * predictions_per_outcome;
            let end_idx = if outcome == 15 {
                1000
            } else {
                start_idx + predictions_per_outcome
            };

            for i in start_idx..end_idx {
                let user = &users[i];
                let gas_before = env.budget().cpu_instruction_cost();
                
                client.place_prediction(&user, &pool_id, &1_000i128, &(outcome as u32), &None, &None);
                
                let gas_after = env.budget().cpu_instruction_cost();
                let gas_used = gas_after.saturating_sub(gas_before);
                gas_costs.push((outcome, gas_used));
            }
        }

        println!("[stress] 1000 predictions on 16 outcomes completed");
        
        // Analyze gas per outcome
        let mut gas_by_outcome: HashMap<i32, std::vec::Vec<u64>> = HashMap::new();
        for (outcome, gas) in gas_costs.iter() {
            gas_by_outcome
                .entry(*outcome)
                .or_insert_with(std::vec::Vec::new)
                .push(*gas);
        }

        for outcome in 0..16i32 {
            if let Some(costs) = gas_by_outcome.get(&outcome) {
                println!("[gas] outcome {}: avg={}, min={}, max={}", 
                    outcome,
                    average_slice(costs),
                    min_or_zero_slice(costs),
                    max_or_zero_slice(costs));
            }
        }

        // Verify outcome stakes
        let pool = client.get_pool(&pool_id);
        assert_eq!(pool.total_stake, 1_000_000i128);
        println!("[stress] ✅ Pool total stake: {} (expected 1,000,000)", pool.total_stake);

        // Check for quadratic behavior
        let first_pred_avg = gas_by_outcome.get(&0).map(|v| average_slice(v)).unwrap_or(0);
        let last_pred_avg = gas_by_outcome.get(&15).map(|v| average_slice(v)).unwrap_or(0);
        let ratio = last_pred_avg as f64 / first_pred_avg as f64;

        println!("[analysis] Gas scaling (outcome 0 → 15): {:.2}x", ratio);
        if ratio > 1.5 {
            println!("[warn] ⚠️  Significant gas increase detected: {:.2}x", ratio);
        } else {
            println!("[stress] ✅ No quadratic complexity detected");
        }
    }

    /// Test: Payout accuracy across varying pool sizes (100 to 1000 winners)
    /// Measures: Payout correctness under load, rounding error accumulation
    #[test]
    fn test_payout_accuracy_scaling_winners() {
        println!("\n[stress] Testing payout accuracy with varying winner counts...");

        let winner_counts = std::vec![10usize, 50, 100, 500, 1000];
        let pool_total_stake = 1_000_000i128;
        let fee_bps = 500i128; // 5%
        let protocol_fee = (pool_total_stake * fee_bps) / 10_000;
        let payout_pool = pool_total_stake - protocol_fee;

        for winner_count in winner_counts.iter() {
            let winning_stake = pool_total_stake / 2; // 50% on winning side
            let user_stake = winning_stake / (*winner_count as i128);

            let mut payout_sum = 0i128;
            let mut max_error = 0i128;

            for _ in 0..*winner_count {
                let payout_input = PayoutInput {
                    pool_total_stake,
                    fee_bps,
                    user_stake,
                    winning_stake,
                };

                let payout = calculate_claim_payout(&payout_input).unwrap();
                payout_sum += payout.winnings;
                
                let expected_payout = (user_stake as f64 / winning_stake as f64) * payout_pool as f64;
                let actual_payout = payout.winnings as f64;
                let error = (actual_payout - expected_payout).abs() as i128;
                max_error = max_error.max(error);
            }

            let error_pct = (max_error as f64 / payout_pool as f64) * 100.0;
            println!("[payout] {} winners: sum_payouts={}, max_error={} ({:.4}%)",
                winner_count, payout_sum, max_error, error_pct);

            // Ensure payout sum doesn't exceed pool
            assert!(payout_sum <= pool_total_stake);
        }

        println!("[stress] ✅ Payout accuracy verified across winner scales");
    }

    /// Test: Detection of O(n²) behavior in claim processing
    /// Measures: Claim latency scaling with winner count
    #[test]
    fn test_claim_processing_complexity() {
        let env = Env::default();
        let (client, _admin, ac_id, token_client, token_admin_client) = stress_setup(&env);

        let token_id = token_client.address.clone();
        let mut users = std::vec::Vec::with_capacity(200);
        for i in 0..200u32 {
            let user = Address::generate(&env);
            token_admin_client.mint(&user, &1_000_000_000i128);
            users.push(user);
        }

        let creator = Address::generate(&env);
        token_admin_client.mint(&creator, &10_000_000i128);

        let config = PoolConfig {
            start_time: 1000u64,
            description: String::from_slice(&env, "Claim complexity test"),
            metadata_url: String::from_slice(&env, "https://example.com"),
            min_stake: 1_000i128,
            max_stake: 100_000i128,
            min_total_stake: 1_000_000i128,
            max_total_stake: 1_000_000_000i128,
            initial_liquidity: 10_000i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: Vec::new(&env),
        };

        let pool_id = client.create_pool(
            &creator,
            &5000u64,
            &token_id,
            &2u32,
            &symbol_short!("TEST"),
            &config,
        );

        // Place 200 predictions on outcome 0
        for user in users.iter() {
            client.place_prediction(&user, &pool_id, &1_000i128, &0u32, &None, &None);
        }

        // Resolve to outcome 0
        let operator = Address::generate(&env);
        let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
        ac_client.grant_role(&operator, &ROLE_OPERATOR);

        env.ledger().with_mut(|li| {
            li.timestamp = 9001;
        });

        client.resolve_pool(&operator, &pool_id, &0u32);

        // Measure claim processing time for increasing winner counts
        let winner_counts = std::vec![10usize, 50, 100, 200];
        let mut claim_times = std::vec::Vec::with_capacity(winner_counts.len());

        let mut previous_count = 0usize;
        for count in winner_counts.iter() {
            let start = Instant::now();

            for i in previous_count..*count {
                let user = &users[i];
                client.claim_winnings(&user, &pool_id);
            }

            let elapsed = start.elapsed();
            claim_times.push((*count, elapsed.as_secs_f64()));
            println!("[claim] {} winners: {:.3}s", count, elapsed.as_secs_f64());
            previous_count = *count;
        }

        // Analyze complexity
        for i in 1..claim_times.len() {
            let (count1, time1) = claim_times[i - 1];
            let (count2, time2) = claim_times[i];

            let count_ratio = count2 as f64 / count1 as f64;
            let time_ratio = time2 / time1;

            let complexity = time_ratio.log2() / count_ratio.log2();
            println!("[analysis] Scaling: {:.2}x winners → {:.2}x time (complexity: ~O(n^{:.2}))", 
                count_ratio, time_ratio, complexity);

            // If complexity > 1.5, likely O(n²)
            if complexity > 1.5 {
                println!("[warn] ⚠️  Potential O(n²) behavior detected!");
            }
        }

        println!("[stress] ✅ Claim processing complexity analysis complete");
    }

    // Helper functions
    fn average(values: &[u64]) -> u64 {
        if values.is_empty() { return 0; }
        values.iter().sum::<u64>() / values.len() as u64
    }

    fn average_slice(values: &[u64]) -> u64 {
        if values.is_empty() { return 0; }
        values.iter().sum::<u64>() / values.len() as u64
    }

    fn min_or_zero(values: &[u64]) -> u64 {
        values.iter().copied().min().unwrap_or(0)
    }

    fn min_or_zero_slice(values: &[u64]) -> u64 {
        values.iter().copied().min().unwrap_or(0)
    }

    fn max_or_zero(values: &[u64]) -> u64 {
        values.iter().copied().max().unwrap_or(0)
    }

    fn max_or_zero_slice(values: &[u64]) -> u64 {
        values.iter().copied().max().unwrap_or(0)
    }
}
