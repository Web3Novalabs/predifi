use contract::interfaces::ipredifi::{
    IPredifiDispatcher, IPredifiDispatcherTrait, IPredifiValidatorDispatcher,
    IPredifiValidatorDispatcherTrait
};
use core::array::ArrayTrait;
use core::serde::Serde;
use core::traits::TryInto;
use openzeppelin::upgrades::upgradeable::UpgradeableComponent::{Event as UpgradeEvent, Upgraded};
use snforge_std::{
    EventSpyAssertionsTrait, EventSpyTrait, get_class_hash, spy_events, start_cheat_caller_address,
    stop_cheat_caller_address,
};
use starknet::ContractAddress;
use super::test_utils::{declare_contract, deploy_predifi};
use contract::base::events::Event::FeeUpdated;
use contract::base::events::FeesCollected;


#[test]
fn test_upgrade_by_admin() {
    let (contract, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let new_class_hash = declare_contract("STARKTOKEN");
    let mut spy = spy_events();

    // Set caller address to admin
    start_cheat_caller_address(validator_contract.contract_address, admin);

    // Call the upgrade function as the admin
    validator_contract.upgrade(new_class_hash);

    stop_cheat_caller_address(validator_contract.contract_address);

    // Verify the upgrade was successful by checking the class hash
    let current_class_hash = get_class_hash(contract.contract_address);
    assert(current_class_hash == new_class_hash, 'Contract upgrade failed');

    // Get emitted events
    let events = spy.get_events();
    assert(events.events.len() == 1, 'Upgrade event not emitted');
    // Verify upgrade event
    let expected_upgrade_event = UpgradeEvent::Upgraded(Upgraded { class_hash: new_class_hash });

    // Assert that the event was emitted
    let expected_events = array![(contract.contract_address, expected_upgrade_event)];
    spy.assert_emitted(@expected_events);
}

#[test]
#[should_panic(expected: 'Caller is missing role')]
fn test_upgrade_by_non_admin_should_panic() {
    let (_, _, validator_contract, pool_creator, _) = deploy_predifi();
    let new_class_hash = declare_contract("STARKTOKEN");

    // Set caller address to non-owner
    start_cheat_caller_address(validator_contract.contract_address, pool_creator);

    // Attempt to call the upgrade function as a non-owner
    validator_contract.upgrade(new_class_hash);
}

#[test]
#[should_panic(expected: 'Pausable: paused')]
fn test_upgrade_fails_when_paused() {
    let (_, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let new_class_hash = declare_contract("STARKTOKEN");

    start_cheat_caller_address(validator_contract.contract_address, admin);
    // Pause the contract
    validator_contract.pause();
    validator_contract.upgrade(new_class_hash);
}

// New tests for issue #221
#[test]
fn test_update_fee_percentages_by_admin() {
    let (contract, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let new_protocol_fee: u256 = 7; // 7%
    let new_validator_fee: u256 = 3; // 3%
    let mut spy = spy_events();

    // Set caller to admin
    start_cheat_caller_address(contract.contract_address, admin);

    // Update fee percentages
    contract.update_fee_percentages(new_protocol_fee, new_validator_fee);

    // Verify storage updates
    let (protocol_fee, validator_fee, max_fee) = contract.get_fee_percentages();
    assert(protocol_fee == new_protocol_fee, 'Protocol fee not updated');
    assert(validator_fee == new_validator_fee, 'Validator fee not updated');
    assert(max_fee == 10, 'Max fee incorrect');

    // Verify FeeUpdated event
    let events = spy.get_events();
    assert(events.events.len() == 1, 'FeeUpdated event not emitted');
    let expected_event = FeeUpdated {
        protocol_fee_percentage: new_protocol_fee,
        validator_fee_percentage: new_validator_fee,
        max_fee_percentage: 10,
        updated_by: admin,
        timestamp: starknet::get_block_timestamp(),
    };
    let expected_events = array![(contract.contract_address, Event::FeeUpdated(expected_event))];
    spy.assert_emitted(@expected_events);

    stop_cheat_caller_address(contract.contract_address);
}

#[test]
#[should_panic(expected: ('Caller is missing role',))]
fn test_update_fee_percentages_by_non_admin() {
    let (contract, _, _, pool_creator, _) = deploy_predifi();
    let new_protocol_fee: u256 = 7;
    let new_validator_fee: u256 = 3;

    // Set caller to non-admin
    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract.update_fee_percentages(new_protocol_fee, new_validator_fee);
}

#[test]
#[should_panic(expected: ('FEE_EXCEEDS_100_PERCENT',))]
fn test_update_fee_percentages_exceeds_100_percent() {
    let (contract, _, _, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let invalid_protocol_fee: u256 = 101; // Exceeds 100%
    let validator_fee: u256 = 0;

    start_cheat_caller_address(contract.contract_address, admin);
    contract.update_fee_percentages(invalid_protocol_fee, validator_fee);
}

#[test]
#[should_panic(expected: ('FEE_EXCEEDS_MAX',))]
fn test_update_fee_percentages_exceeds_max_fee() {
    let (contract, _, _, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let invalid_protocol_fee: u256 = 15; // Exceeds max_fee_percentage (10%)
    let validator_fee: u256 = 0;

    start_cheat_caller_address(contract.contract_address, admin);
    contract.update_fee_percentages(invalid_protocol_fee, validator_fee);
}

#[test]
fn test_get_fee_percentages() {
    let (contract, _, _, _, _) = deploy_predifi();

    // Initial fees set in constructor (5%, 5%, 10%)
    let (protocol_fee, validator_fee, max_fee) = contract.get_fee_percentages();
    assert(protocol_fee == 5, 'Initial protocol fee incorrect');
    assert(validator_fee == 5, 'Initial validator fee incorrect');
    assert(max_fee == 10, 'Initial max fee incorrect');
}

#[test]
fn test_zero_fees_edge_case() {
    let (contract, _, validator_contract, _, _) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let zero_fee: u256 = 0;
    let mut spy = spy_events();

    // Update fees to 0% as admin
    start_cheat_caller_address(contract.contract_address, admin);
    contract.update_fee_percentages(zero_fee, zero_fee);

    // Verify storage updates
    let (protocol_fee, validator_fee, max_fee) = contract.get_fee_percentages();
    assert(protocol_fee == 0, 'Protocol fee not zero');
    assert(validator_fee == 0, 'Validator fee not zero');
    assert(max_fee == 10, 'Max fee incorrect');

    // Verify FeeUpdated event
    let events = spy.get_events();
    assert(events.events.len() == 1, 'FeeUpdated event not emitted');
    let expected_event = FeeUpdated {
        protocol_fee_percentage: zero_fee,
        validator_fee_percentage: zero_fee,
        max_fee_percentage: 10,
        updated_by: admin,
        timestamp: starknet::get_block_timestamp(),
    };
    let expected_events = array![(contract.contract_address, Event::FeeUpdated(expected_event))];
    spy.assert_emitted(@expected_events);

    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_fees_collected_in_payout() {
    let (contract, _, validator_contract, pool_creator, token_contract) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let pool_id: u256 = 1;
    let mut spy = spy_events();

    // Setup: Create a pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract.create_pool(
        poolName: 'Test Pool',
        poolType: 0,
        poolDescription: "Test description",
        poolImage: "image_url",
        poolEventSourceUrl: "source_url",
        poolStartTime: starknet::get_block_timestamp() + 1000,
        poolLockTime: starknet::get_block_timestamp() + 2000,
        poolEndTime: starknet::get_block_timestamp() + 3000,
        option1: 'Option1',
        option2: 'Option2',
        minBetAmount: 100,
        maxBetAmount: 1000,
        creatorFee: 2,
        isPrivate: false,
        category: 0
    );
    stop_cheat_caller_address(contract.contract_address);

    // Simulate pool stakes
    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract.vote(pool_id, 'Option1', 1000);
    stop_cheat_caller_address(contract.contract_address);

    // Set pool to locked and validate result
    start_cheat_caller_address(validator_contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1); // Locked
    validator_contract.validate_pool_result(pool_id, true); // Option2 wins
    stop_cheat_caller_address(validator_contract.contract_address);

    // Calculate payout (assuming 2 validators for consensus)
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.validate_pool_result(pool_id, true); // Second validator confirms
    stop_cheat_caller_address(validator_contract.contract_address);

    // Verify FeesCollected events
    let events = spy.get_events();
    assert(events.events.len() >= 3, 'FeesCollected events missing');

    let expected_protocol_fee = (1000 * 5) / 100; // 5% of 1000 = 50
    let expected_validator_fee = (1000 * 5) / 100; // 5% of 1000 = 50
    let expected_creator_fee = (1000 * 2) / 100; // 2% of 1000 = 20

    let contract_address = contract.contract_address;
    let expected_events = array![
        (
            contract_address,
            Event::FeesCollected(FeesCollected {
                pool_id,
                fee_type: 'protocol',
                recipient: contract_address,
                amount: expected_protocol_fee
            })
        ),
        (
            contract_address,
            Event::FeesCollected(FeesCollected {
                pool_id,
                fee_type: 'validator',
                recipient: contract_address,
                amount: expected_validator_fee
            })
        ),
        (
            contract_address,
            Event::FeesCollected(FeesCollected {
                pool_id,
                fee_type: 'creator',
                recipient: pool_creator,
                amount: expected_creator_fee
            })
        )
    ];
    spy.assert_emitted(@expected_events);
}