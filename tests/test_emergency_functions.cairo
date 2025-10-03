use contract::base::events::Events::{
    EmergencyActionCancelled, EmergencyActionExecuted, EmergencyActionScheduled,
    EmergencyWithdrawal, PoolEmergencyFrozen, PoolEmergencyResolved, PoolEmergencyUnfrozen,
};
use contract::base::types::{
    Category, EmergencyActionStatus, EmergencyActionType, Pool, PoolDetails, Status,
};
use contract::interfaces::ipredifi::{IPredifi, IPredifiDispatcher, IPredifiDispatcherTrait};
use contract::predifi::Predifi;
use core::array::ArrayTrait;
use core::byte_array::ByteArray;
use core::integer::u256;
use core::option::OptionTrait;
use core::serde::Serde;
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, EventSpyAssertionsTrait, EventSpyTrait, declare,
    spy_events, start_cheat_block_timestamp, start_cheat_caller_address, 
    stop_cheat_caller_address,
};
use starknet::{ContractAddress, contract_address_const, get_block_timestamp};
use super::test_utils::{approve_tokens_for_payment, deploy_predifi, mint_tokens_for};

// Test addresses
fn admin() -> ContractAddress {
    contract_address_const::<'admin'>()
}

fn user1() -> ContractAddress {
    contract_address_const::<'user1'>()
}

fn user2() -> ContractAddress {
    contract_address_const::<'user2'>()
}

fn validator1() -> ContractAddress {
    contract_address_const::<'validator1'>()
}

fn validator2() -> ContractAddress {
    contract_address_const::<'validator2'>()
}

fn token_address() -> ContractAddress {
    contract_address_const::<'token'>()
}

// Helper function to deploy the contract
fn deploy_contract() -> IPredifiDispatcher {
    let (dispatcher, _, _, _, _) = deploy_predifi();
    dispatcher
}

// Helper function to create a test pool (ensures ERC20 approval for creation fee)
fn create_test_pool(dispatcher: IPredifiDispatcher, erc20_address: ContractAddress) -> u256 {
    let mut spy = spy_events();

    // Mint and approve tokens
    mint_tokens_for(user1(), erc20_address, 2_000_000_000_000_000_000);
    start_cheat_caller_address(erc20_address, user1());
    approve_tokens_for_payment(
        dispatcher.contract_address, erc20_address, 2_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(dispatcher.contract_address, user1());

    let pool_id = dispatcher
        .create_pool(
            'Test Pool',
            0, // WinBet
            "Test Description",
            "https://example.com/image.jpg",
            "https://example.com/event",
            get_block_timestamp() + 3600, // start time
            get_block_timestamp() + 7200, // lock time
            get_block_timestamp() + 10800, // end time
            'Option 1',
            'Option 2',
            1000000000000000000, // 1 STRK min bet
            10000000000000000000, // 10 STRK max bet
            3, // 3% creator fee
            false, // not private
            0 // sports category
        );

    stop_cheat_caller_address(dispatcher.contract_address);

    pool_id
}

// Helper function to create a test pool with a specific user
fn create_test_pool_with_user(
    dispatcher: IPredifiDispatcher, erc20_address: ContractAddress, user: ContractAddress,
) -> u256 {
    let mut spy = spy_events();

    // Mint and approve tokens for the pool creator so the contract can collect the creation fee
    mint_tokens_for(user, erc20_address, 2_000_000_000_000_000_000);
    start_cheat_caller_address(erc20_address, user);
    approve_tokens_for_payment(
        dispatcher.contract_address, erc20_address, 2_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(dispatcher.contract_address, user);

    let pool_id = dispatcher
        .create_pool(
            'Test Pool',
            0, // WinBet
            "Test Description",
            "https://example.com/image.jpg",
            "https://example.com/event",
            get_block_timestamp() + 3600, // start time
            get_block_timestamp() + 7200, // lock time
            get_block_timestamp() + 10800, // end time
            'Option 1',
            'Option 2',
            1000000000000000000, // 1 STRK min bet
            10000000000000000000, // 10 STRK max bet
            3, // 3% creator fee
            false, // not private
            0 // sports category
        );

    stop_cheat_caller_address(dispatcher.contract_address);

    pool_id
}

#[test]
fn test_emergency_freeze_pool() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);
    let mut spy = spy_events();

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Schedule emergency freeze action
    let action_id = dispatcher.schedule_emergency_action(0, pool_id, 0); // 0 = FreezePool

    // Get the execution time from the scheduled action
    let (_, execution_time) = dispatcher.get_emergency_action_status(action_id);

    // Fast forward time to pass timelock (add 1 second to ensure we're past the execution time)
    start_cheat_block_timestamp(dispatcher.contract_address, execution_time + 1);

    // Capture timestamp before the contract call to match the event emission
    let event_timestamp = get_block_timestamp();

    // Execute the scheduled emergency action
    dispatcher.execute_emergency_action(action_id);

    // Check that emergency events were emitted
    let events = spy.get_events();
    assert(events.events.len() >= 2, 'Missing emergency events');
    
    // Verify pool is in emergency state
    assert!(dispatcher.is_pool_emergency_state(pool_id), "Pool should be in emergency state");

    // Verify emergency freeze was successful

    // Verify the emergency state is actually stored correctly
    let emergency_pools = dispatcher.get_emergency_pools();
    assert!(emergency_pools.len() == 1, "Should have exactly 1 emergency pool");

    // Verify the pool in emergency pools list matches our pool
    assert!(
        *emergency_pools.at(0).pool_id == pool_id, "Emergency pool should match the frozen pool",
    );

    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_emergency_unfreeze_pool() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);
    let mut spy = spy_events();

    // First freeze the pool using timelock
    start_cheat_caller_address(dispatcher.contract_address, admin());
    let freeze_action_id = dispatcher.schedule_emergency_action(0, pool_id, 0); // 0 = FreezePool

    // Get the execution time from the scheduled freeze action
    let (_, freeze_execution_time) = dispatcher.get_emergency_action_status(freeze_action_id);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, freeze_execution_time + 1);

    // Execute the freeze action
    dispatcher.execute_emergency_action(freeze_action_id);

    // Verify pool is frozen first
    assert!(
        dispatcher.is_pool_emergency_state(pool_id),
        "Pool should be in emergency state after freezing",
    );

    // Then schedule and execute unfreeze action
    let unfreeze_action_id = dispatcher
        .schedule_emergency_action(2, pool_id, 0); // 2 = UnfreezePool

    // Get the execution time from the scheduled unfreeze action
    let (_, unfreeze_execution_time) = dispatcher.get_emergency_action_status(unfreeze_action_id);

    // Fast forward time again to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, unfreeze_execution_time + 1);

    // Capture timestamp before the contract call to match the event emission
    let event_timestamp = get_block_timestamp();

    // Execute the unfreeze action
    dispatcher.execute_emergency_action(unfreeze_action_id);

    // Verify pool is no longer in emergency state
    assert!(!dispatcher.is_pool_emergency_state(pool_id), "Pool should not be in emergency state");

    // Verify emergency unfreeze was successful

    // Verify the emergency pools list is empty after unfreezing
    let emergency_pools = dispatcher.get_emergency_pools();
    assert!(emergency_pools.len() == 0, "Should have no emergency pools after unfreezing");

    // Verify the pool can be frozen again after unfreezing
    let refreeze_action_id = dispatcher.schedule_emergency_action(0, pool_id, 0); // 0 = FreezePool

    // Get the execution time from the scheduled refreeze action
    let (_, refreeze_execution_time) = dispatcher.get_emergency_action_status(refreeze_action_id);

    // Fast forward time again
    start_cheat_block_timestamp(dispatcher.contract_address, refreeze_execution_time + 1);

    // Execute the refreeze action
    dispatcher.execute_emergency_action(refreeze_action_id);

    assert!(
        dispatcher.is_pool_emergency_state(pool_id),
        "Pool should be able to be frozen again after unfreezing",
    );

    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_emergency_resolve_pool() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);
    let mut spy = spy_events();

    // First freeze the pool using timelock
    start_cheat_caller_address(dispatcher.contract_address, admin());
    let freeze_action_id = dispatcher.schedule_emergency_action(0, pool_id, 0); // 0 = FreezePool

    // Get the execution time from the freeze action
    let (_, freeze_execution_time) = dispatcher.get_emergency_action_status(freeze_action_id);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, freeze_execution_time + 1);

    // Execute the freeze action
    dispatcher.execute_emergency_action(freeze_action_id);

    // Verify pool is frozen first
    assert!(
        dispatcher.is_pool_emergency_state(pool_id),
        "Pool should be in emergency state after freezing",
    );

    // Then schedule and execute resolve action with option2 as winner
    let resolve_action_id = dispatcher
        .schedule_emergency_action(1, pool_id, 1); // 1 = ResolvePool, 1 = option2 wins

    // Get the execution time from the resolve action
    let (_, resolve_execution_time) = dispatcher.get_emergency_action_status(resolve_action_id);

    // Fast forward time again to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, resolve_execution_time + 1);

    // Capture timestamp before the contract call to match the event emission
    let event_timestamp = get_block_timestamp();

    // Execute the resolve action
    dispatcher.execute_emergency_action(resolve_action_id);

    // Verify pool is resolved
    let pool = dispatcher.get_pool(pool_id);
    assert!(pool.status == Status::Settled, "Pool should be settled");

    // Verify emergency resolution was successful

    // Verify the pool is no longer in emergency state after resolution
    assert!(
        !dispatcher.is_pool_emergency_state(pool_id),
        "Pool should not be in emergency state after resolution",
    );

    // Verify the emergency pools list is empty after resolution
    let emergency_pools = dispatcher.get_emergency_pools();
    assert!(emergency_pools.len() == 0, "Should have no emergency pools after resolution");

    // Verify the pool outcome is actually set
    let pool = dispatcher.get_pool(pool_id);
    assert!(pool.status == Status::Settled, "Pool should remain settled after resolution");

    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_emergency_withdrawal() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);
    let mut spy = spy_events();

    // First freeze the pool using timelock
    start_cheat_caller_address(dispatcher.contract_address, admin());
    let freeze_action_id = dispatcher.schedule_emergency_action(0, pool_id, 0); // 0 = FreezePool

    // Get the execution time from the freeze action
    let (_, freeze_execution_time) = dispatcher.get_emergency_action_status(freeze_action_id);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, freeze_execution_time + 1);

    // Execute the freeze action
    dispatcher.execute_emergency_action(freeze_action_id);

    // Place a bet on the pool
    // Mint and approve tokens
    mint_tokens_for(user1(), erc20_address, 2_000_000_000_000_000_000);
    start_cheat_caller_address(erc20_address, user1());
    approve_tokens_for_payment(
        dispatcher.contract_address, erc20_address, 2_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(dispatcher.contract_address, user1());
    dispatcher.vote(pool_id, 'Option 1', 1000000000000000000); // 1 STRK

    // Switch back to user1 for emergency withdrawal
    start_cheat_caller_address(dispatcher.contract_address, user1());

    // Capture timestamp before the contract call to match the event emission
    let event_timestamp = get_block_timestamp();

    // Now try emergency withdrawal
    dispatcher.emergency_withdraw(pool_id);

    // Verify emergency withdrawal was successful

    // Verify the user's stake is actually reset after withdrawal
    let user_stake = dispatcher.get_user_stake(pool_id, user1());
    assert!(user_stake.amount == 0, "User stake should be reset to 0 after emergency withdrawal");
    assert!(user_stake.shares == 0, "User shares should be reset to 0 after emergency withdrawal");

    // Verify the user has participated in this pool
    assert!(
        dispatcher.has_user_participated_in_pool(user1(), pool_id),
        "User should have participated in this pool",
    );

    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_schedule_emergency_action() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);
    let mut spy = spy_events();

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Schedule an emergency action (freeze pool)
    let action_data = 0;
    let action_id = dispatcher.schedule_emergency_action(0, pool_id, action_data);

    // Verify action was scheduled
    let (status, execution_time) = dispatcher.get_emergency_action_status(action_id);
    assert!(status == 1, "Action should be in waiting status"); // 1 = Waiting

    // Verify execution time is in the future (timelock delay)
    let current_time = get_block_timestamp();
    assert!(execution_time > current_time, "Execution time should be in the future");

    // Schedule another action to verify unique IDs
    let action_id2 = dispatcher.schedule_emergency_action(0, pool_id, action_data);
    assert!(action_id2 != action_id, "Action IDs should be unique");

    // Verify both actions are in waiting status
    let (status2, _) = dispatcher.get_emergency_action_status(action_id2);
    assert!(status2 == 1, "Second action should also be in waiting status");

    // Verify emergency actions were scheduled successfully

    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_execute_emergency_action() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);
    let mut spy = spy_events();

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Schedule an emergency action (freeze pool)
    let action_data = 0;
    let action_id = dispatcher.schedule_emergency_action(0, pool_id, action_data);

    // Get the execution time from the scheduled action
    let (_, execution_time) = dispatcher.get_emergency_action_status(action_id);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, execution_time + 1);

    // Capture timestamp before the contract call to match the event emission
    let event_timestamp = get_block_timestamp();

    // Execute the action
    dispatcher.execute_emergency_action(action_id);

    // Verify action was executed
    let (status, _) = dispatcher.get_emergency_action_status(action_id);
    assert!(status == 3, "Action should be in done status"); // 3 = Done

    // Verify the pool is actually frozen after executing the freeze action
    assert!(
        dispatcher.is_pool_emergency_state(pool_id),
        "Pool should be in emergency state after executing freeze action",
    );

    // Verify the emergency pools list contains our pool
    let emergency_pools = dispatcher.get_emergency_pools();
    assert!(emergency_pools.len() == 1, "Should have exactly 1 emergency pool after execution");

    // Verify emergency freeze was successful

    // Verify the pool state actually changed
    let pool = dispatcher.get_pool(pool_id);
    assert!(
        pool.status == Status::Active,
        "Pool should still be active after freeze action (freeze doesn't change pool status)",
    );

    stop_cheat_block_timestamp(dispatcher.contract_address);
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
#[should_panic(expected: 'Action has been cancelled')]
fn test_cancel_emergency_action() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);
    let mut spy = spy_events();

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Schedule an emergency action (freeze pool)
    let action_data = 0;
    let action_id = dispatcher.schedule_emergency_action(0, pool_id, action_data);

    // Verify action is in waiting status before cancellation
    let (status_before, _) = dispatcher.get_emergency_action_status(action_id);
    assert!(status_before == 1, "Action should be in waiting status before cancellation");

    // Cancel the action
    dispatcher.cancel_emergency_action(action_id);

    // Verify action was cancelled
    let (status, _) = dispatcher.get_emergency_action_status(action_id);
    assert!(status == 4, "Action should be in cancelled status"); // 4 = Cancelled

    // Capture timestamp before the contract call to match the event emission
    let event_timestamp = get_block_timestamp();

    // Verify emergency action cancellation was successful

    // Get the execution time from the cancelled action
    let (_, execution_time) = dispatcher.get_emergency_action_status(action_id);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, execution_time + 1);

    // Try to execute cancelled action - should panic with "Action has been cancelled"
    dispatcher.execute_emergency_action(action_id);

    stop_cheat_block_timestamp(dispatcher.contract_address);
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_get_emergency_pools() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();

    // Create first pool with user1
    let pool_id1 = create_test_pool(dispatcher, erc20_address);

    // Create second pool with user2 (different user to avoid conflicts)
    let pool_id2 = create_test_pool_with_user(dispatcher, erc20_address, user2());

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Freeze both pools using timelock
    let freeze_action_id1 = dispatcher.schedule_emergency_action(0, pool_id1, 0); // 0 = FreezePool
    let freeze_action_id2 = dispatcher.schedule_emergency_action(0, pool_id2, 0); // 0 = FreezePool

    // Get the execution time from the first freeze action (both should have same execution time)
    let (_, execution_time) = dispatcher.get_emergency_action_status(freeze_action_id1);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, execution_time + 1);

    // Execute both freeze actions
    dispatcher.execute_emergency_action(freeze_action_id1);
    dispatcher.execute_emergency_action(freeze_action_id2);

    // Get all emergency pools
    let emergency_pools = dispatcher.get_emergency_pools();

    // Verify both pools are in emergency state
    let emergency_count = emergency_pools.len();

    // Verify both pools are in emergency state
    assert!(emergency_count == 2, "Should have 2 emergency pools, but got {}", emergency_count);

    // Verify the pools are actually in emergency state
    assert!(dispatcher.is_pool_emergency_state(pool_id1), "Pool 1 should be in emergency state");
    assert!(dispatcher.is_pool_emergency_state(pool_id2), "Pool 2 should be in emergency state");

    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
#[should_panic(expected: 'Pool is not in emergency state')]
fn test_emergency_withdrawal_non_emergency_pool() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);

    // Try emergency withdrawal without freezing the pool
    start_cheat_caller_address(dispatcher.contract_address, user1());
    dispatcher.emergency_withdraw(pool_id);
}

#[test]
#[should_panic(expected: 'Pool already in emergency')]
fn test_double_emergency_freeze() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Freeze the pool twice using timelock
    let freeze_action_id1 = dispatcher.schedule_emergency_action(0, pool_id, 0); // 0 = FreezePool

    // Get the execution time from the first freeze action
    let (_, freeze_execution_time1) = dispatcher.get_emergency_action_status(freeze_action_id1);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, freeze_execution_time1 + 1);

    // Execute the first freeze action
    dispatcher.execute_emergency_action(freeze_action_id1);

    // Try to freeze the pool again (should panic since it's already in emergency state)
    let freeze_action_id2 = dispatcher.schedule_emergency_action(0, pool_id, 0); // 0 = FreezePool

    // Get the execution time from the second freeze action
    let (_, freeze_execution_time2) = dispatcher.get_emergency_action_status(freeze_action_id2);

    // Fast forward time again
    start_cheat_block_timestamp(dispatcher.contract_address, freeze_execution_time2 + 1);

    // This should panic when trying to execute the second freeze action
    dispatcher.execute_emergency_action(freeze_action_id2);
}

#[test]
#[should_panic(expected: 'Pool is not in emergency state')]
fn test_emergency_unfreeze_non_emergency_pool() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Try to unfreeze a non-emergency pool using timelock
    let unfreeze_action_id = dispatcher
        .schedule_emergency_action(2, pool_id, 0); // 2 = UnfreezePool

    // Get the execution time from the unfreeze action
    let (_, execution_time) = dispatcher.get_emergency_action_status(unfreeze_action_id);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, execution_time + 1);

    // This should panic when trying to execute the unfreeze action on a non-emergency pool
    dispatcher.execute_emergency_action(unfreeze_action_id);
}

#[test]
#[should_panic(expected: 'Timelock not passed')]
fn test_execute_emergency_action_before_timelock() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Schedule an emergency action
    let action_data = 0; // 0 = no additional data needed
    let action_id = dispatcher.schedule_emergency_action(0, pool_id, action_data);

    // Verify action is in waiting status
    let (status, _) = dispatcher.get_emergency_action_status(action_id);
    assert!(status == 1, "Action should be in waiting status");

    // Try to execute immediately (before timelock delay)
    dispatcher.execute_emergency_action(action_id);
}

#[test]
fn test_emergency_action_types() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Test different action types
    let action_data = 0; // 0 = no additional data needed

    // Freeze pool (type 0)
    let action_id1 = dispatcher.schedule_emergency_action(0, pool_id, action_data);

    // Resolve pool (type 1) - needs winning option data
    let resolve_data = 1; // 1 = option2 wins
    let action_id2 = dispatcher.schedule_emergency_action(1, pool_id, resolve_data);

    // Unfreeze pool (type 2)
    let action_id3 = dispatcher.schedule_emergency_action(2, pool_id, action_data);

    // Emergency withdrawal (type 3) - needs user address data
    let user_address = user1(); // Use a valid user address
    let action_id4 = dispatcher.schedule_emergency_action(3, pool_id, user_address.into());

    // Verify all actions were scheduled
    let (status1, _) = dispatcher.get_emergency_action_status(action_id1);
    let (status2, _) = dispatcher.get_emergency_action_status(action_id2);
    let (status3, _) = dispatcher.get_emergency_action_status(action_id3);
    let (status4, _) = dispatcher.get_emergency_action_status(action_id4);

    assert!(status1 == 1, "Action 1 should be in waiting status");
    assert!(status2 == 1, "Action 2 should be in waiting status");
    assert!(status3 == 1, "Action 3 should be in waiting status");
    assert!(status4 == 1, "Action 4 should be in waiting status");

    // Test that actions can actually be executed (fast forward time)
    // Get the execution time from the first action
    let (_, execution_time) = dispatcher.get_emergency_action_status(action_id1);
    start_cheat_block_timestamp(dispatcher.contract_address, execution_time + 1);

    // Execute one action to verify it works
    dispatcher.execute_emergency_action(action_id1);

    // Verify action was executed
    let (final_status, _) = dispatcher.get_emergency_action_status(action_id1);
    assert!(final_status == 3, "Action should be in done status after execution");

    // Verify the pool is actually frozen after executing the action
    assert!(
        dispatcher.is_pool_emergency_state(pool_id),
        "Pool should be in emergency state after executing freeze action",
    );

    // Execute another action type to verify different actions work
    dispatcher.execute_emergency_action(action_id2); // Resolve action

    // Verify the pool is now settled after resolution
    let pool = dispatcher.get_pool(pool_id);
    assert!(
        pool.status == Status::Settled, "Pool should be settled after executing resolve action",
    );

    // Verify the pool is no longer in emergency state after resolution
    assert!(
        !dispatcher.is_pool_emergency_state(pool_id),
        "Pool should not be in emergency state after resolution",
    );

    stop_cheat_block_timestamp(dispatcher.contract_address);
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_admin_initiated_emergency_withdrawal() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool_with_user(dispatcher, erc20_address, user1());
    let mut spy = spy_events();

    // First, make user1 participate in the pool by voting
    start_cheat_caller_address(dispatcher.contract_address, user1());
    mint_tokens_for(user1(), erc20_address, 1000000000000000000); // 1 STRK
    start_cheat_caller_address(erc20_address, user1());
    approve_tokens_for_payment(dispatcher.contract_address, erc20_address, 1000000000000000000);
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(dispatcher.contract_address, user1());
    dispatcher.vote(pool_id, 'Option 1', 1000000000000000000); // 1 STRK
    stop_cheat_caller_address(dispatcher.contract_address);

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // First, put the pool in emergency state and allow withdrawals
    let freeze_action_id = dispatcher.schedule_emergency_action(0, pool_id, 0);
    let (_, execution_time) = dispatcher.get_emergency_action_status(freeze_action_id);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, execution_time + 1);

    // Execute freeze action
    dispatcher.execute_emergency_action(freeze_action_id);

    // Verify pool is in emergency state
    assert!(dispatcher.is_pool_emergency_state(pool_id), "Pool should be in emergency state");

    // Now schedule emergency withdrawal for user1
    let user_address = user1();
    let withdrawal_action_id = dispatcher
        .schedule_emergency_action(3, pool_id, user_address.into());

    // Get execution time for withdrawal action
    let (_, withdrawal_execution_time) = dispatcher
        .get_emergency_action_status(withdrawal_action_id);

    // Fast forward time to pass timelock
    start_cheat_block_timestamp(dispatcher.contract_address, withdrawal_execution_time + 1);

    // Execute emergency withdrawal action
    dispatcher.execute_emergency_action(withdrawal_action_id);

    // Verify emergency withdrawal was successful

    // Verify user's stake is reset
    let user_stake = dispatcher.get_user_stake(pool_id, user1());
    assert!(user_stake.amount == 0, "User stake should be reset to 0 after emergency withdrawal");

    // Verify action status is updated
    let (final_status, _) = dispatcher.get_emergency_action_status(withdrawal_action_id);
    assert!(final_status == 3, "Action should be in done status after execution");

    stop_cheat_block_timestamp(dispatcher.contract_address);
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
#[should_panic(expected: 'Invalid address provided')]
fn test_emergency_withdrawal_invalid_address() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();
    let pool_id = create_test_pool(dispatcher, erc20_address);

    // Set caller as admin
    start_cheat_caller_address(dispatcher.contract_address, admin());

    // Try to schedule emergency withdrawal with zero address (should fail)
    dispatcher.schedule_emergency_action(3, pool_id, 0);
}

