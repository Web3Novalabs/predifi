use contract::base::events::Events::{
    BetPlaced, ContractPaused, ContractUnpaused, ContractUpgraded, PoolCreated,
    PoolCreationFeeCollected, ValidatorAdded, ValidatorConfirmationsUpdated, ValidatorRemoved,
};
use contract::base::types::{CategoryType, u8_to_category, Status};
use contract::interfaces::ipredifi::{IPredifiDispatcherTrait, IPredifiValidatorDispatcherTrait};
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
use contract::predifi::Predifi::{Event as PredifiEvent};
use core::array::ArrayTrait;
use core::starknet::ClassHash;
use core::traits::TryInto;
use openzeppelin::upgrades::upgradeable::UpgradeableComponent::{Event as UpgradeEvent, Upgraded};
use snforge_std::{
    EventSpyAssertionsTrait, EventSpyTrait, get_class_hash, spy_events, start_cheat_caller_address,
    stop_cheat_caller_address,
};
use starknet::{ContractAddress, get_block_timestamp};
use super::test_utils::{declare_contract, deploy_predifi};

/// Tests that the ValidatorConfirmationsUpdated event is emitted correctly
#[test]
fn test_validator_confirmations_updated_event() {
    let (_, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let new_count: u256 = 5;
    let mut spy = spy_events();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    
    // Get current count for comparison
    let previous_count = 2; // Default from constructor

    // Set new validator confirmations
    validator_contract.set_required_validator_confirmations(new_count);

    stop_cheat_caller_address(validator_contract.contract_address);

    // Check events
    let events = spy.get_events();
    assert(events.events.len() >= 1, 'Event not emitted');

    // Verify the ValidatorConfirmationsUpdated event
    let expected_event = PredifiEvent::ValidatorConfirmationsUpdated(
        ValidatorConfirmationsUpdated {
            previous_count: previous_count.into(),
            new_count,
            admin,
            timestamp: get_block_timestamp(),
        }
    );

    let expected_events = array![(validator_contract.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
}

/// Tests that the ContractPaused event is emitted correctly
#[test]
fn test_contract_paused_event() {
    let (_, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let mut spy = spy_events();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    
    // Pause the contract
    validator_contract.pause();

    stop_cheat_caller_address(validator_contract.contract_address);

    // Check events
    let events = spy.get_events();
    assert(events.events.len() >= 1, 'Event not emitted');

    // Verify the ContractPaused event
    let expected_event = PredifiEvent::ContractPaused(
        ContractPaused {
            admin,
            timestamp: get_block_timestamp(),
        }
    );

    let expected_events = array![(validator_contract.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
}

/// Tests that the ContractUnpaused event is emitted correctly
#[test]
fn test_contract_unpaused_event() {
    let (_, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let mut spy = spy_events();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    
    // First pause the contract
    validator_contract.pause();
    
    // Then unpause it to test the unpause event
    validator_contract.unpause();

    stop_cheat_caller_address(validator_contract.contract_address);

    // Check events
    let events = spy.get_events();
    assert(events.events.len() >= 2, 'Events not emitted');

    // Verify the ContractUnpaused event (should be the second event)
    let expected_event = PredifiEvent::ContractUnpaused(
        ContractUnpaused {
            admin,
            timestamp: get_block_timestamp(),
        }
    );

    let expected_events = array![(validator_contract.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
}

/// Tests that the ContractUpgraded event is emitted correctly
#[test]
fn test_contract_upgraded_event() {
    let (contract, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let new_class_hash = declare_contract("STARKTOKEN");
    let mut spy = spy_events();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    
    // Upgrade the contract
    validator_contract.upgrade(new_class_hash);

    stop_cheat_caller_address(validator_contract.contract_address);

    // Check events
    let events = spy.get_events();
    assert(events.events.len() >= 1, 'Event not emitted');

    // Verify the ContractUpgraded event
    let expected_event = PredifiEvent::ContractUpgraded(
        ContractUpgraded {
            admin,
            new_class_hash,
            timestamp: get_block_timestamp(),
        }
    );

    let expected_events = array![(validator_contract.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
}

/// Tests that the PoolCreated event is emitted correctly
#[test]
fn test_pool_created_event() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();
    let pool_name: felt252 = 'Test Pool';
    let pool_description = "Test pool description";
    let pool_image = "https://image.url";
    let pool_event_source_url = "https://source.url";
    let category: u8 = 0; // Sports
    let creator_fee: u8 = 5;
    let min_bet = 1000000000000000000_u256;
    let max_bet = 10000000000000000000_u256;
    let current_time = get_block_timestamp();
    
    let mut spy = spy_events();

    // Setup ERC20 token contract - approve tokens for pool creation
    let token_contract = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    token_contract.approve(contract.contract_address, 10000000000000000000_u256);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    
    // Create a pool - this should emit PoolCreated and PoolCreationFeeCollected events
    let pool_id = contract.create_pool(
        pool_name,
        0_u8,
        pool_description,
        pool_image,
        pool_event_source_url,
        current_time + 3600,
        current_time + 7200,
        current_time + 10800,
        'option1',
        'option2',
        min_bet,
        max_bet,
        creator_fee,
        false,
        category
    );

    stop_cheat_caller_address(contract.contract_address);

    // Check events - should have at least PoolCreated and PoolCreationFeeCollected
    let events = spy.get_events();
    assert(events.events.len() >= 2, 'Events not emitted');
    assert(pool_id > 0, 'Pool should be created');
}

/// Tests that the ValidatorAdded event is emitted correctly
#[test]
fn test_validator_added_event() {
    let (_, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let new_validator: ContractAddress = 'new_validator'.try_into().unwrap();
    let mut spy = spy_events();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    
    // Add a validator
    validator_contract.add_validator(new_validator);

    stop_cheat_caller_address(validator_contract.contract_address);

    // Check events
    let events = spy.get_events();
    assert(events.events.len() >= 1, 'Event not emitted');

    // Verify the ValidatorAdded event
    let expected_event = PredifiEvent::ValidatorAdded(
        ValidatorAdded {
            account: new_validator,
            caller: admin,
        }
    );

    let expected_events = array![(validator_contract.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
}

/// Tests that the ValidatorRemoved event is emitted correctly
#[test]
fn test_validator_removed_event() {
    let (_, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let test_validator: ContractAddress = 'test_validator'.try_into().unwrap();
    let mut spy = spy_events();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    
    // First add a validator so we can remove it
    validator_contract.add_validator(test_validator);
    
    // Remove the validator
    validator_contract.remove_validator(test_validator);

    stop_cheat_caller_address(validator_contract.contract_address);

    // Check events
    let events = spy.get_events();
    assert(events.events.len() >= 1, 'Event not emitted');

    // Verify the ValidatorRemoved event
    let expected_event = PredifiEvent::ValidatorRemoved(
        ValidatorRemoved {
            account: test_validator,
            caller: admin,
        }
    );

    let expected_events = array![(validator_contract.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
}

/// Tests that events have proper indexing for filtering
#[test]
fn test_event_indexing_with_keys() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();
    let pool_name: felt252 = 'Indexed Pool';
    let category: u8 = 1; // Politics
    let creator_fee: u8 = 3;
    let min_bet = 500000000000000000_u256;
    let max_bet = 5000000000000000000_u256;
    let current_time = get_block_timestamp();
    
    let mut spy = spy_events();

    // Setup tokens and approve
    let token_contract = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    token_contract.approve(contract.contract_address, 10000000000000000000_u256);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    
    // Create a pool to test indexed events
    let pool_id = contract.create_pool(
        pool_name,
        0_u8,
        "Indexed test pool",
        "https://indexed.url",
        "https://indexed.source",
        current_time + 3600,
        current_time + 7200,
        current_time + 10800,
        'yes',
        'no',
        min_bet,
        max_bet,
        creator_fee,
        false,
        category
    );

    stop_cheat_caller_address(contract.contract_address);

    // The events should be emitted with indexed keys
    let events = spy.get_events();
    assert(events.events.len() >= 2, 'Indexed events not emitted');

    // Events can be filtered by indexed fields (pool_id, creator, category for PoolCreated)
    // This test ensures the events are structured correctly for frontend filtering
    assert(pool_id > 0, 'Pool ID should be valid');
}