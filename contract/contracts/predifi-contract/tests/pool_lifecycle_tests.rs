#![cfg(test)]

use predifi_contract::{Contract, Pool, PoolState, PoolLifecycle};
use predifi_errors::PrediFiError;

#[test]
fn test_mark_pool_ready_transitions_state() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("LIFE/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    assert_eq!(pool.get_state(), PoolState::Created);

    pool.mark_ready().expect("Should mark ready");
    assert_eq!(pool.get_state(), PoolState::Ready);
}

#[test]
fn test_mark_already_ready_pool_returns_error() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("READY/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    pool.mark_ready().expect("Should mark ready first time");

    let result = pool.mark_ready();
    assert_eq!(result, Err(PrediFiError::PoolAlreadyReady));
}

#[test]
fn test_close_staking_on_ready_pool() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("STAKE/USD".to_string(), 2000000, 200000, 100).expect("Should create");

    pool.mark_ready().expect("Should mark ready");
    assert_eq!(pool.get_state(), PoolState::Ready);

    pool.close_staking().expect("Should close staking");
    assert_eq!(pool.get_state(), PoolState::StakingClosed);
}

#[test]
fn test_close_staking_on_not_ready_pool_returns_error() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("NOTREADY/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    // Try to close staking without marking ready
    let result = pool.close_staking();
    assert_eq!(result, Err(PrediFiError::PoolNotReady));
}

#[test]
fn test_double_close_staking_returns_error() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("DOUBLE/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    pool.mark_ready().expect("Should mark ready");
    pool.close_staking().expect("Should close staking first time");

    let result = pool.close_staking();
    assert_eq!(result, Err(PrediFiError::StakingAlreadyClosed));
}

#[test]
fn test_state_transition_ordering_violations() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("ORDER/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    // Try to close staking before marking ready
    let result = pool.close_staking();
    assert_eq!(result, Err(PrediFiError::PoolNotReady));

    // Try to resolve without closing staking
    let resolve_result = pool.resolve("UP".to_string());
    assert_eq!(resolve_result, Err(PrediFiError::PoolNotClosed));
}

#[test]
fn test_timestamp_boundary_conditions() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let current_time = contract.get_current_timestamp();

    // Create pool with timestamp in past (should still work)
    let past_time = current_time - 3600; // 1 hour ago
    let mut pool = contract.create_pool_with_timestamp(
        "TIME/USD".to_string(),
        1000000,
        100000,
        100,
        past_time,
    ).expect("Should create");

    pool.mark_ready().expect("Should mark ready");
    pool.close_staking().expect("Should close staking");
    pool.resolve("UP".to_string()).expect("Should resolve");
}

#[test]
fn test_pool_lifecycle_sequence() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("SEQ/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    // Verify state progression
    assert_eq!(pool.get_state(), PoolState::Created);

    pool.mark_ready().expect("Should mark ready");
    assert_eq!(pool.get_state(), PoolState::Ready);

    pool.close_staking().expect("Should close staking");
    assert_eq!(pool.get_state(), PoolState::StakingClosed);

    pool.resolve("DOWN".to_string()).expect("Should resolve");
    assert_eq!(pool.get_state(), PoolState::Resolved);
}

#[test]
fn test_pool_creation_with_boundary_timestamps() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    // Minimum timestamp (0)
    let pool1 = contract.create_pool_with_timestamp(
        "MIN/USD".to_string(),
        1000000,
        100000,
        100,
        0,
    );
    assert!(pool1.is_ok());

    // Maximum u64 timestamp
    let pool2 = contract.create_pool_with_timestamp(
        "MAX/USD".to_string(),
        1000000,
        100000,
        100,
        u64::MAX,
    );
    assert!(pool2.is_ok());
}

#[test]
fn test_staking_enabled_in_ready_state() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("ENABLE/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    pool.mark_ready().expect("Should mark ready");

    // Should be able to place predictions in Ready state
    let stake_result = contract.place_prediction("user1".to_string(), 100, "UP".to_string());
    assert!(stake_result.is_ok());
}

#[test]
fn test_staking_disabled_after_close() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("DISABLE/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    pool.mark_ready().expect("Should mark ready");
    pool.close_staking().expect("Should close staking");

    // Should NOT be able to place predictions after staking closed
    let stake_result = contract.place_prediction("user2".to_string(), 100, "DOWN".to_string());
    assert_eq!(stake_result, Err(PrediFiError::StakingClosed));
}

#[test]
fn test_multiple_pools_independent_states() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    let mut pool1 = contract.create_pool("POOL1/USD".to_string(), 1000000, 100000, 100).expect("Should create");
    let mut pool2 = contract.create_pool("POOL2/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    // Transition pool1
    pool1.mark_ready().expect("Should mark ready");
    pool1.close_staking().expect("Should close staking");

    // pool2 should still be in Created state
    assert_eq!(pool2.get_state(), PoolState::Created);

    // pool2 can independently transition
    pool2.mark_ready().expect("Should mark ready");
    assert_eq!(pool2.get_state(), PoolState::Ready);
}

#[test]
fn test_mark_ready_with_predictions_existing() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("PRED/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    // Place predictions before marking ready
    contract.place_prediction("user1".to_string(), 500, "UP".to_string()).expect("Should place");
    contract.place_prediction("user2".to_string(), 300, "DOWN".to_string()).expect("Should place");

    // Should still be able to mark ready
    pool.mark_ready().expect("Should mark ready with existing predictions");
    assert_eq!(pool.get_state(), PoolState::Ready);
}

#[test]
fn test_close_staking_snapshot_state() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("SNAP/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    pool.mark_ready().expect("Should mark ready");

    let total_staked_before = pool.get_total_staked();

    pool.close_staking().expect("Should close staking");

    let total_staked_after = pool.get_total_staked();

    // Total staked should be immutable after close
    assert_eq!(total_staked_before, total_staked_after);
}

#[test]
fn test_resolve_requires_closed_staking() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("RESOLVE/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    pool.mark_ready().expect("Should mark ready");

    // Try to resolve without closing staking
    let result = pool.resolve("UP".to_string());
    assert_eq!(result, Err(PrediFiError::PoolNotClosed));

    // Now close staking
    pool.close_staking().expect("Should close staking");

    // Should be able to resolve
    let resolve_result = pool.resolve("UP".to_string());
    assert!(resolve_result.is_ok());
}

#[test]
fn test_pool_state_persistence_across_operations() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("PERSIST/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    // Perform multiple operations
    contract.place_prediction("user1".to_string(), 100, "UP".to_string()).expect("Should place");
    pool.mark_ready().expect("Should mark ready");
    contract.place_prediction("user2".to_string(), 50, "DOWN".to_string()).expect("Should place");
    pool.close_staking().expect("Should close staking");

    // Verify state is persistent
    assert_eq!(pool.get_state(), PoolState::StakingClosed);

    // Resolve
    pool.resolve("UP".to_string()).expect("Should resolve");
    assert_eq!(pool.get_state(), PoolState::Resolved);
}

#[test]
fn test_invalid_outcome_on_resolve() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let mut pool = contract.create_pool("OUTCOME/USD".to_string(), 1000000, 100000, 100).expect("Should create");

    pool.mark_ready().expect("Should mark ready");
    pool.close_staking().expect("Should close staking");

    // Try to resolve with invalid outcome
    let result = pool.resolve("SIDEWAYS".to_string());
    assert_eq!(result, Err(PrediFiError::InvalidOutcome));
}
