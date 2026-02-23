//! # Safe Math Usage Examples
//!
//! This module demonstrates how to use the SafeMath module for various
//! payout and fee calculation scenarios in the PrediFi contract.

extern crate std;

use crate::safe_math::{RoundingMode, SafeMath};
use predifi_errors::PrediFiError;
use std::println;

/// Example: Calculate protocol fee from a prediction amount
///
/// When a user places a prediction, we need to deduct a protocol fee.
/// Using ProtocolFavor rounding ensures any fractional amounts stay in the pool.
#[test]
fn example_calculate_protocol_fee() {
    let prediction_amount = 1_000_000_000; // 100 tokens (7 decimals)
    let fee_bps = 250; // 2.5%

    // Calculate fee with protocol-favoring rounding
    let fee = SafeMath::percentage(prediction_amount, fee_bps, RoundingMode::ProtocolFavor)
        .expect("Fee calculation failed");

    let net_amount = SafeMath::safe_sub(prediction_amount, fee).expect("Subtraction failed");

    assert_eq!(fee, 25_000_000); // 2.5 tokens
    assert_eq!(net_amount, 975_000_000); // 97.5 tokens

    println!("Prediction: {}", prediction_amount);
    println!("Fee (2.5%): {}", fee);
    println!("Net stake: {}", net_amount);
}

/// Example: Calculate winner payout proportionally
///
/// When a pool resolves, winners get paid proportionally to their stake
/// in the winning outcome.
#[test]
fn example_calculate_winner_payout() {
    let user_stake = 500_000_000; // User staked 50 tokens
    let winning_outcome_total = 2_000_000_000; // Total winning stake: 200 tokens
    let pool_balance = 10_000_000_000; // Pool has 1000 tokens to distribute

    // Calculate user's proportional payout
    // Using Neutral rounding for fairness
    let payout = SafeMath::proportion(
        user_stake,
        winning_outcome_total,
        pool_balance,
        RoundingMode::Neutral,
    )
    .expect("Payout calculation failed");

    // User staked 25% of winning side, gets 25% of pool
    assert_eq!(payout, 2_500_000_000); // 250 tokens

    println!("User stake: {}", user_stake);
    println!("Total winning stake: {}", winning_outcome_total);
    println!("Pool balance: {}", pool_balance);
    println!("User payout: {}", payout);
}

/// Example: Handle edge case where user is sole winner
#[test]
fn example_sole_winner() {
    let user_stake = 100_000_000; // User staked 10 tokens
    let winning_outcome_total = 100_000_000; // Only winner
    let pool_balance = 5_000_000_000; // Pool has 500 tokens

    let payout = SafeMath::proportion(
        user_stake,
        winning_outcome_total,
        pool_balance,
        RoundingMode::Neutral,
    )
    .expect("Payout calculation failed");

    // User gets entire pool
    assert_eq!(payout, pool_balance);

    println!("Sole winner gets entire pool: {}", payout);
}

/// Example: Prevent division by zero
#[test]
fn example_zero_stake_protection() {
    let user_stake = 100_000_000;
    let winning_outcome_total = 0; // No winning stakes (shouldn't happen, but protected)
    let pool_balance = 5_000_000_000;

    let result = SafeMath::proportion(
        user_stake,
        winning_outcome_total,
        pool_balance,
        RoundingMode::Neutral,
    );

    // Should return error, not panic
    assert_eq!(result, Err(PrediFiError::ArithmeticError));

    println!("Division by zero safely caught");
}

/// Example: Calculate fee and verify it doesn't exceed amount
#[test]
fn example_fee_validation() {
    let amount = 1000;
    let fee_bps = 10000; // 100%

    let fee = SafeMath::percentage(amount, fee_bps, RoundingMode::Neutral)
        .expect("Fee calculation failed");

    assert_eq!(fee, amount); // Fee equals amount at 100%

    // Verify fee doesn't exceed amount
    assert!(fee <= amount);

    // Invalid fee (> 100%) is rejected
    let invalid_result = SafeMath::percentage(amount, 10001, RoundingMode::Neutral);
    assert_eq!(invalid_result, Err(PrediFiError::InvalidFeeBps));

    println!("Fee validation working correctly");
}

/// Example: Rounding mode comparison
#[test]
fn example_rounding_modes() {
    let user_stake = 333; // 33.3% of total
    let total_stake = 1000;
    let pool_balance = 100;

    // Protocol favor: rounds down (33)
    let protocol_payout = SafeMath::proportion(
        user_stake,
        total_stake,
        pool_balance,
        RoundingMode::ProtocolFavor,
    )
    .unwrap();

    // Neutral: rounds to nearest (33)
    let neutral_payout =
        SafeMath::proportion(user_stake, total_stake, pool_balance, RoundingMode::Neutral).unwrap();

    // User favor: rounds up (34)
    let user_payout = SafeMath::proportion(
        user_stake,
        total_stake,
        pool_balance,
        RoundingMode::UserFavor,
    )
    .unwrap();

    println!("Protocol favor: {}", protocol_payout); // 33
    println!("Neutral: {}", neutral_payout); // 33
    println!("User favor: {}", user_payout); // 34

    // For production, use ProtocolFavor or Neutral to prevent over-distribution
    assert!(protocol_payout <= neutral_payout);
    assert!(neutral_payout <= user_payout);
}

/// Example: Safe arithmetic operations
#[test]
fn example_safe_arithmetic() {
    let pool_balance = 1_000_000_000;
    let payout = 250_000_000;

    // Safe subtraction
    let remaining = SafeMath::safe_sub(pool_balance, payout).expect("Subtraction failed");
    assert_eq!(remaining, 750_000_000);

    // Safe addition
    let new_stake = 100_000_000;
    let updated_total = SafeMath::safe_add(pool_balance, new_stake).expect("Addition failed");
    assert_eq!(updated_total, 1_100_000_000);

    // Overflow protection
    let overflow_result = SafeMath::safe_add(i128::MAX, 1);
    assert_eq!(overflow_result, Err(PrediFiError::ArithmeticError));

    println!("Safe arithmetic prevents overflow/underflow");
}

/// Example: Real-world payout scenario with multiple winners
#[test]
fn example_realistic_payout_scenario() {
    // Pool setup
    let total_pool = 10_000_000_000; // 1000 tokens
    let fee_bps = 200; // 2% protocol fee

    // Calculate protocol fee
    let protocol_fee = SafeMath::percentage(total_pool, fee_bps, RoundingMode::ProtocolFavor)
        .expect("Fee calculation failed");

    // Remaining for winners
    let payout_pool = SafeMath::safe_sub(total_pool, protocol_fee).expect("Subtraction failed");

    // Winner stakes on the winning outcome
    let winner1_stake = 300_000_000; // 30 tokens
    let winner2_stake = 500_000_000; // 50 tokens
    let winner3_stake = 200_000_000; // 20 tokens
    let total_winning_stake = 1_000_000_000; // 100 tokens total

    // Calculate each winner's payout
    let payout1 = SafeMath::proportion(
        winner1_stake,
        total_winning_stake,
        payout_pool,
        RoundingMode::Neutral,
    )
    .expect("Payout 1 failed");

    let payout2 = SafeMath::proportion(
        winner2_stake,
        total_winning_stake,
        payout_pool,
        RoundingMode::Neutral,
    )
    .expect("Payout 2 failed");

    let payout3 = SafeMath::proportion(
        winner3_stake,
        total_winning_stake,
        payout_pool,
        RoundingMode::Neutral,
    )
    .expect("Payout 3 failed");

    println!("Total pool: {}", total_pool);
    println!("Protocol fee (2%): {}", protocol_fee);
    println!("Payout pool: {}", payout_pool);
    println!("Winner 1 (30%): {}", payout1);
    println!("Winner 2 (50%): {}", payout2);
    println!("Winner 3 (20%): {}", payout3);

    // Verify total payouts don't exceed payout pool
    let total_paid = SafeMath::safe_add(payout1, payout2)
        .and_then(|sum| SafeMath::safe_add(sum, payout3))
        .expect("Sum failed");

    assert!(total_paid <= payout_pool);
    println!("Total paid: {} (safe)", total_paid);
}

/// Example: Handling very small amounts (dust)
#[test]
fn example_dust_handling() {
    let tiny_amount = 10; // Very small amount
    let fee_bps = 100; // 1%

    // With protocol favor, small fees round down to 0
    let fee = SafeMath::percentage(tiny_amount, fee_bps, RoundingMode::ProtocolFavor)
        .expect("Fee calculation failed");

    // 1% of 10 = 0.1, rounds down to 0
    assert_eq!(fee, 0);

    // User keeps full amount when fee rounds to 0
    let net = SafeMath::safe_sub(tiny_amount, fee).expect("Subtraction failed");
    assert_eq!(net, tiny_amount);

    println!("Dust amounts handled gracefully");
}
