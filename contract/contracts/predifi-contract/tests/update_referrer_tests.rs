#![cfg(test)]

use predifi_contract::{Contract, ReferralTracker, ReferrerCommission};
use predifi_errors::PrediFiError;

#[test]
fn test_set_valid_referrer() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let result = contract.update_referrer("user1".to_string(), "valid_referrer".to_string());
    assert!(result.is_ok());

    let referrer = contract.get_referrer("user1").expect("Should get referrer");
    assert_eq!(referrer, "valid_referrer");
}

#[test]
fn test_set_referrer_to_self_returns_error() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let result = contract.update_referrer("user1".to_string(), "user1".to_string());
    assert_eq!(result, Err(PrediFiError::InvalidReferrer));
}

#[test]
fn test_set_referrer_to_invalid_address_returns_error() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    // Empty address
    let result = contract.update_referrer("user1".to_string(), "".to_string());
    assert_eq!(result, Err(PrediFiError::InvalidAddress));

    // Invalid format
    let result = contract.update_referrer("user1".to_string(), "not_an_address!@#".to_string());
    assert_eq!(result, Err(PrediFiError::InvalidAddress));
}

#[test]
fn test_change_referrer_after_predictions_placed() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("TEST/USD".to_string(), 1000000, 100000, 100).expect("Should create pool");

    // Set initial referrer
    contract.update_referrer("user1".to_string(), "ref1".to_string()).expect("Should set referrer");

    // Place prediction
    contract.place_prediction("user1".to_string(), 500, "UP".to_string()).expect("Should place prediction");

    // Change referrer
    contract.update_referrer("user1".to_string(), "ref2".to_string()).expect("Should change referrer");

    let new_referrer = contract.get_referrer("user1").expect("Should get referrer");
    assert_eq!(new_referrer, "ref2");

    // Verify future commissions go to new referrer
    contract.place_prediction("user1".to_string(), 300, "DOWN".to_string()).expect("Should place prediction");

    let ref1_commission = contract.get_referral_commission("ref1").expect("Should get commission");
    let ref2_commission = contract.get_referral_commission("ref2").expect("Should get commission");

    assert!(ref2_commission > ref1_commission);
}

#[test]
fn test_referral_chain_detection() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    // Create a simple chain: A <- B <- C
    contract.update_referrer("A".to_string(), "B".to_string()).expect("Should set A's referrer");
    contract.update_referrer("B".to_string(), "C".to_string()).expect("Should set B's referrer");

    // Try to create cycle: C <- A (would create A -> B -> C -> A)
    let result = contract.update_referrer("C".to_string(), "A".to_string());
    assert_eq!(result, Err(PrediFiError::ReferralCycle));
}

#[test]
fn test_self_referral_cycle_detection() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    // Try to set user as own referrer
    let result = contract.update_referrer("user1".to_string(), "user1".to_string());
    assert_eq!(result, Err(PrediFiError::InvalidReferrer));
}

#[test]
fn test_multi_hop_cycle_detection() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    // Build chain: A -> B -> C -> D
    contract.update_referrer("A".to_string(), "B".to_string()).expect("Should work");
    contract.update_referrer("B".to_string(), "C".to_string()).expect("Should work");
    contract.update_referrer("C".to_string(), "D".to_string()).expect("Should work");

    // Try to create cycle: D -> A would close the loop
    let result = contract.update_referrer("D".to_string(), "A".to_string());
    assert_eq!(result, Err(PrediFiError::ReferralCycle));
}

#[test]
fn test_referral_volume_tracking() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("TRACK/USD".to_string(), 2000000, 200000, 100).expect("Should create pool");

    // Set referrer
    contract.update_referrer("user1".to_string(), "referrer1".to_string()).expect("Should set");

    // Place predictions with different amounts
    contract.place_prediction("user1".to_string(), 500, "UP".to_string()).expect("Should place");
    contract.place_prediction("user1".to_string(), 300, "DOWN".to_string()).expect("Should place");
    contract.place_prediction("user1".to_string(), 200, "UP".to_string()).expect("Should place");

    // Verify referral volume
    let volume = contract.get_referral_volume("referrer1").expect("Should get volume");
    assert_eq!(volume, 1000);
}

#[test]
fn test_referral_commission_tracking_accuracy() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("COMM/USD".to_string(), 1000000, 100000, 100).expect("Should create pool");

    let commission_rate = contract.get_commission_rate(); // Assume 5%

    contract.update_referrer("user1".to_string(), "ref_tracker".to_string()).expect("Should set");
    contract.place_prediction("user1".to_string(), 1000, "UP".to_string()).expect("Should place");

    let expected_commission = 1000 * commission_rate / 100;
    let actual_commission = contract.get_referral_commission("ref_tracker").expect("Should get");

    assert_eq!(actual_commission, expected_commission as u128);
}

#[test]
fn test_multiple_referees_per_referrer() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("MULTI/USD".to_string(), 5000000, 500000, 100).expect("Should create pool");

    // Multiple users refer to same referrer
    for i in 0..10 {
        let user = format!("user_{}", i);
        contract.update_referrer(user, "popular_ref".to_string()).expect("Should set");
    }

    // All place predictions
    for i in 0..10 {
        let user = format!("user_{}", i);
        contract.place_prediction(user, 100, if i % 2 == 0 { "UP" } else { "DOWN" }.to_string()).expect("Should place");
    }

    // Verify referrer earned from all
    let total_volume = contract.get_referral_volume("popular_ref").expect("Should get");
    assert_eq!(total_volume, 1000); // 10 users * 100 each

    let total_commission = contract.get_referral_commission("popular_ref").expect("Should get");
    assert!(total_commission > 0);
}

#[test]
fn test_referrer_change_preserves_history() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("HIST/USD".to_string(), 1000000, 100000, 100).expect("Should create pool");

    // First referrer
    contract.update_referrer("user1".to_string(), "ref_old".to_string()).expect("Should set");
    contract.place_prediction("user1".to_string(), 500, "UP".to_string()).expect("Should place");

    let old_commission = contract.get_referral_commission("ref_old").expect("Should get");

    // Change referrer
    contract.update_referrer("user1".to_string(), "ref_new".to_string()).expect("Should change");
    contract.place_prediction("user1".to_string(), 300, "DOWN".to_string()).expect("Should place");

    // Old referrer keeps their commission
    let final_old_commission = contract.get_referral_commission("ref_old").expect("Should get");
    assert_eq!(final_old_commission, old_commission);

    // New referrer earns from new prediction
    let new_commission = contract.get_referral_commission("ref_new").expect("Should get");
    assert!(new_commission > 0);
}

#[test]
fn test_zero_referral_volume_calculation() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    // Set referrer but don't place any predictions
    contract.update_referrer("inactive_user".to_string(), "ref_inactive".to_string()).expect("Should set");

    let volume = contract.get_referral_volume("ref_inactive").expect("Should get");
    assert_eq!(volume, 0);

    let commission = contract.get_referral_commission("ref_inactive").expect("Should get");
    assert_eq!(commission, 0);
}

#[test]
fn test_referrer_query_for_unknown_address() {
    let contract = Contract::new(Admin::new("admin".to_string()));

    let result = contract.get_referrer("unknown_user".to_string());
    assert_eq!(result, Err(PrediFiError::UserNotFound));
}

#[test]
fn test_circular_referral_complex_scenario() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));

    // Create complex network that should allow acyclic structure
    contract.update_referrer("A".to_string(), "root".to_string()).expect("A -> root");
    contract.update_referrer("B".to_string(), "A".to_string()).expect("B -> A");
    contract.update_referrer("C".to_string(), "A".to_string()).expect("C -> A (allows multiple refs)");
    contract.update_referrer("D".to_string(), "B".to_string()).expect("D -> B");

    // But prevent any backward link
    let result = contract.update_referrer("root".to_string(), "A".to_string());
    assert_eq!(result, Err(PrediFiError::ReferralCycle));
}

#[test]
fn test_commission_distribution_multiple_referrers() {
    let mut contract = Contract::new(Admin::new("admin".to_string()));
    let pool = contract.create_pool("DIST/USD".to_string(), 10000000, 1000000, 100).expect("Should create pool");

    // 3 different referrers with their respective users
    for i in 0..3 {
        let referrer = format!("referrer_{}", i);
        for j in 0..5 {
            let user = format!("user_{}_{}", i, j);
            contract.update_referrer(user.clone(), referrer.clone()).expect("Should set");
            contract.place_prediction(user, 200, "UP".to_string()).expect("Should place");
        }
    }

    // Each referrer should have earned commissions
    for i in 0..3 {
        let referrer = format!("referrer_{}", i);
        let commission = contract.get_referral_commission(referrer).expect("Should get");
        assert!(commission > 0);
    }
}
