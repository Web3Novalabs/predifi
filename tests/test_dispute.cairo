use contract::base::events::Events::StakeRefunded;
use contract::base::types::Status;
use contract::interfaces::ipredifi::{
    IPredifiDispatcherTrait, IPredifiDisputeDispatcherTrait, IPredifiValidatorDispatcherTrait,
};
use contract::predifi::Predifi;
use core::array::ArrayTrait;
use core::serde::Serde;
use core::traits::TryInto;
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
use snforge_std::{
    EventSpyAssertionsTrait, spy_events, start_cheat_block_timestamp, start_cheat_caller_address,
    stop_cheat_block_timestamp, stop_cheat_caller_address,
};
use starknet::{ContractAddress, get_block_timestamp};
use super::test_utils::{approve_tokens_for_payment, create_default_pool, deploy_predifi};


// ================================================================================================
// DISPUTE TESTS
// ================================================================================================

#[test]
fn test_dispute_threshold_initial_value() {
    let (_, dispute_contract, _, _, _erc20_address) = deploy_predifi();

    let threshold = dispute_contract.get_dispute_threshold();
    assert(threshold == 3, 'Default threshold should be 3');
}

#[test]
fn test_raise_dispute_success() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Create a user and raise dispute
    let user1 = 'user1'.try_into().unwrap();

    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Verify dispute was raised
    let dispute_count = dispute_contract.get_dispute_count(pool_id);
    assert(dispute_count == 1, 'Dispute count should be 1');

    let has_disputed = dispute_contract.has_user_disputed(pool_id, user1);
    assert(has_disputed, 'User should have disputed');

    // Pool should still be active (threshold not reached)
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Active, 'Pool should still be active');
}

#[test]
fn test_raise_dispute_threshold_reached() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Create users and raise disputes to reach threshold
    let user1 = 'user1'.try_into().unwrap();
    let user2 = 'user2'.try_into().unwrap();
    let user3 = 'user3'.try_into().unwrap();

    // First dispute
    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Second dispute
    start_cheat_caller_address(dispute_contract.contract_address, user2);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Third dispute - should trigger suspension
    start_cheat_caller_address(dispute_contract.contract_address, user3);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Verify pool is suspended
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Suspended, 'Pool should be suspended');

    let dispute_count = dispute_contract.get_dispute_count(pool_id);
    assert(dispute_count == 3, 'Dispute count should be 3');

    let is_suspended = dispute_contract.is_pool_suspended(pool_id);
    assert(is_suspended, 'Pool should be suspended');
}

#[test]
#[should_panic(expected: 'User already raised dispute')]
fn test_raise_dispute_already_disputed() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    let user1 = 'user1'.try_into().unwrap();

    // Raise dispute first time
    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool_id);

    // Try to raise dispute again (should panic)
    dispute_contract.raise_dispute(pool_id);
}

#[test]
#[should_panic(expected: 'Pool does not exist')]
fn test_raise_dispute_nonexistent_pool() {
    let (_, dispute_contract, _, _pool_creator, _erc20_address) = deploy_predifi();

    let user1 = 'user1'.try_into().unwrap();
    let nonexistent_pool_id = 999999;

    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(nonexistent_pool_id);
}

#[test]
#[should_panic(expected: 'Pool is suspended')]
fn test_raise_dispute_already_suspended() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup and create pool
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Reach threshold to suspend pool
    let user1 = 'user1'.try_into().unwrap();
    let user2 = 'user2'.try_into().unwrap();
    let user3 = 'user3'.try_into().unwrap();
    let user4 = 'user4'.try_into().unwrap();

    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user2);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user3);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Try to raise dispute on suspended pool
    start_cheat_caller_address(dispute_contract.contract_address, user4);
    dispute_contract.raise_dispute(pool_id);
}

#[test]
fn test_resolve_dispute_success() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Get the initial status
    let initial_pool = contract.get_pool(pool_id);
    let initial_status = initial_pool.status;

    // Suspend pool by reaching threshold
    let user1 = 'user1'.try_into().unwrap();
    let user2 = 'user2'.try_into().unwrap();
    let user3 = 'user3'.try_into().unwrap();

    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user2);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user3);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Verify suspension
    let suspended_pool = contract.get_pool(pool_id);
    assert(suspended_pool.status == Status::Suspended, 'Pool should be suspended');

    // Admin resolves dispute
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(dispute_contract.contract_address, admin);
    dispute_contract.resolve_dispute(pool_id, true);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Verify resolution
    let resolved_pool = contract.get_pool(pool_id);
    assert(resolved_pool.status == initial_status, 'Status should be restored');

    let dispute_count = dispute_contract.get_dispute_count(pool_id);
    assert(dispute_count == 0, 'Dispute count should be reset');

    let is_suspended = dispute_contract.is_pool_suspended(pool_id);
    assert(!is_suspended, 'Pool should not be suspended');
}

#[test]
#[should_panic(expected: 'Pool is not suspended')]
fn test_resolve_dispute_not_suspended() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Try to resolve dispute on non-suspended pool
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(dispute_contract.contract_address, admin);
    dispute_contract.resolve_dispute(pool_id, true);
}

#[test]
fn test_get_suspended_pools() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create two pools
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool1_id = create_default_pool(contract);
    create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Suspend only first pool
    let user1 = 'user1'.try_into().unwrap();
    let user2 = 'user2'.try_into().unwrap();
    let user3 = 'user3'.try_into().unwrap();

    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool1_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user2);
    dispute_contract.raise_dispute(pool1_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user3);
    dispute_contract.raise_dispute(pool1_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Check suspended pools
    let suspended_pools = dispute_contract.get_suspended_pools();
    assert(suspended_pools.len() == 1, 'Should have 1 suspended pool');

    let suspended_pool = suspended_pools.at(0);
    assert(*suspended_pool.pool_id == pool1_id, 'Wrong pool suspended');
}

#[test]
#[should_panic(expected: 'Pool is inactive')]
fn test_vote_on_suspended_pool() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create pool with proper timestamps
    let current_time = get_block_timestamp();
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A test pool for suspension",
            "image.png",
            "event.com/details",
            current_time + 100, // Start time
            current_time + 200, // Lock time
            current_time + 300, // End time
            'Team A',
            'Team B',
            100,
            10000,
            5,
            false,
            0,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Verify pool exists and is active
    let initial_pool = contract.get_pool(pool_id);
    assert(initial_pool.exists, 'Pool should exist');
    assert(initial_pool.status == Status::Active, 'Pool should be active');

    // Suspend pool by raising enough disputes
    let user1 = 'user1'.try_into().unwrap();
    let user2 = 'user2'.try_into().unwrap();
    let user3 = 'user3'.try_into().unwrap();

    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user2);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user3);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Verify pool is suspended
    let suspended_pool = contract.get_pool(pool_id);
    assert(suspended_pool.status == Status::Suspended, 'Pool should be suspended');

    // Try to vote on suspended pool (should panic)
    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract.vote(pool_id, 'Team A', 200);
}

#[test]
#[should_panic(expected: 'Pool is suspended')]
fn test_stake_on_suspended_pool() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Suspend pool
    let user1 = 'user1'.try_into().unwrap();
    let user2 = 'user2'.try_into().unwrap();
    let user3 = 'user3'.try_into().unwrap();

    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user2);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user3);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Try to stake on suspended pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract.stake(pool_id, 200_000_000_000_000_000_000);
}

#[test]
fn test_refund_stake_successful() {
    let (contract, _, _, caller, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, caller);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, caller);
    let pool_id = create_default_pool(contract);
    let stake_amount: u256 = 200_000_000_000_000_000_000;

    contract.stake(pool_id, stake_amount);
    contract.cancel_pool(pool_id);

    contract.refund_stake(pool_id);
    assert(contract.get_user_stake(pool_id, caller).amount == 0, 'Invalid stake amount');
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_refund_stake_event_emission() {
    let (contract, _, _, caller, erc20_address) = deploy_predifi();
    let mut spy = spy_events();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, caller);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, caller);
    let pool_id = create_default_pool(contract);
    let stake_amount: u256 = 200_000_000_000_000_000_000;

    contract.stake(pool_id, stake_amount);
    contract.cancel_pool(pool_id);

    contract.refund_stake(pool_id);
    assert(contract.get_user_stake(pool_id, caller).amount == 0, 'Invalid stake amount');
    stop_cheat_caller_address(contract.contract_address);

    // Assert event emitted
    let expected_event = Predifi::Event::StakeRefunded(
        StakeRefunded { pool_id, address: caller, amount: stake_amount },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_event)]);
}

#[test]
#[should_panic(expected: 'Pool is not closed')]
fn test_refund_stake_on_open_pool() {
    let (contract, _, _, caller, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, caller);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, caller);
    let pool_id = create_default_pool(contract);
    let stake_amount: u256 = 200_000_000_000_000_000_000;

    contract.stake(pool_id, stake_amount);
    stop_cheat_caller_address(contract.contract_address);

    contract.refund_stake(pool_id);
    assert(contract.get_user_stake(pool_id, caller).amount == 0, 'Invalid stake amount');
}

#[test]
#[should_panic(expected: 'Zero user stake')]
fn test_refund_zero_stake() {
    let (contract, _, _, caller, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, caller);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, caller);
    let pool_id = create_default_pool(contract);
    contract.cancel_pool(pool_id);

    contract.refund_stake(pool_id);
    assert(contract.get_user_stake(pool_id, caller).amount == 0, 'Invalid stake amount');
    stop_cheat_caller_address(contract.contract_address);
}


// ================================================================================================
// VALIDATOR TESTS
// ================================================================================================

#[test]
fn test_validate_outcome_success() {
    let (contract, dispute_contract, validator_contract, pool_creator, erc20_address) =
        deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Get current time and create pool with relative timestamps
    let current_time = get_block_timestamp();
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A test pool for validation",
            "image.png",
            "event.com/details",
            current_time + 100, // Start time
            current_time + 200, // Lock time
            current_time + 300, // End time
            'Team A',
            'Team B',
            100,
            10000,
            5,
            false,
            0,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Move time to after lock time but before end time
    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Verify pool is locked
    let locked_pool = contract.get_pool(pool_id);
    assert(locked_pool.status == Status::Locked, 'Pool should be locked');

    // Add a validator and validate outcome
    let admin = 'admin'.try_into().unwrap();
    let validator = 'validator'.try_into().unwrap();
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator);
    stop_cheat_caller_address(validator_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, validator);
    dispute_contract.validate_outcome(pool_id, true);
    stop_cheat_caller_address(dispute_contract.contract_address);

    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Locked, 'Pool should remain locked');
}

#[test]
#[should_panic(expected: 'Pool is suspended')]
fn test_validate_outcome_suspended_pool() {
    let (contract, dispute_contract, validator_contract, pool_creator, erc20_address) =
        deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Suspend pool
    let user1 = 'user1'.try_into().unwrap();
    let user2 = 'user2'.try_into().unwrap();
    let user3 = 'user3'.try_into().unwrap();

    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user2);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user3);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Try to validate suspended pool
    let admin = 'admin'.try_into().unwrap();
    let validator = 'admin'.try_into().unwrap();
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator);
    stop_cheat_caller_address(validator_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, validator);
    dispute_contract.validate_outcome(pool_id, true);
}

#[test]
#[should_panic(expected: 'Pool is suspended')]
fn test_claim_reward_suspended_pool() {
    let (contract, dispute_contract, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Suspend pool
    let user1 = 'user1'.try_into().unwrap();
    let user2 = 'user2'.try_into().unwrap();
    let user3 = 'user3'.try_into().unwrap();

    start_cheat_caller_address(dispute_contract.contract_address, user1);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user2);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, user3);
    dispute_contract.raise_dispute(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Try to claim reward on suspended pool
    start_cheat_caller_address(dispute_contract.contract_address, pool_creator);
    dispute_contract.claim_reward(pool_id);
}


#[test]
#[should_panic(expected: 'Pausable: paused')]
fn test_predify_contract_pause_success() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract (by admin)
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.pause();
    stop_cheat_caller_address(validator_contract.contract_address);

    // Try to create a pool while paused
    let _ = create_default_pool(contract);
}

#[test]
#[should_panic(expected: 'Caller is missing role')]
fn test_non_admin_pause_predify_contract() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract by non-admin
    start_cheat_caller_address(validator_contract.contract_address, pool_creator);
    validator_contract.pause();
    stop_cheat_caller_address(validator_contract.contract_address);
}


#[test]
#[should_panic(expected: 'Pausable: not paused')]
fn test_unpause_not_paused_predify_contract() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.unpause();
}

#[test]
#[should_panic(expected: 'Pausable: paused')]
fn test_pause_paused_predify_contract() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract
    start_cheat_caller_address(contract.contract_address, admin);
    validator_contract.pause();
    validator_contract.pause();
}

#[test]
fn test_predify_contract_unpause_success() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract
    start_cheat_caller_address(contract.contract_address, admin);
    validator_contract.pause();
    stop_cheat_caller_address(contract.contract_address);

    // Unpause the contract
    start_cheat_caller_address(contract.contract_address, admin);
    validator_contract.unpause();

    // Create a pool after unpausing
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "Pool not created successfully");
}

#[test]
#[should_panic(expected: 'Pausable: paused')]
fn test_validate_pool_result_paused() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract using the validator dispatcher
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.pause();

    // Try to validate pool result while paused - should panic
    validator_contract.validate_pool_result(1, true);
}

#[test]
#[should_panic(expected: 'Pausable: paused')]
fn test_claim_reward_paused() {
    let (contract, dispute_contract, validator_contract, pool_creator, erc20_address) =
        deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract using the validator dispatcher
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.pause();
    stop_cheat_caller_address(validator_contract.contract_address);

    // Try to claim reward while paused - should panic
    start_cheat_caller_address(dispute_contract.contract_address, admin);
    dispute_contract.claim_reward(1);
}

#[test]
#[should_panic(expected: 'Pausable: paused')]
fn test_refund_stake_paused() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract using the validator dispatcher
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.pause();
    stop_cheat_caller_address(validator_contract.contract_address);

    // Try to refund stake while paused - should panic
    start_cheat_caller_address(contract.contract_address, admin);
    contract.refund_stake(1);
}
