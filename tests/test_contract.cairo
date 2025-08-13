use contract::interfaces::ipredifi::IPredifiValidatorDispatcherTrait;
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


#[test]
fn test_upgrade_by_admin() {
    let (contract, _, validator_contract, _, _) = deploy_predifi();
    let admin = 'admin'.try_into().unwrap();
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
