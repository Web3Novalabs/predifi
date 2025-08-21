use contract::base::events::Events::{ValidatorAdded, ValidatorRemoved};
use contract::base::types::Status;
use contract::interfaces::ipredifi::{
    IPredifiDispatcherTrait, IPredifiValidator, IPredifiValidatorDispatcherTrait,
};
use contract::predifi::Predifi;
use core::array::ArrayTrait;
use core::serde::Serde;
use core::traits::{Into, TryInto};
use openzeppelin::access::accesscontrol::AccessControlComponent::InternalTrait as AccessControlInternalTrait;
use openzeppelin::access::accesscontrol::DEFAULT_ADMIN_ROLE;
use snforge_std::{
    EventSpyAssertionsTrait, spy_events, start_cheat_block_timestamp, start_cheat_caller_address,
    stop_cheat_block_timestamp, stop_cheat_caller_address, test_address,
};
use starknet::storage::{MutableVecTrait, StoragePointerReadAccess};
use starknet::{ContractAddress, get_block_timestamp};
use super::test_utils::{approve_tokens_for_payment, create_default_pool, deploy_predifi};


#[test]
fn test_validate_pool_result_success() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup ERC20 approval
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create pool
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

    // Move time to lock the pool
    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Verify pool is locked
    let locked_pool = contract.get_pool(pool_id);
    assert(locked_pool.status == Status::Locked, 'Pool should be locked');

    // Add validators
    let admin = 'admin'.try_into().unwrap();
    let validator1 = 'validator1'.try_into().unwrap();
    let validator2 = 'validator2'.try_into().unwrap();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator1);
    validator_contract.add_validator(validator2);
    stop_cheat_caller_address(validator_contract.contract_address);

    // First validator validates - pool should remain locked
    start_cheat_caller_address(validator_contract.contract_address, validator1);
    validator_contract.validate_pool_result(pool_id, true); // Vote for option2
    stop_cheat_caller_address(validator_contract.contract_address);

    let pool_after_first_validation = contract.get_pool(pool_id);
    assert(pool_after_first_validation.status == Status::Locked, 'Pool should still be locked');

    // Check validation status
    let (validation_count, is_settled, _) = validator_contract.get_pool_validation_status(pool_id);
    assert(validation_count == 1, 'Should have 1 validation');
    assert(!is_settled, 'Pool should not be settled yet');

    // Second validator validates - pool should be settled
    start_cheat_caller_address(validator_contract.contract_address, validator2);
    validator_contract.validate_pool_result(pool_id, true); // Vote for option2
    stop_cheat_caller_address(contract.contract_address);

    let pool_after_second_validation = contract.get_pool(pool_id);
    assert(pool_after_second_validation.status == Status::Settled, 'Pool should be settled');

    // Check final validation status
    let (final_validation_count, final_is_settled, final_outcome) = validator_contract
        .get_pool_validation_status(pool_id);
    assert(final_validation_count == 2, 'Should have 2 validations');
    assert(final_is_settled, 'Pool should be settled');
    assert(final_outcome, 'Option2 should win');
}

#[test]
#[should_panic(expected: 'Validator not authorized')]
fn test_validate_pool_result_unauthorized() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();
    // Setup and create pool
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Lock the pool
    let current_time = get_block_timestamp();
    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Try to validate without being a validator
    let unauthorized_user = 'unauthorized'.try_into().unwrap();
    start_cheat_caller_address(validator_contract.contract_address, unauthorized_user);
    validator_contract.validate_pool_result(pool_id, true);
}

#[test]
#[should_panic(expected: 'Validator already validated')]
fn test_validate_pool_result_double_validation() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create and lock pool
    let current_time = get_block_timestamp();
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A test pool",
            "image.png",
            "event.com/details",
            current_time + 100,
            current_time + 200,
            current_time + 300,
            'Team A',
            'Team B',
            100,
            10000,
            5,
            false,
            0,
        );
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Add validator
    let admin = 'admin'.try_into().unwrap();
    let validator = 'validator'.try_into().unwrap();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator);
    stop_cheat_caller_address(validator_contract.contract_address);

    // First validation
    start_cheat_caller_address(validator_contract.contract_address, validator);
    validator_contract.validate_pool_result(pool_id, true);

    // Try to validate again - should panic
    validator_contract.validate_pool_result(pool_id, false);
}

#[test]
#[should_panic(expected: 'Pool not ready for validation')]
fn test_validate_pool_result_wrong_status() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create pool but don't lock it
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Add validator
    let admin = 'admin'.try_into().unwrap();
    let validator = 'validator'.try_into().unwrap();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Try to validate active pool - should panic
    start_cheat_caller_address(validator_contract.contract_address, validator);
    validator_contract.validate_pool_result(pool_id, true);
}

#[test]
fn test_validation_consensus_majority_option1() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create and lock pool
    let current_time = get_block_timestamp();
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A test pool",
            "image.png",
            "event.com/details",
            current_time + 100,
            current_time + 200,
            current_time + 300,
            'Team A',
            'Team B',
            100,
            10000,
            5,
            false,
            0,
        );
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Add 3 validators and set required confirmations to 3
    let admin = 'admin'.try_into().unwrap();
    let validator1 = 'validator1'.try_into().unwrap();
    let validator2 = 'validator2'.try_into().unwrap();
    let validator3 = 'validator3'.try_into().unwrap();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator1);
    validator_contract.add_validator(validator2);
    validator_contract.add_validator(validator3);
    validator_contract.set_required_validator_confirmations(3);
    stop_cheat_caller_address(contract.contract_address);

    // Validators vote: 2 for option1 (false), 1 for option2 (true)
    start_cheat_caller_address(validator_contract.contract_address, validator1);
    validator_contract.validate_pool_result(pool_id, false); // Option1
    stop_cheat_caller_address(validator_contract.contract_address);

    start_cheat_caller_address(validator_contract.contract_address, validator2);
    validator_contract.validate_pool_result(pool_id, false); // Option1
    stop_cheat_caller_address(validator_contract.contract_address);

    start_cheat_caller_address(validator_contract.contract_address, validator3);
    validator_contract.validate_pool_result(pool_id, true); // Option2 - this triggers settlement
    stop_cheat_caller_address(validator_contract.contract_address);

    // Check final outcome - should be option1 (false) since it got majority
    let (_, is_settled, final_outcome) = validator_contract.get_pool_validation_status(pool_id);
    assert(is_settled, 'Pool should be settled');
    assert(!final_outcome, 'Option1 should win majority');

    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Settled, 'Pool should be settled');
}


#[test]
fn test_get_validator_confirmation() {
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup and create locked pool
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    let current_time = get_block_timestamp();
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A test pool",
            "image.png",
            "event.com/details",
            current_time + 100,
            current_time + 200,
            current_time + 300,
            'Team A',
            'Team B',
            100,
            10000,
            5,
            false,
            0,
        );
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Add validator
    let admin = 'admin'.try_into().unwrap();
    let validator = 'validator'.try_into().unwrap();

    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Check before validation
    let (has_validated, _) = validator_contract.get_validator_confirmation(pool_id, validator);
    assert(!has_validated, 'Should not have validated yet');

    // Validate
    start_cheat_caller_address(validator_contract.contract_address, validator);
    validator_contract.validate_pool_result(pool_id, true);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Check after validation
    let (has_validated_after, selected_option) = validator_contract
        .get_validator_confirmation(pool_id, validator);
    assert(has_validated_after, 'Should have validated');
    assert(selected_option, 'Should have selected option2');
}


#[test]
fn test_validator_can_update_state() {
    let (mut contract, _, mut validator_contract, admin, erc20_address) = deploy_predifi();

    // Create a validator
    let validator = 'validator'.try_into().unwrap();

    // Add token approval for admin
    start_cheat_caller_address(erc20_address, admin);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Add validators
    let admin_role = 'admin'.try_into().unwrap();
    start_cheat_caller_address(validator_contract.contract_address, admin_role);
    validator_contract.add_validator(validator);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Get current time
    let current_time = get_block_timestamp();

    // Create a pool using admin
    start_cheat_caller_address(contract.contract_address, admin);
    let pool_id = contract
        .create_pool(
            'Validator Test Pool',
            0, // 0 = WinBet
            "A pool for testing validator updates",
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
    stop_cheat_caller_address(contract.contract_address);

    // Validator updates state
    start_cheat_caller_address(contract.contract_address, validator);
    let updated_state = contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);

    assert(updated_state == Status::Locked, 'Validator update should succeed');

    // Verify state change
    let updated_pool = contract.get_pool(pool_id);
    assert(updated_pool.status == Status::Locked, 'should be updated by validator');
}


#[test]
fn test_assign_random_validators() {
    // Deploy the contract
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Create validators
    let validator1 = 'validator1'.try_into().unwrap();
    let validator2 = 'validator2'.try_into().unwrap();
    let validator3 = 'validator3'.try_into().unwrap();
    let validator4 = 'validator4'.try_into().unwrap();
    let zero_address: ContractAddress = 'zero'.try_into().unwrap();

    // Add validators to the contract
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator1);
    validator_contract.add_validator(validator2);
    validator_contract.add_validator(validator3);
    validator_contract.add_validator(validator4);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Set up token approval for pool creation
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create a pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Assign random validators to the pool
    validator_contract.assign_random_validators(pool_id);

    // Get the assigned validators
    let (assigned_validator1, assigned_validator2) = validator_contract
        .get_pool_validators(pool_id);

    // Verify that validators were assigned
    assert(assigned_validator1 != zero_address, 'Validator1 should be assigned');
    assert(assigned_validator2 != zero_address, 'Validator2 should be assigned');

    // Verify that the assigned validators are from our added validators
    let is_valid_validator1 = assigned_validator1 == validator1
        || assigned_validator1 == validator2
        || assigned_validator1 == validator3
        || assigned_validator1 == validator4;

    let is_valid_validator2 = assigned_validator2 == validator1
        || assigned_validator2 == validator2
        || assigned_validator2 == validator3
        || assigned_validator2 == validator4;

    assert(is_valid_validator1, 'Invalid validator1 assigned');
    assert(is_valid_validator2, 'Invalid validator2 assigned');
}

#[test]
fn test_assign_exactly_two_validators() {
    // Deploy the contract
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Create exactly two validators with different addresses
    let validator1 = 'validator1'.try_into().unwrap();
    let validator2 = 'validator2'.try_into().unwrap();
    let zero_address: ContractAddress = 'zero'.try_into().unwrap();

    // Add validators to the contract (overriding any existing validators)
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator1);
    validator_contract.add_validator(validator2);
    stop_cheat_caller_address(contract.contract_address);

    // Set up token approval for pool creation
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create a pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Assign random validators to the pool
    validator_contract.assign_random_validators(pool_id);

    // Get the assigned validators
    let (assigned_validator1, assigned_validator2) = validator_contract
        .get_pool_validators(pool_id);

    // Verify that validators were assigned
    assert(assigned_validator1 != zero_address, 'Validator1 should be assigned');
    assert(assigned_validator2 != zero_address, 'Validator2 should be assigned');

    // Verify that the assigned validators are from our added validators
    assert(
        assigned_validator1 == validator1 || assigned_validator1 == validator2,
        'Invalid validator1 assigned',
    );
    assert(
        assigned_validator2 == validator1 || assigned_validator2 == validator2,
        'Invalid validator2 assigned',
    );

    // Create multiple pools to test consistency
    let num_pools = 3;
    let mut pool_ids: Array<u256> = ArrayTrait::new();

    start_cheat_caller_address(contract.contract_address, pool_creator);

    // Create additional pools
    let mut i: u8 = 0;
    while i < num_pools {
        let pool_id = create_default_pool(contract);
        pool_ids.append(pool_id);
        i += 1;
    }
    stop_cheat_caller_address(contract.contract_address);

    // Assign validators to all pools
    let mut j: u8 = 0;
    while j < num_pools {
        let pool_id = *pool_ids.at(j.into());
        validator_contract.assign_random_validators(pool_id);
        j += 1;
    }

    // Verify all pools have valid validators assigned
    let mut k: u8 = 0;
    while k < num_pools {
        let pool_id = *pool_ids.at(k.into());
        let (pool_validator1, pool_validator2) = validator_contract.get_pool_validators(pool_id);

        // Verify validators are assigned
        assert(pool_validator1 != zero_address, 'Pool validator1 not assigned');
        assert(pool_validator2 != zero_address, 'Pool validator2 not assigned');

        // Verify validators are from our added validators
        assert(
            pool_validator1 == validator1 || pool_validator1 == validator2,
            'Invalid pool validator1',
        );
        assert(
            pool_validator2 == validator1 || pool_validator2 == validator2,
            'Invalid pool validator2',
        );

        k += 1;
    }
}

#[test]
fn test_assign_multiple_validators() {
    // Deploy the contract
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Create multiple validators with different addresses
    let validator1 = 'validator1'.try_into().unwrap();
    let validator2 = 'validator2'.try_into().unwrap();
    let validator3 = 'validator3'.try_into().unwrap();
    let validator4 = 'validator4'.try_into().unwrap();

    // Add validators to the contract
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(validator1);
    validator_contract.add_validator(validator2);
    validator_contract.add_validator(validator3);
    validator_contract.add_validator(validator4);
    stop_cheat_caller_address(contract.contract_address);

    // Set up token approval for pool creation
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 1_000_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Get current timestamp
    let current_time = get_block_timestamp();

    // Create multiple pools
    let mut pool_ids: Array<u256> = ArrayTrait::new();
    let num_pools = 5;

    start_cheat_caller_address(contract.contract_address, pool_creator);

    // Create 5 different pools
    let mut i: u8 = 0;
    while i < num_pools {
        let pool_id = contract
            .create_pool(
                'Test Pool', // poolName
                0, // 0 = WinBet // poolType
                "Test pool description", // poolDescription
                "image.jpg", // poolImage
                "https://example.com", // poolEventSourceUrl
                current_time + 50, // poolStartTime (future)
                current_time + 150, // poolLockTime (future)
                current_time + 1000, // poolEndTime (future)
                'Yes', // option1
                'No', // option2
                1_000_000_000_000_000_000, // minBetAmount (1 token)
                100_000_000_000_000_000_000, // maxBetAmount (100 tokens)
                5, // creatorFee (5%)
                false, // isPrivate
                0 // category
            );
        pool_ids.append(pool_id);
        i += 1;
    }
    stop_cheat_caller_address(contract.contract_address);

    // Assign validators to each pool
    let mut i: u32 = 0;
    while i < pool_ids.len() {
        let pool_id = *pool_ids.at(i);
        validator_contract.assign_random_validators(pool_id);
        i += 1;
    }

    // Check that validators are distributed across pools
    // We should see different validators assigned to different pools
    let mut validator1_count = 0_u8;
    let mut validator2_count = 0_u8;
    let mut validator3_count = 0_u8;
    let mut validator4_count = 0_u8;

    let mut i: u32 = 0;
    while i < pool_ids.len() {
        let pool_id = *pool_ids.at(i);
        let (assigned_validator1, assigned_validator2) = validator_contract
            .get_pool_validators(pool_id);

        // Count how many times each validator is assigned
        if assigned_validator1 == validator1 || assigned_validator2 == validator1 {
            validator1_count += 1;
        }

        if assigned_validator1 == validator2 || assigned_validator2 == validator2 {
            validator2_count += 1;
        }

        if assigned_validator1 == validator3 || assigned_validator2 == validator3 {
            validator3_count += 1;
        }

        if assigned_validator1 == validator4 || assigned_validator2 == validator4 {
            validator4_count += 1;
        }

        i += 1;
    }

    // Verify that at least 3 different validators were used
    // This is a simple check to ensure some level of distribution
    let mut validators_used = 0;
    if validator1_count > 0 {
        validators_used += 1;
    }
    if validator2_count > 0 {
        validators_used += 1;
    }
    if validator3_count > 0 {
        validators_used += 1;
    }
    if validator4_count > 0 {
        validators_used += 1;
    }

    // With 5 pools and 4 validators, we should see at least 3 different validators used
    assert(validators_used >= 3_u8, 'Not enough validators used');
}

#[test]
fn test_limited_validators_assignment() {
    // Deploy the contract
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Create just one validator
    let single_validator = 'single_validator'.try_into().unwrap();

    // Add only one validator to the contract
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(single_validator);
    stop_cheat_caller_address(contract.contract_address);

    // Set up token approval for pool creation
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 1_000_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Get current timestamp
    let current_time = get_block_timestamp();

    // Create multiple pools
    let mut pool_ids: Array<u256> = ArrayTrait::new();
    let num_pools = 3;

    start_cheat_caller_address(contract.contract_address, pool_creator);

    // Create 3 different pools
    let mut i: u8 = 0;
    while i < num_pools {
        let pool_id = contract
            .create_pool(
                'Limited Test Pool', // poolName
                0, // 0 = WinBet // poolType
                "Testing limited validators", // poolDescription
                "image.jpg", // poolImage
                "https://example.com", // poolEventSourceUrl
                current_time + 50, // poolStartTime (future)
                current_time + 150, // poolLockTime (future)
                current_time + 1000, // poolEndTime (future)
                'Yes', // option1
                'No', // option2
                1_000_000_000_000_000_000, // minBetAmount (1 token)
                100_000_000_000_000_000_000, // maxBetAmount (100 tokens)
                5, // creatorFee (5%)
                false, // isPrivate
                0 // category
            );
        pool_ids.append(pool_id);
        i += 1;
    }
    stop_cheat_caller_address(contract.contract_address);

    // Assign validators to each pool
    let mut i: u32 = 0;
    while i < pool_ids.len() {
        let pool_id = *pool_ids.at(i);
        validator_contract.assign_random_validators(pool_id);
        i += 1;
    }

    // Check that all pools have the same validator assigned
    // Since there's only one validator, it should be assigned to all pools
    let mut i: u32 = 0;
    while i < pool_ids.len() {
        let pool_id = *pool_ids.at(i);
        let (assigned_validator1, assigned_validator2) = validator_contract
            .get_pool_validators(pool_id);

        // Both validator1 and validator2 should be the single validator we added
        assert(assigned_validator1 == single_validator, 'Wrong validator1 assigned');
        assert(assigned_validator2 == single_validator, 'Wrong validator2 assigned');

        i += 1;
    }

    // Now add a second validator and verify it gets used for new pools
    let second_validator = 'second_validator'.try_into().unwrap();

    // Add second validator to the contract
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(second_validator);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Create one more pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let new_pool_id = contract
        .create_pool(
            'New Pool', // poolName
            0, // 0 = WinBet // poolType
            "Testing with two validators", // poolDescription
            "image.jpg", // poolImage
            "https://example.com", // poolEventSourceUrl
            current_time + 50, // poolStartTime (future)
            current_time + 150, // poolLockTime (future)
            current_time + 1000, // poolEndTime (future)
            'Yes', // option1
            'No', // option2
            1_000_000_000_000_000_000, // minBetAmount (1 token)
            100_000_000_000_000_000_000, // maxBetAmount (100 tokens)
            5, // creatorFee (5%)
            false, // isPrivate
            0 // category
        );
    stop_cheat_caller_address(contract.contract_address);

    // Assign validators to the new pool
    validator_contract.assign_random_validators(new_pool_id);

    // Check that the new pool has different validators assigned
    let (new_assigned_validator1, new_assigned_validator2) = validator_contract
        .get_pool_validators(new_pool_id);

    // At least one of the validators should be the second validator
    let has_second_validator = new_assigned_validator1 == second_validator
        || new_assigned_validator2 == second_validator;

    assert(has_second_validator, 'Second validator not used');
}


#[test]
fn test_assign_random_validators_initial_validator() {
    // Deploy the contract
    let (contract, _, validator_contract, pool_creator, erc20_address) = deploy_predifi();

    // Get the validator that was added during deployment
    let expected_validator = 'validator'.try_into().unwrap();

    // Explicitly add the validator to the validators list
    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(expected_validator);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Set up token approval for pool creation
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create a pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Assign random validators to the pool
    validator_contract.assign_random_validators(pool_id);

    // Get the assigned validators
    let (assigned_validator1, assigned_validator2) = validator_contract
        .get_pool_validators(pool_id);

    // Verify that both assigned validators are the expected validator
    assert(assigned_validator1 == expected_validator, 'Should assign initial valdator');
    assert(assigned_validator2 == expected_validator, 'Should assign initial validator');
}

#[test]
fn test_add_validator() {
    let mut state = Predifi::contract_state_for_testing();
    let test_address: ContractAddress = test_address();

    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let validator: ContractAddress = 'validator'.try_into().unwrap();

    // Initialize access control and grant DEFAULT_ADMIN_ROLE to admin
    AccessControlInternalTrait::initializer(ref state.accesscontrol);
    AccessControlInternalTrait::_grant_role(ref state.accesscontrol, DEFAULT_ADMIN_ROLE, admin);

    // Act as admin to add a validator
    start_cheat_caller_address(test_address, admin);
    let mut spy = spy_events();
    IPredifiValidator::add_validator(ref state, validator);
    stop_cheat_caller_address(test_address);

    // Assert validator is added to the list
    let added_validator: ContractAddress = state.validators.at(0).read();
    assert(added_validator == validator, 'Validator not added');

    // Assert validator role is set
    let is_validator = IPredifiValidator::is_validator(@state, validator);
    assert(is_validator, 'Validator role not set');

    // Assert event emitted
    let expected_event = Predifi::Event::ValidatorAdded(
        ValidatorAdded { account: validator, caller: admin },
    );
    spy.assert_emitted(@array![(test_address, expected_event)]);
}

#[test]
#[should_panic(expected: 'Caller is missing role')]
fn test_add_validator_unauthorized() {
    let mut state = Predifi::contract_state_for_testing();
    let validator: ContractAddress = 'validator'.try_into().unwrap();

    AccessControlInternalTrait::initializer(ref state.accesscontrol);

    // Unauthorized caller attempt to add a new validator
    IPredifiValidator::add_validator(ref state, validator);
}

#[test]
fn test_remove_validator_role() {
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    let validator1: ContractAddress = 'validator1'.try_into().unwrap();
    let validator2: ContractAddress = 'validator2'.try_into().unwrap();

    let mut state = Predifi::contract_state_for_testing();
    let test_address: ContractAddress = test_address();

    // Initialize access control and grant DEFAULT_ADMIN_ROLE to admin
    AccessControlInternalTrait::initializer(ref state.accesscontrol);
    AccessControlInternalTrait::_grant_role(ref state.accesscontrol, DEFAULT_ADMIN_ROLE, admin);

    // Act as admin to add two validators
    start_cheat_caller_address(test_address, admin);
    IPredifiValidator::add_validator(ref state, validator1);
    IPredifiValidator::add_validator(ref state, validator2);
    stop_cheat_caller_address(test_address);

    // Act as admin to remove validator1
    start_cheat_caller_address(test_address, admin);
    let mut spy = spy_events();
    IPredifiValidator::remove_validator(ref state, validator1);
    stop_cheat_caller_address(test_address);

    // Assert only one validator remains
    let validator_count = state.validators.len();
    assert(validator_count == 1, 'Expected only one validator');

    // Assert validator2 is the remaining validator
    let remaining_validator: ContractAddress = state.validators.at(0).read();
    assert(remaining_validator == validator2, 'Validator2 should remain');

    // Assert validator1 role is revoked
    let is_validator = IPredifiValidator::is_validator(@state, validator1);
    assert(!is_validator, 'Validator1 was not revoked');

    // Assert correct event was emitted
    let expected_event = Predifi::Event::ValidatorRemoved(
        ValidatorRemoved { account: validator1, caller: admin },
    );
    spy.assert_emitted(@array![(test_address, expected_event)]);

    // Act as admin to remove the second validator
    start_cheat_caller_address(test_address, admin);
    IPredifiValidator::remove_validator(ref state, validator2);
    stop_cheat_caller_address(test_address);

    // Assert no validators remain
    let validator_count = state.validators.len();
    assert(validator_count == 0, 'Expected zero validators');
}

#[test]
#[should_panic(expected: 'Caller is missing role')]
fn test_remove_validator_unauthorized() {
    let mut state = Predifi::contract_state_for_testing();
    let validator: ContractAddress = 'validator'.try_into().unwrap();

    AccessControlInternalTrait::initializer(ref state.accesscontrol);

    // Unauthorized caller attempt to remove the validator role
    IPredifiValidator::remove_validator(ref state, validator);
}
