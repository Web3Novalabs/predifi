#![cfg(test)]
use super::*;
use soroban_sdk::Env;

#[test]
fn test_is_pool_active() {
    let pool = Pool {
        status: PoolStatus::Active,
        end_time: 100,
    };
    assert!(pool.is_pool_active());

    let pool_resolved = Pool {
        status: PoolStatus::Resolved,
        end_time: 100,
    };
    assert!(!pool_resolved.is_pool_active());
}

#[test]
fn test_is_pool_resolved() {
    let pool = Pool {
        status: PoolStatus::Resolved,
        end_time: 100,
    };
    assert!(pool.is_pool_resolved());

    let pool_active = Pool {
        status: PoolStatus::Active,
        end_time: 100,
    };
    assert!(!pool_active.is_pool_resolved());
}

#[test]
fn test_can_accept_predictions() {
    let env = Env::default();
    
    // Test active pool within time
    let pool = Pool {
        status: PoolStatus::Active,
        end_time: 100,
    };
    // Default ledger timestamp is 0
    assert!(pool.can_accept_predictions(&env));

    // Test active pool expierd
    let pool_expired = Pool {
        status: PoolStatus::Active,
        end_time: 0, 
    };
    // 0 < 0 is false
    assert!(!pool_expired.can_accept_predictions(&env));

    // Test inactive pool
    let pool_closed = Pool {
        status: PoolStatus::Closed,
        end_time: 100,
    };
    assert!(!pool_closed.can_accept_predictions(&env));
}

#[test]
fn test_validate_state_transition() {
    let pool_active = Pool {
        status: PoolStatus::Active,
        end_time: 100,
    };
    
    assert!(pool_active.validate_state_transition(PoolStatus::Resolved));
    assert!(pool_active.validate_state_transition(PoolStatus::Closed));
    assert!(!pool_active.validate_state_transition(PoolStatus::Disputed));

    let pool_resolved = Pool {
        status: PoolStatus::Resolved,
        end_time: 100,
    };
    assert!(pool_resolved.validate_state_transition(PoolStatus::Disputed));
    assert!(!pool_resolved.validate_state_transition(PoolStatus::Active));
}
