use contract::base::events::Events::PoolCancelled;
use contract::base::types::{Pool, Status};
use contract::interfaces::ipredifi::IPredifiDispatcherTrait;
use contract::predifi::Predifi;
use core::array::ArrayTrait;
use core::felt252;
use core::serde::Serde;
use core::traits::TryInto;
use snforge_std::{
    EventSpyAssertionsTrait, spy_events, start_cheat_block_timestamp, start_cheat_caller_address,
    stop_cheat_block_timestamp, stop_cheat_caller_address,
};
use starknet::{ContractAddress, get_block_timestamp};

// Validator role
const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");
// Pool creator address constant
const POOL_CREATOR: ContractAddress = 123.try_into().unwrap();
const USER_ONE: ContractAddress = 'User1'.try_into().unwrap();

const ONE_STRK: u256 = 1_000_000_000_000_000_000;
use super::test_utils::{
    approve_tokens_for_payment, create_default_pool, create_test_pool, deploy_predifi,
    get_default_pool_params,
};


#[test]
fn test_create_pool() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");
}

#[test]
fn test_cancel_pool() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");
    contract.cancel_pool(pool_id);
    let fetched_pool = contract.get_pool(pool_id);
    assert(fetched_pool.status == Status::Closed, 'Pool not closed');
}

#[test]
fn test_cancel_pool_event_emission() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();
    let mut spy = spy_events();

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");

    contract.cancel_pool(pool_id);

    let fetched_pool = contract.get_pool(pool_id);

    assert(fetched_pool.status == Status::Closed, 'Pool not closed');

    let expected_event = Predifi::Event::PoolCancelled(
        PoolCancelled { pool_id, timestamp: get_block_timestamp() },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_event)]);
}

#[test]
#[should_panic(expected: 'Unauthorized Caller')]
fn test_cancel_pool_by_unauthorized_caller() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");

    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.cancel_pool(pool_id);
}

#[test]
#[should_panic(expected: 'Invalid lock time')]
fn test_invalid_time_sequence_start_after_lock() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    let (
        poolName,
        poolType,
        poolDescription,
        poolImage,
        poolEventSourceUrl,
        _,
        _,
        poolEndTime,
        option1,
        option2,
        minBetAmount,
        maxBetAmount,
        creatorFee,
        isPrivate,
        category,
    ) =
        get_default_pool_params();

    let current_time = get_block_timestamp();
    let invalid_start_time = current_time + 3600;
    let invalid_lock_time = current_time + 1800;

    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract
        .create_pool(
            poolName,
            poolType,
            poolDescription,
            poolImage,
            poolEventSourceUrl,
            invalid_start_time,
            invalid_lock_time,
            poolEndTime,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );
}

#[test]
#[should_panic(expected: 'Minimum bet cannot be zero')]
fn test_zero_min_bet() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    let (
        poolName,
        poolType,
        poolDescription,
        poolImage,
        poolEventSourceUrl,
        poolStartTime,
        poolLockTime,
        poolEndTime,
        option1,
        option2,
        _,
        maxBetAmount,
        creatorFee,
        isPrivate,
        category,
    ) =
        get_default_pool_params();

    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract
        .create_pool(
            poolName,
            poolType,
            poolDescription,
            poolImage,
            poolEventSourceUrl,
            poolStartTime,
            poolLockTime,
            poolEndTime,
            option1,
            option2,
            0,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );
}

#[test]
#[should_panic(expected: 'Creator fee cannot exceed 5%')]
fn test_excessive_creator_fee() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    let (
        poolName,
        poolType,
        poolDescription,
        poolImage,
        poolEventSourceUrl,
        poolStartTime,
        poolLockTime,
        poolEndTime,
        option1,
        option2,
        minBetAmount,
        maxBetAmount,
        _,
        isPrivate,
        category,
    ) =
        get_default_pool_params();

    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract
        .create_pool(
            poolName,
            poolType,
            poolDescription,
            poolImage,
            poolEventSourceUrl,
            poolStartTime,
            poolLockTime,
            poolEndTime,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            6,
            isPrivate,
            category,
        );
}

#[test]
#[should_panic(expected: "Invalid pool type: must be 0-3")]
fn test_invalid_pool_type() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    let (
        poolName,
        _,
        poolDescription,
        poolImage,
        poolEventSourceUrl,
        poolStartTime,
        poolLockTime,
        poolEndTime,
        option1,
        option2,
        minBetAmount,
        maxBetAmount,
        creatorFee,
        isPrivate,
        category,
    ) =
        get_default_pool_params();

    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract
        .create_pool(
            poolName,
            99, // Invalid pool type
            poolDescription,
            poolImage,
            poolEventSourceUrl,
            poolStartTime,
            poolLockTime,
            poolEndTime,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );
}

#[test]
fn test_valid_pool_types() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    let (
        _,
        _,
        poolDescription,
        poolImage,
        poolEventSourceUrl,
        poolStartTime,
        poolLockTime,
        poolEndTime,
        option1,
        option2,
        minBetAmount,
        maxBetAmount,
        creatorFee,
        isPrivate,
        category,
    ) =
        get_default_pool_params();

    start_cheat_caller_address(contract.contract_address, pool_creator);

    // Test all valid pool types (0-3)
    let pool_id_0 = contract
        .create_pool(
            'WinBet Pool',
            0, // WinBet
            poolDescription.clone(),
            poolImage.clone(),
            poolEventSourceUrl.clone(),
            poolStartTime,
            poolLockTime,
            poolEndTime,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );

    let pool_id_1 = contract
        .create_pool(
            'VoteBet Pool',
            1, // VoteBet
            poolDescription.clone(),
            poolImage.clone(),
            poolEventSourceUrl.clone(),
            poolStartTime + 1,
            poolLockTime + 1,
            poolEndTime + 1,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );

    let pool_id_2 = contract
        .create_pool(
            'OverUnderBet Pool',
            2, // OverUnderBet
            poolDescription.clone(),
            poolImage.clone(),
            poolEventSourceUrl.clone(),
            poolStartTime + 2,
            poolLockTime + 2,
            poolEndTime + 2,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );

    let pool_id_3 = contract
        .create_pool(
            'ParlayPool Pool',
            3, // ParlayPool
            poolDescription,
            poolImage,
            poolEventSourceUrl,
            poolStartTime + 3,
            poolLockTime + 3,
            poolEndTime + 3,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );

    // Verify all pools were created successfully
    assert!(pool_id_0 != 0, "WinBet pool not created");
    assert!(pool_id_1 != 0, "VoteBet pool not created");
    assert!(pool_id_2 != 0, "OverUnderBet pool not created");
    assert!(pool_id_3 != 0, "ParlayPool pool not created");

    // Verify the pool types are correctly stored
    let pool_0 = contract.get_pool(pool_id_0);
    let pool_1 = contract.get_pool(pool_id_1);
    let pool_2 = contract.get_pool(pool_id_2);
    let pool_3 = contract.get_pool(pool_id_3);

    assert(pool_0.poolType == Pool::WinBet, 'Wrong type for pool 0');
    assert(pool_1.poolType == Pool::VoteBet, 'Wrong type for pool 1');
    assert(pool_2.poolType == Pool::OverUnderBet, 'Wrong type for pool 2');
    assert(pool_3.poolType == Pool::ParlayPool, 'Wrong type for pool 3');
}


#[test]
fn test_minimal_timing() {
    let (dispatcher, _, _, pool_creator, erc20_address) = deploy_predifi();

    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        dispatcher.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    let t0 = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, t0);

    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    create_test_pool(
        dispatcher, 'Test Pool', t0 + 1000, // 2000
        t0 + 2000, // 3000
        t0 + 3000 // 4000
    );
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_get_active_pools() {
    // Deploy the contract
    let (dispatcher, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Approve the dispatcher contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        dispatcher.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Set initial block timestamp
    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);

    // Impersonate pool_creator for pool creation
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher, 'Active Pool 1', initial_time + 1600, initial_time + 2000, initial_time + 3000,
    );
    stop_cheat_caller_address(dispatcher.contract_address);

    let time_2 = initial_time + 1000;
    stop_cheat_block_timestamp(dispatcher.contract_address);
    start_cheat_block_timestamp(dispatcher.contract_address, time_2);

    // Impersonate pool_creator for pool creation
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool2_id = create_test_pool(
        dispatcher, 'Active Pool 2', time_2 + 500, time_2 + 1500, time_2 + 3500,
    );
    stop_cheat_caller_address(dispatcher.contract_address);

    // Advance time to 3500 (after both pools' start, before both pools' lock)
    stop_cheat_block_timestamp(dispatcher.contract_address);
    let active_time = 1500;
    start_cheat_block_timestamp(dispatcher.contract_address, active_time);

    // Update pool states before checking
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(dispatcher.contract_address, admin);
    dispatcher.manually_update_pool_state(pool1_id, 0);
    dispatcher.manually_update_pool_state(pool2_id, 0);
    stop_cheat_caller_address(dispatcher.contract_address);

    // Get active pools
    let active_pools = dispatcher.get_active_pools();

    // Debug: check pool statuses
    let pool1 = dispatcher.get_pool(pool1_id);
    let pool2 = dispatcher.get_pool(pool2_id);

    assert(pool1.status == Status::Active, 'Pool 1 should be active');
    assert(pool2.status == Status::Active, 'Pool 2 should be active');

    // Verify we have 2 active pools
    assert(active_pools.len() == 2, 'Expected 2 active pools');

    // Clean up
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_get_locked_pools() {
    let (dispatcher, _, _, pool_creator, erc20_address) = deploy_predifi();

    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        dispatcher.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);
    // Pool 1
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher,
        'Locked Pool 1',
        initial_time + 1000, // start: 2000
        initial_time + 2000, // lock: 3000
        initial_time + 3000 // end: 4000
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Set block timestamp for Pool 2 creation
    let time_2 = initial_time + 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, time_2);

    // Pool 2 (start time strictly greater than block timestamp)
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool2_id = create_test_pool(
        dispatcher,
        'Locked Pool 2',
        time_2 + 1, // start: 2001
        time_2 + 1001, // lock: 3001
        time_2 + 2001 // end: 4001
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);
    // Advance time to just after both locks but before both ends
    let locked_time = time_2 + 1200; // 2200 > 2001 (lock), < 4001 (end)
    start_cheat_block_timestamp(dispatcher.contract_address, locked_time);

    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(dispatcher.contract_address, admin);
    dispatcher.manually_update_pool_state(pool1_id, 1);
    dispatcher.manually_update_pool_state(pool2_id, 1);
    stop_cheat_caller_address(dispatcher.contract_address);

    let locked_pools = dispatcher.get_locked_pools();
    let pool1 = dispatcher.get_pool(pool1_id);
    let pool2 = dispatcher.get_pool(pool2_id);

    assert(pool1.status == Status::Locked, 'Pool 1 should be locked');
    assert(pool2.status == Status::Locked, 'Pool 2 should be locked');
    assert(locked_pools.len() == 2, 'Expected 2 locked pools');
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_get_settled_pools() {
    let (dispatcher, _, _, pool_creator, erc20_address) = deploy_predifi();
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        dispatcher.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);
    // Pool 1
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher,
        'Settled Pool 1',
        initial_time + 1000, // start: 2000
        initial_time + 2000, // lock: 3000
        initial_time + 3000 // end: 4000
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Set block timestamp for Pool 2 creation
    let time_2 = initial_time + 1500; // 2500
    start_cheat_block_timestamp(dispatcher.contract_address, time_2);

    // Pool 2 (start time strictly greater than block timestamp)
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool2_id = create_test_pool(
        dispatcher,
        'Settled Pool 2',
        time_2 + 100, // start: 2600
        time_2 + 1100, // lock: 3600
        time_2 + 2100 // end: 4600
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);
    // Advance time to after both ends
    let settled_time = initial_time + 5000; // 5000 > 4000 and 4600
    start_cheat_block_timestamp(dispatcher.contract_address, settled_time);
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(dispatcher.contract_address, admin);
    dispatcher.manually_update_pool_state(pool1_id, 2);
    dispatcher.manually_update_pool_state(pool2_id, 2);
    stop_cheat_caller_address(dispatcher.contract_address);

    let settled_pools = dispatcher.get_settled_pools();
    let pool1 = dispatcher.get_pool(pool1_id);
    let pool2 = dispatcher.get_pool(pool2_id);
    assert(pool1.status == Status::Settled, 'Pool 1 should be settled');
    assert(pool2.status == Status::Settled, 'Pool 2 should be settled');
    assert(settled_pools.len() == 2, 'Expected 2 settled pools');
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_get_closed_pools() {
    let (dispatcher, _, _, pool_creator, erc20_address) = deploy_predifi();
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        dispatcher.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);
    // Pool 1
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher,
        'Closed Pool 1',
        initial_time + 1000, // start: 2000
        initial_time + 2000, // lock: 3000
        initial_time + 3000 // end: 4000
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Set block timestamp for Pool 2 creation
    let time_2 = initial_time + 1500; // 2500
    start_cheat_block_timestamp(dispatcher.contract_address, time_2);

    // Pool 2 (start time strictly greater than block timestamp)
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool2_id = create_test_pool(
        dispatcher,
        'Closed Pool 2',
        time_2 + 100, // start: 2600
        time_2 + 1100, // lock: 3600
        time_2 + 2100 // end: 4600
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Assume pool1_id and pool2_id are created, and you have their end times
    let end_time_1 = 4000; // set to pool 1's end time
    let end_time_2 = 4600; // set to pool 2's end time
    let after_end = core::cmp::max(end_time_1, end_time_2) + 1;
    start_cheat_block_timestamp(dispatcher.contract_address, after_end);
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(dispatcher.contract_address, admin);
    dispatcher.manually_update_pool_state(pool1_id, 1);
    dispatcher.manually_update_pool_state(pool2_id, 1);
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Now advance to after end_time + 86401 for the latest pool
    let after_closed = core::cmp::max(end_time_1, end_time_2) + 86401;
    start_cheat_block_timestamp(dispatcher.contract_address, after_closed);
    start_cheat_caller_address(dispatcher.contract_address, admin);
    dispatcher.manually_update_pool_state(pool1_id, 3);
    dispatcher.manually_update_pool_state(pool2_id, 3);
    stop_cheat_caller_address(dispatcher.contract_address);

    let closed_pools = dispatcher.get_closed_pools();
    let pool1 = dispatcher.get_pool(pool1_id);
    let pool2 = dispatcher.get_pool(pool2_id);
    println!("Pool 1 status: {:?}", pool1.status);
    println!("Pool 2 status: {:?}", pool2.status);
    println!("closed_pools.len(): {:?}", closed_pools.len());
    assert(pool1.status == Status::Closed, 'Pool 1 should be closed');
    assert(pool2.status == Status::Closed, 'Pool 2 should be closed');
    assert(closed_pools.len() == 2, 'Expected 2 closed pools');
}

#[test]
fn test_automatic_pool_state_transitions() {
    let (contract, _, _, admin, erc20_address) = deploy_predifi();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval
    start_cheat_caller_address(erc20_address, admin);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, admin);
    // Create a pool with specific timestamps
    let active_pool_id = contract
        .create_pool(
            'Active Pool',
            0, // 0 = WinBet
            "Pool in active state",
            "image.png",
            "event.com/details",
            current_time + 1000, // start time in future
            current_time + 2000, // lock time in future
            current_time + 3000, // end time in future
            'Option A',
            'Option B',
            100,
            10000,
            5,
            false,
            0,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Verify initial state
    let pool = contract.get_pool(active_pool_id);
    assert(pool.status == Status::Active, 'Initial state should be Active');

    // Test no change when time hasn't reached lock time
    start_cheat_block_timestamp(contract.contract_address, current_time + 1500);
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    let same_state = contract.manually_update_pool_state(active_pool_id, 0);
    stop_cheat_caller_address(contract.contract_address);
    assert(same_state == Status::Active, 'State should remain Active');

    // Check pool state is still Active
    let pool_after_check = contract.get_pool(active_pool_id);
    assert(pool_after_check.status == Status::Active, 'Status should not change');

    // Test transition: Active -> Locked
    // Set block timestamp to just after lock time
    start_cheat_block_timestamp(contract.contract_address, current_time + 2001);
    start_cheat_caller_address(contract.contract_address, admin);
    let new_state = contract.manually_update_pool_state(active_pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    assert(new_state == Status::Locked, 'State should be Locked');

    // Verify state was actually updated in storage
    let locked_pool = contract.get_pool(active_pool_id);
    assert(locked_pool.status == Status::Locked, 'should be Locked in storage');

    // Try updating again - should stay in Locked state
    start_cheat_caller_address(contract.contract_address, admin);
    let same_locked_state = contract.manually_update_pool_state(active_pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    assert(same_locked_state == Status::Locked, 'Should remain Locked');

    // Test transition: Locked -> Settled
    // Set block timestamp to just after end time
    start_cheat_block_timestamp(contract.contract_address, current_time + 3001);
    start_cheat_caller_address(contract.contract_address, admin);
    let new_state = contract.manually_update_pool_state(active_pool_id, 2);
    stop_cheat_caller_address(contract.contract_address);
    assert(new_state == Status::Settled, 'State should be Settled');

    // Verify state was updated in storage
    let settled_pool = contract.get_pool(active_pool_id);
    assert(settled_pool.status == Status::Settled, 'should be Settled in storage');

    // Test transition: Settled -> Closed
    // Set block timestamp to 24 hours + 1 second after end time
    start_cheat_block_timestamp(contract.contract_address, current_time + 3000 + 86401);
    start_cheat_caller_address(contract.contract_address, admin);
    let final_state = contract.manually_update_pool_state(active_pool_id, 3);
    stop_cheat_caller_address(contract.contract_address);
    assert(final_state == Status::Closed, 'State should be Closed');

    // Verify state was updated in storage
    let closed_pool = contract.get_pool(active_pool_id);
    assert(closed_pool.status == Status::Closed, 'should be Closed in storage');

    // Test that no further transitions occur once Closed
    // Set block timestamp to much later
    start_cheat_block_timestamp(contract.contract_address, current_time + 10000);
    start_cheat_caller_address(contract.contract_address, admin);
    let final_state = contract.manually_update_pool_state(active_pool_id, 3);
    stop_cheat_caller_address(contract.contract_address);
    assert(final_state == Status::Closed, 'Should remain Closed');

    // Reset block timestamp cheat
    stop_cheat_block_timestamp(contract.contract_address);
}

#[test]
#[should_panic(expected: 'Pool does not exist')]
fn test_nonexistent_pool_state_update() {
    let (contract, _, _, _, _) = deploy_predifi();

    // Attempt to update a pool that doesn't exist - should panic
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(999, 3);
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_manual_pool_state_update() {
    let (contract, _, _, user, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Get current time
    let current_time = get_block_timestamp();
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, user);
    // Create a pool with specific timestamps
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A pool for testing manual updates",
            "image.png",
            "event.com/details",
            current_time + 1000,
            current_time + 2000,
            current_time + 3000,
            'Option A',
            'Option B',
            100,
            10000,
            5,
            false,
            0,
        );

    // Verify initial state
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Active, 'Initial state should be Active');

    // Manually update to Locked state
    start_cheat_caller_address(contract.contract_address, admin);
    let locked_state = contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);

    assert(locked_state == Status::Locked, 'State should be Locked');

    // Verify state change in storage
    let locked_pool = contract.get_pool(pool_id);
    assert(locked_pool.status == Status::Locked, 'should be Locked in storage');

    // Update to Settled state
    start_cheat_caller_address(contract.contract_address, admin);
    let settled_state = contract.manually_update_pool_state(pool_id, 2);
    stop_cheat_caller_address(contract.contract_address);

    assert(settled_state == Status::Settled, 'State should be Settled');

    // Verify state change in storage
    let settled_pool = contract.get_pool(pool_id);
    assert(settled_pool.status == Status::Settled, 'should be Settled in storage');

    // Update to Closed state
    start_cheat_caller_address(contract.contract_address, admin);
    let closed_state = contract.manually_update_pool_state(pool_id, 3);
    stop_cheat_caller_address(contract.contract_address);

    assert(closed_state == Status::Closed, 'State should be Closed');

    // Verify final state in storage
    let final_pool = contract.get_pool(pool_id);
    assert(final_pool.status == Status::Closed, 'should be Closed in storage');
}
