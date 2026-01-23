#![cfg(test)]
use super::*;
use soroban_sdk::{Env, testutils::Ledger};

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

    // Test active pool expired
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

#[test]
fn test_create_pool_success() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let pool_id = 1;
    let end_time = 100;

    client.create_pool(&pool_id, &end_time);

    let pool = client.get_pool(&pool_id).unwrap();
    assert_eq!(pool.status, PoolStatus::Active);
    assert_eq!(pool.end_time, end_time);
}

#[test]
fn test_create_pool_invalid_end_time() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let result = client.try_create_pool(&1, &0);
    assert_eq!(result, Err(Ok(Error::InvalidEndTime)));
}

#[test]
fn test_submit_prediction_success() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let pool_id = 1;
    let end_time = 100;
    client.create_pool(&pool_id, &end_time);

    // Mock ledger time (default is 0)
    env.ledger().set_timestamp(50);

    let result = client.try_submit_prediction(&pool_id);
    assert!(result.is_ok());
}

#[test]
fn test_submit_prediction_after_deadline() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let pool_id = 1;
    let end_time = 100;
    client.create_pool(&pool_id, &end_time);

    // Set ledger time past the deadline
    env.ledger().set_timestamp(101);

    let result = client.try_submit_prediction(&pool_id);
    assert_eq!(result, Err(Ok(Error::DeadlinePassed)));
}

#[test]
fn test_submit_prediction_pool_not_found() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let result = client.try_submit_prediction(&1);
    assert_eq!(result, Err(Ok(Error::PoolNotFound)));
}
