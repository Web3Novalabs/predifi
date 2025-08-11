use contract::base::events::Events::{
    PoolCancelled, StakeRefunded, ValidatorAdded, ValidatorRemoved,
};
use contract::base::types::{Pool, PoolDetails, Status};
use contract::interfaces::iUtils::IUtilityDispatcher;
use contract::interfaces::ipredifi::{
    IPredifiDispatcher, IPredifiDispatcherTrait, IPredifiDisputeDispatcher,
    IPredifiDisputeDispatcherTrait, IPredifiValidator, IPredifiValidatorDispatcher,
    IPredifiValidatorDispatcherTrait,
};
use contract::predifi::Predifi;
use contract::utils::Utils;
use contract::utils::Utils::InternalFunctionsTrait;
use core::array::ArrayTrait;
use core::felt252;
use core::serde::Serde;
use core::traits::{Into, TryInto};
use openzeppelin::access::accesscontrol::AccessControlComponent::InternalTrait as AccessControlInternalTrait;
use openzeppelin::access::accesscontrol::DEFAULT_ADMIN_ROLE;
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
use openzeppelin::upgrades::upgradeable::UpgradeableComponent::{Event as UpgradeEvent, Upgraded};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, EventSpyAssertionsTrait, EventSpyTrait, declare,
    get_class_hash, spy_events, start_cheat_block_timestamp, start_cheat_caller_address,
    stop_cheat_block_timestamp, stop_cheat_caller_address, test_address,
};
use starknet::storage::{MutableVecTrait, StoragePointerReadAccess, StoragePointerWriteAccess};
use starknet::{ClassHash, ContractAddress, get_block_timestamp, get_caller_address};

// Validator role
const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");
// Pool creator address constant
const POOL_CREATOR: ContractAddress = 123.try_into().unwrap();
const USER_ONE: ContractAddress = 'User1'.try_into().unwrap();
const ONE_STRK: u256 = 1_000_000_000_000_000_000;

use super::test_utils::deploy_predifi;
use super::test_utils::create_default_pool;
use super::test_utils::declare_contract;


#[test]
fn test_create_pool() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "Pool not created successfully");
}

#[test]
fn test_cancel_pool() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "Pool not created successfully");

    contract.cancel_pool(pool_id);

    let fetched_pool = contract.get_pool(pool_id);
    assert(fetched_pool.status == Status::Closed, 'Pool not closed');
}

#[test]
fn test_zero_min_bet() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
        creatorFee,
        isPrivate,
        category,
    ) = (
        'Example Pool',
        0,
        "A simple betting pool",
        "image.png",
        "event.com/details",
        1710000000,
        1710003600,
        1710007200,
        'Team A',
        'Team B',
        0, // Zero min bet
        10000,
        5,
        false,
        0,
    );

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = contract
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
            creatorFee,
            isPrivate,
            category,
        );

    let pool = contract.get_pool(pool_id);
    assert(pool.minBetAmount == 0, 'Min bet should be 0');
}

#[test]
fn test_valid_pool_types() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);

    // Test WinBet (0)
    let pool_id_1 = contract
        .create_pool(
            'WinBet Pool',
            0,
            "Win bet pool",
            "image.png",
            "event.com/details",
            1710000000,
            1710003600,
            1710007200,
            'Team A',
            'Team B',
            100,
            10000,
            5,
            false,
            0,
        );

    let pool_1 = contract.get_pool(pool_id_1);
    assert(pool_1.poolType == Pool::WinBet, 'Should be WinBet');

    // Test VoteBet (1)
    let pool_id_2 = contract
        .create_pool(
            'VoteBet Pool',
            1,
            "Vote bet pool",
            "image.png",
            "event.com/details",
            1710000000,
            1710003600,
            1710007200,
            'Option A',
            'Option B',
            100,
            10000,
            5,
            false,
            0,
        );

    let pool_2 = contract.get_pool(pool_id_2);
    assert(pool_2.poolType == Pool::VoteBet, 'Should be VoteBet');

    // Test OverUnderBet (2)
    let pool_id_3 = contract
        .create_pool(
            'OverUnder Pool',
            2,
            "Over/Under bet pool",
            "image.png",
            "event.com/details",
            1710000000,
            1710003600,
            1710007200,
            'Over',
            'Under',
            100,
            10000,
            5,
            false,
            0,
        );

    let pool_3 = contract.get_pool(pool_id_3);
    assert(pool_3.poolType == Pool::OverUnderBet, 'Should be OverUnderBet');

    // Test ParlayPool (3)
    let pool_id_4 = contract
        .create_pool(
            'Parlay Pool',
            3,
            "Parlay pool",
            "image.png",
            "event.com/details",
            1710000000,
            1710003600,
            1710007200,
            'Parlay A',
            'Parlay B',
            100,
            10000,
            5,
            false,
            0,
        );

    let pool_4 = contract.get_pool(pool_id_4);
    assert(pool_4.poolType == Pool::ParlayPool, 'Should be ParlayPool');
}

#[test]
fn test_get_pool_odds() {
    let (contract, _, _, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);

    // Get initial odds (should be equal)
    let initial_odds = contract.pool_odds(pool_id);
    assert(initial_odds.option1_odds == 10000, 'Initial odds should be 1.0');
    assert(initial_odds.option2_odds == 10000, 'Initial odds should be 1.0');

    // Place a bet on option1 to change odds
    contract.vote(pool_id, 'Team A', 1000);

    // Get updated odds
    let updated_odds = contract.pool_odds(pool_id);
    // After betting on option1, its odds should decrease (become less favorable)
    assert(updated_odds.option1_odds < 10000, 'Option1 odds should decrease');
    assert(updated_odds.option2_odds > 10000, 'Option2 odds should increase');
}

#[test]
fn test_automatic_pool_state_transitions() {
    let (contract, _, _, user, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, user);
    let pool_id = create_default_pool(contract);
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Active, 'Pool should be active');

    // Advance time to lock time
    start_cheat_block_timestamp(contract.contract_address, 1710003600);
    contract.manually_update_pool_state(pool_id, 1);
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Locked, 'Pool should be locked');

    // Advance time to end time
    start_cheat_block_timestamp(contract.contract_address, 1710007200);
    contract.manually_update_pool_state(pool_id, 2);
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Settled, 'Pool should be settled');

    stop_cheat_block_timestamp(contract.contract_address);
}

fn create_test_pool(
    contract: IPredifiDispatcher,
    pool_name: felt252,
    start_time: u64,
    lock_time: u64,
    end_time: u64,
) -> u256 {
    contract
        .create_pool(
            pool_name,
            0, // 0 = WinBet
            "Test pool",
            "image.png",
            "event.com/details",
            start_time,
            lock_time,
            end_time,
            'Option A',
            'Option B',
            100,
            10000,
            5,
            false,
            0,
        )
}

#[test]
fn test_get_active_pools() {
    // Deploy the contract
    let (dispatcher, _, _, pool_creator, erc20_address) = deploy_predifi();

    let erc20_dispatcher: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20_dispatcher.approve(dispatcher.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

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

    let erc20_dispatcher: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20_dispatcher.approve(dispatcher.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);

    // Impersonate pool_creator for pool creation
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher, 'Locked Pool 1', initial_time + 100, initial_time + 200, initial_time + 3000,
    );
    stop_cheat_caller_address(dispatcher.contract_address);

    // Advance time to lock time
    stop_cheat_block_timestamp(dispatcher.contract_address);
    let locked_time = initial_time + 250;
    start_cheat_block_timestamp(dispatcher.contract_address, locked_time);

    // Update pool state to locked
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(dispatcher.contract_address, admin);
    dispatcher.manually_update_pool_state(pool1_id, 1);
    stop_cheat_caller_address(dispatcher.contract_address);

    // Get locked pools
    let locked_pools = dispatcher.get_locked_pools();

    // Verify we have 1 locked pool
    assert(locked_pools.len() == 1, 'Expected 1 locked pool');

    // Clean up
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_get_settled_pools() {
    let (dispatcher, _, _, pool_creator, erc20_address) = deploy_predifi();

    let erc20_dispatcher: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20_dispatcher.approve(dispatcher.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);

    // Impersonate pool_creator for pool creation
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher, 'Settled Pool 1', initial_time + 100, initial_time + 200, initial_time + 300,
    );
    stop_cheat_caller_address(dispatcher.contract_address);

    // Advance time to end time
    stop_cheat_block_timestamp(dispatcher.contract_address);
    let settled_time = initial_time + 350;
    start_cheat_block_timestamp(dispatcher.contract_address, settled_time);

    // Update pool state to settled
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(dispatcher.contract_address, admin);
    dispatcher.manually_update_pool_state(pool1_id, 2);
    stop_cheat_caller_address(dispatcher.contract_address);

    // Get settled pools
    let settled_pools = dispatcher.get_settled_pools();

    // Verify we have 1 settled pool
    assert(settled_pools.len() == 1, 'Expected 1 settled pool');

    // Clean up
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_get_closed_pools() {
    let (dispatcher, _, _, pool_creator, erc20_address) = deploy_predifi();

    let erc20_dispatcher: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20_dispatcher.approve(dispatcher.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);

    // Impersonate pool_creator for pool creation
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher, 'Pool to Cancel', initial_time + 100, initial_time + 200, initial_time + 300,
    );

    // Cancel the pool to make it closed
    dispatcher.cancel_pool(pool1_id);
    stop_cheat_caller_address(dispatcher.contract_address);

    // Get closed pools
    let closed_pools = dispatcher.get_closed_pools();

    // Verify we have 1 closed pool
    assert(closed_pools.len() == 1, 'Expected 1 closed pool');

    // Clean up
    stop_cheat_block_timestamp(dispatcher.contract_address);
}
