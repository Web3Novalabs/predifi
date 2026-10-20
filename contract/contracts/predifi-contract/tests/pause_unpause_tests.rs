#![cfg(test)]

use predifi_contract::{Contract, ContractState, AdminAction};
use predifi_errors::PrediFiError;

#[test]
fn test_pause_contract_by_admin() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    contract.pause("admin".to_string()).expect("Should pause");
    assert_eq!(contract.is_paused(), true);
}

#[test]
fn test_pause_already_paused_contract_returns_error() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    contract.pause("admin".to_string()).expect("Should pause first time");
    let result = contract.pause("admin".to_string());
    assert_eq!(result, Err(PrediFiError::AlreadyPaused));
}

#[test]
fn test_unpause_paused_contract_by_admin() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    contract.pause("admin".to_string()).expect("Should pause");
    contract.unpause("admin".to_string()).expect("Should unpause");
    assert_eq!(contract.is_paused(), false);
}

#[test]
fn test_unpause_already_unpaused_contract_returns_error() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let result = contract.unpause("admin".to_string());
    assert_eq!(result, Err(PrediFiError::NotPaused));
}

#[test]
fn test_pause_by_non_admin_returns_unauthorized() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let result = contract.pause("user".to_string());
    assert_eq!(result, Err(PrediFiError::Unauthorized));
}

#[test]
fn test_unpause_by_non_admin_returns_unauthorized() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    contract.pause("admin".to_string()).expect("Should pause");
    let result = contract.unpause("user".to_string());
    assert_eq!(result, Err(PrediFiError::Unauthorized));
}

#[test]
fn test_operations_fail_when_contract_paused() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    contract.pause("admin".to_string()).expect("Should pause");

    // Test that predictions fail
    let predict_result = contract.place_prediction("user".to_string(), 100, "UP".to_string());
    assert_eq!(predict_result, Err(PrediFiError::ContractPaused));

    // Test that refunds fail
    let refund_result = contract.request_refund("user".to_string(), 1);
    assert_eq!(refund_result, Err(PrediFiError::ContractPaused));

    // Test that pool creation fails
    let pool_result = contract.create_pool("pool".to_string(), 100, 1000, 50);
    assert_eq!(pool_result, Err(PrediFiError::ContractPaused));
}

#[test]
fn test_operations_succeed_when_contract_unpaused() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    contract.pause("admin".to_string()).expect("Should pause");
    contract.unpause("admin".to_string()).expect("Should unpause");

    // Test that predictions work
    let predict_result = contract.place_prediction("user".to_string(), 100, "UP".to_string());
    assert!(predict_result.is_ok());
}

#[test]
fn test_multiple_pause_unpause_cycles() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    for _ in 0..5 {
        contract.pause("admin".to_string()).expect("Should pause");
        assert_eq!(contract.is_paused(), true);

        contract.unpause("admin".to_string()).expect("Should unpause");
        assert_eq!(contract.is_paused(), false);
    }
}

#[test]
fn test_pause_state_consistency_after_failed_operation() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    contract.pause("admin".to_string()).expect("Should pause");

    // Try an operation that fails due to pause
    let _ = contract.place_prediction("user".to_string(), 100, "UP".to_string());

    // Verify contract is still paused
    assert_eq!(contract.is_paused(), true);
}

#[test]
fn test_pause_status_query() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    assert_eq!(contract.is_paused(), false);

    contract.pause("admin".to_string()).expect("Should pause");
    assert_eq!(contract.is_paused(), true);
    assert_eq!(contract.get_pause_status(), ContractState::Paused);

    contract.unpause("admin".to_string()).expect("Should unpause");
    assert_eq!(contract.is_paused(), false);
    assert_eq!(contract.get_pause_status(), ContractState::Active);
}

#[test]
fn test_pause_with_active_predictions() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    // Place some predictions
    contract.place_prediction("user1".to_string(), 100, "UP".to_string()).expect("Should place");
    contract.place_prediction("user2".to_string(), 50, "DOWN".to_string()).expect("Should place");

    // Pause the contract
    contract.pause("admin".to_string()).expect("Should pause");

    // Verify pause doesn't affect existing prediction data
    let predictions = contract.get_active_predictions();
    assert_eq!(predictions.len(), 2);

    // But new predictions should fail
    let result = contract.place_prediction("user3".to_string(), 75, "UP".to_string());
    assert_eq!(result, Err(PrediFiError::ContractPaused));
}

#[test]
fn test_pause_emergency_stop_scenario() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    // Simulate emergency: pause immediately
    contract.pause("admin".to_string()).expect("Should pause");
    assert_eq!(contract.is_paused(), true);

    // All operations blocked
    assert!(contract.place_prediction("user".to_string(), 100, "UP".to_string()).is_err());
    assert!(contract.create_pool("pool".to_string(), 100, 1000, 50).is_err());
    assert!(contract.request_refund("user".to_string(), 1).is_err());

    // Resume operations
    contract.unpause("admin".to_string()).expect("Should unpause");
    assert!(contract.place_prediction("user".to_string(), 100, "UP".to_string()).is_ok());
}
