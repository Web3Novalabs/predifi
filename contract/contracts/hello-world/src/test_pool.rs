#![cfg(test)]
use super::*;
use soroban_sdk::{Env, String};

fn create_test_pool(status: PoolStatus, end_time: u64) -> Pool {
    Pool {
        pool_id: 1,
        name: String::from_str(&Env::default(), "Test Pool"),
        total_liquidity: 0,
        token_a: String::from_str(&Env::default(), "ETH"),
        token_b: String::from_str(&Env::default(), "USDC"),
        fee_rate: 30,
        is_active: true,
        status,
        end_time,
    }
}

fn create_test_pool(status: PoolStatus, end_time: u64) -> Pool {
    Pool {
        pool_id: 1,
        name: String::from_str(&Env::default(), "Test Pool"),
        total_liquidity: 0,
        token_a: String::from_str(&Env::default(), "ETH"),
        token_b: String::from_str(&Env::default(), "USDC"),
        fee_rate: 30,
        is_active: true,
        status,
        end_time,
    }
}

#[test]
fn test_is_pool_active() {
    let pool = create_test_pool(PoolStatus::Active, 100);
    assert!(pool.is_pool_active());

    let pool_resolved = create_test_pool(PoolStatus::Resolved, 100);
    assert!(!pool_resolved.is_pool_active());
}

#[test]
fn test_is_pool_resolved() {
    let pool = create_test_pool(PoolStatus::Resolved, 100);
    assert!(pool.is_pool_resolved());

    let pool_active = create_test_pool(PoolStatus::Active, 100);
    assert!(!pool_active.is_pool_resolved());
}

#[test]
fn test_can_accept_predictions() {
    let env = Env::default();

    // Test active pool within time
    let pool = create_test_pool(PoolStatus::Active, 100);
    // Default ledger timestamp is 0
    assert!(pool.can_accept_predictions(&env));

    // Test active pool expierd
    let pool_expired = create_test_pool(PoolStatus::Active, 0);
    // 0 < 0 is false
    assert!(!pool_expired.can_accept_predictions(&env));

    // Test inactive pool
    let pool_closed = create_test_pool(PoolStatus::Closed, 100);
    assert!(!pool_closed.can_accept_predictions(&env));
}

#[test]
fn test_validate_state_transition() {
    let pool_active = create_test_pool(PoolStatus::Active, 100);

    assert!(pool_active.validate_state_transition(PoolStatus::Resolved));
    assert!(pool_active.validate_state_transition(PoolStatus::Closed));
    assert!(!pool_active.validate_state_transition(PoolStatus::Disputed));

    let pool_resolved = create_test_pool(PoolStatus::Resolved, 100);
    assert!(pool_resolved.validate_state_transition(PoolStatus::Disputed));
    assert!(!pool_resolved.validate_state_transition(PoolStatus::Active));
}
