use contract::base::types::Status;
use contract::interfaces::ipredifi::{IPredifiDispatcherTrait, IPredifiValidatorDispatcherTrait};
use contract::utils::Utils;
use contract::utils::Utils::InternalFunctionsTrait;
use core::array::ArrayTrait;
use core::serde::Serde;
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
use snforge_std::{
    EventSpyTrait, spy_events, start_cheat_block_timestamp, start_cheat_caller_address,
    stop_cheat_block_timestamp, stop_cheat_caller_address, test_address,
};
use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
use starknet::{ContractAddress, get_block_timestamp};
use super::test_utils::{approve_tokens_for_payment, create_default_pool, deploy_predifi};

/// testing access of owner's address value
#[test]
fn test_get_utils_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = 'owner'.try_into().unwrap();
    state.owner.write(owner); // setting the current owner's addrees

    let retrieved_owner = state.get_owner(); // retrieving the owner's address from contract storage
    assert_eq!(retrieved_owner, owner);
}

///  testing contract owner updation by the current contract owner
#[test]
fn test_set_utils_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = 'owner'.try_into().unwrap();
    state.owner.write(owner); // setting the current owner's addrees

    let initial_owner = state.owner.read(); // current owner of Utils contract
    let new_owner: ContractAddress = 'new_owner'.try_into().unwrap();

    let test_address: ContractAddress = test_address();

    start_cheat_caller_address(test_address, initial_owner);

    state
        .set_owner(
            new_owner,
        ); // owner updation, changing contract storage - expect successfull process

    let retrieved_owner = state.owner.read();
    assert_eq!(retrieved_owner, new_owner);
}

/// testing contract onwer updation by a party who is not the current owner
/// expect to panic - only owner can modify the ownership
#[test]
#[should_panic(expected: "Only the owner can set ownership")]
fn test_set_utils_wrong_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = 'owner'.try_into().unwrap();
    state.owner.write(owner); // setting the current owner's addrees

    let new_owner: ContractAddress = 'new_owner'.try_into().unwrap();
    let another_owner: ContractAddress = 'another_owner'.try_into().unwrap();

    let test_address: ContractAddress = test_address();

    start_cheat_caller_address(
        test_address, another_owner,
    ); // cofiguration to call from 'another_owner'

    state.set_owner(new_owner); // expect to panic
}

/// testing contract onwer updation to 0x0
/// expect to panic - cannot assign ownership to 0x0
#[test]
#[should_panic(expected: "Cannot change ownership to 0x0")]
fn test_set_utils_zero_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = 'owner'.try_into().unwrap();
    state.owner.write(owner); // setting the current owner's addrees

    let initial_owner = state.owner.read(); // current owner of Utils contract
    let zero_owner: ContractAddress = 0x0.try_into().unwrap(); // 0x0 address

    let test_address: ContractAddress = test_address();

    start_cheat_caller_address(test_address, initial_owner);

    state.set_owner(zero_owner); // expect to panic
}

/// testing access of pragma contract address value
#[test]
fn test_get_pragma_contract() {
    let mut state = Utils::contract_state_for_testing();
    let pragma: ContractAddress = 'PRAGMA'.try_into().unwrap();
    state.pragma_contract.write(pragma);

    let retrieved_addr = state
        .get_pragma_contract_address(); // reading the pragma contract address from contract storage
    assert_eq!(retrieved_addr, pragma);
}

/// testing pragma contract address updation by owner
#[test]
fn test_set_pragma_contract() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = 'owner'.try_into().unwrap();
    state.owner.write(owner); // setting the current owner's addrees

    let initial_owner = state.owner.read(); // current owner of Utils contract

    let pragma: ContractAddress = 'PRAGMA'.try_into().unwrap();
    state.pragma_contract.write(pragma); // setting the current pragma contract address

    let test_address: ContractAddress = test_address();
    let new_pragma: ContractAddress = 'NEW_PRAGMA'.try_into().unwrap();

    start_cheat_caller_address(test_address, initial_owner);

    state
        .set_pragma_contract_address(
            new_pragma,
        ); // contract address updation, changing contract storage - expect successfull process

    let retrieved_addr = state.pragma_contract.read();
    assert_eq!(retrieved_addr, new_pragma);
}

#[test]
fn test_get_creator_fee_percentage() {
    let (contract, _, _, voter, erc20_address) = deploy_predifi();

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = contract
        .create_pool(
            'Example Pool',
            0, // 0 = WinBet
            "A simple betting pool",
            "image.png",
            "event.com/details",
            1710000000,
            1710003600,
            1710007200,
            'Team A',
            'Team B',
            100,
            10000,
            3,
            false,
            0,
        );

    let creator_fee = contract.get_creator_fee_percentage(pool_id);

    assert(creator_fee == 3, 'Creator fee should be 3%');
}

#[test]
fn test_get_validator_fee_percentage() {
    let (contract, _, validator_contract, voter, erc20_address) = deploy_predifi();

    start_cheat_caller_address(erc20_address, voter);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = contract
        .create_pool(
            'Example Pool',
            0, // 0 = WinBet
            "A simple betting pool",
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

    let validator_fee = validator_contract.get_validator_fee_percentage(pool_id);

    assert(validator_fee == 10, 'Validator fee should be 10%');
}

#[test]
fn test_creator_fee_multiple_pools() {
    let (contract, _, _, voter, erc20_address) = deploy_predifi();

    start_cheat_caller_address(erc20_address, voter);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id1 = contract
        .create_pool(
            'Pool One',
            0, // 0 = WinBet
            "First betting pool",
            "image1.png",
            "event.com/details1",
            1710000000,
            1710003600,
            1710007200,
            'Team A',
            'Team B',
            100,
            10000,
            2,
            false,
            0,
        );

    let pool_id2 = contract
        .create_pool(
            'Pool Two',
            0, // 0 = WinBet
            "Second betting pool",
            "image2.png",
            "event.com/details2",
            1710000000,
            1710003600,
            1710007200,
            'Team X',
            'Team Y',
            200,
            20000,
            4,
            false,
            0,
        );
    stop_cheat_caller_address(contract.contract_address);

    let creator_fee1 = contract.get_creator_fee_percentage(pool_id1);
    let creator_fee2 = contract.get_creator_fee_percentage(pool_id2);

    assert(creator_fee1 == 2, 'Pool 1 creator fee should be 2%');
    assert(creator_fee2 == 4, 'Pool 2 creator fee should be 4%');
}

#[test]
fn test_creator_and_validator_fee_for_same_pool() {
    let (contract, _, validator_contract, voter, erc20_address) = deploy_predifi();
    start_cheat_caller_address(erc20_address, voter);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = contract
        .create_pool(
            'Example Pool',
            0, // 0 = WinBet
            "A simple betting pool",
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

    let creator_fee = contract.get_creator_fee_percentage(pool_id);
    let validator_fee = validator_contract.get_validator_fee_percentage(pool_id);

    assert(creator_fee == 5, 'Creator fee should be 5%');
    assert(validator_fee == 10, 'Validator fee should be 10%');

    let total_fee = creator_fee + validator_fee;
    assert(total_fee == 15, 'Total fee should be 15%');
}

/// testing pragma contract address updation by party who is not an owner
/// expecting panic - only owner can set pragma contract address
#[test]
#[should_panic(expected: "Only the owner can change contract address")]
fn test_set_pragma_contract_wrong_owner() {
    let mut state = Utils::contract_state_for_testing();

    let owner: ContractAddress = 'owner'.try_into().unwrap();
    state.owner.write(owner); // setting the current owner's addrees

    let pragma: ContractAddress = 'PRAGMA'.try_into().unwrap();
    state.pragma_contract.write(pragma); // setting the current pragma contract address

    let another_owner: ContractAddress = 'another_owner'.try_into().unwrap();

    let test_address: ContractAddress = test_address();
    let new_pragma: ContractAddress = 'NEW_PRAGMA'.try_into().unwrap();

    start_cheat_caller_address(
        test_address, another_owner,
    ); // cofiguration to call from 'another_owner'

    state.set_pragma_contract_address(new_pragma); // expect to panic
}

/// testing pragma contract address updation to 0x0
/// expecting panic - cannot changee contract address to 0x0
#[test]
#[should_panic(expected: "Cannot change contract address to 0x0")]
fn test_set_pragma_contract_zero_addr() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = 'owner'.try_into().unwrap();
    state.owner.write(owner); // setting the current owner's addrees

    let initial_owner = state.owner.read(); // current owner of Utils contract

    let pragma: ContractAddress = 'PRAGMA'.try_into().unwrap();
    state.pragma_contract.write(pragma); // setting the current pragma contract address

    let zero_addr: ContractAddress = 0x0.try_into().unwrap(); // 0x0 address

    let test_address: ContractAddress = test_address();

    start_cheat_caller_address(test_address, initial_owner);

    state.set_pragma_contract_address(zero_addr); // expect to panic
}

#[test]
#[should_panic(expected: 'Insufficient STRK balance')]
fn test_insufficient_stark_balance() {
    let (dispatcher, _, _, _, erc20_address) = deploy_predifi();

    let test_addr: ContractAddress = 'test'.try_into().unwrap();
    let erc20 = IERC20Dispatcher { contract_address: erc20_address };
    let balance = erc20.balance_of(test_addr);
    start_cheat_caller_address(erc20_address, test_addr);
    approve_tokens_for_payment(dispatcher.contract_address, erc20_address, balance);
    stop_cheat_caller_address(erc20_address);

    // Test insufficient balance by trying to create a pool with insufficient funds
    start_cheat_caller_address(dispatcher.contract_address, test_addr);
    create_default_pool(dispatcher);
}

#[test]
#[should_panic(expected: 'Insufficient allowance')]
fn test_insufficient_stark_allowance() {
    let (dispatcher, _, _, POOL_CREATOR, erc20_address) = deploy_predifi();

    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    approve_tokens_for_payment(dispatcher.contract_address, erc20_address, 1_000_000);
    stop_cheat_caller_address(erc20_address);

    // Test insufficient allowance by trying to create a pool
    start_cheat_caller_address(dispatcher.contract_address, POOL_CREATOR);
    create_default_pool(dispatcher);
}

// #[test]
// fn test_collect_creation_fee() {
//     let (dispatcher, _, _, POOL_CREATOR, erc20_address) = deploy_predifi();

//     let erc20 = IERC20Dispatcher { contract_address: erc20_address };

//     let initial_contract_balance = erc20.balance_of(dispatcher.contract_address);
//     assert(initial_contract_balance == 0, 'incorrect deployment details');

//     let balance = erc20.balance_of(POOL_CREATOR);
//     start_cheat_caller_address(erc20_address, POOL_CREATOR);
//     erc20.approve(dispatcher.contract_address, balance);
//     stop_cheat_caller_address(erc20_address);

//     // Test that pool creation collects the fee automatically
//     start_cheat_caller_address(dispatcher.contract_address, POOL_CREATOR);
//     create_default_pool(dispatcher);

//     let user_balance_after = erc20.balance_of(POOL_CREATOR);
//     assert(user_balance_after == balance - ONE_STRK, 'deduction failed');

//     let contract_balance_after_collection = erc20.balance_of(dispatcher.contract_address);
//     assert(contract_balance_after_collection == ONE_STRK, 'fee collection failed');
// }

#[test]
fn test_collect_validation_fee() {
    let (_, _, validator_dispatcher, _, _) = deploy_predifi();

    let validation_fee = validator_dispatcher.calculate_validator_fee(54, 10_000);
    assert(validation_fee == 500, 'invalid calculation');
}

#[test]
fn test_distribute_validation_fee() {
    let (mut dispatcher, _, mut validator_dispatcher, POOL_CREATOR, erc20_address) =
        deploy_predifi();

    let validator1 = 'validator1'.try_into().unwrap();
    let validator2 = 'validator2'.try_into().unwrap();
    let validator3 = 'validator3'.try_into().unwrap();
    let validator4 = 'validator4'.try_into().unwrap();

    let erc20 = IERC20Dispatcher { contract_address: erc20_address };

    let admin = 'admin'.try_into().unwrap();
    start_cheat_caller_address(validator_dispatcher.contract_address, admin);
    validator_dispatcher.add_validator(validator1);
    validator_dispatcher.add_validator(validator2);
    validator_dispatcher.add_validator(validator3);
    validator_dispatcher.add_validator(validator4);
    stop_cheat_caller_address(validator_dispatcher.contract_address);

    let initial_contract_balance = erc20.balance_of(dispatcher.contract_address);
    assert(initial_contract_balance == 0, 'incorrect deployment details');

    let balance = erc20.balance_of(POOL_CREATOR);
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    approve_tokens_for_payment(dispatcher.contract_address, erc20_address, balance);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(dispatcher.contract_address, POOL_CREATOR);
    dispatcher.collect_pool_creation_fee(POOL_CREATOR, 1); // Using pool_id 1 for test

    validator_dispatcher.calculate_validator_fee(18, 10_000);

    // Setup event spy before fee distribution
    let mut spy = spy_events();

    start_cheat_caller_address(dispatcher.contract_address, dispatcher.contract_address);
    validator_dispatcher.distribute_validator_fees(18);

    // Check that ValidatorFeesDistributed event was emitted
    let events = spy.get_events();
    assert(events.events.len() > 0, 'No fee distribution events');

    let balance_validator1 = erc20.balance_of(validator1);
    assert(balance_validator1 == 125, 'distribution failed');
    let balance_validator2 = erc20.balance_of(validator2);
    assert(balance_validator2 == 125, 'distribution failed');
    let balance_validator3 = erc20.balance_of(validator3);
    assert(balance_validator3 == 125, 'distribution failed');
    let balance_validator4 = erc20.balance_of(validator4);
    assert(balance_validator4 == 125, 'distribution failed');
}

/// testing if pragma price feed is accessible and returning values
// #[test]
// #[fork("SEPOLIA_LATEST")]
// fn test_get_strk_usd_price() {
//     let (utils_dispatcher, _) = deploy_utils();
//     let strk_in_usd = utils_dispatcher.get_strk_usd_price(); // accessing pragma price feeds
//     assert!(strk_in_usd > 0, "Price should be greater than 0");
// }

#[test]
#[should_panic(expected: 'Unauthorized Caller')]
fn test_unauthorized_manual_update() {
    let (contract, _, _, admin, erc20_address) = deploy_predifi();

    // Random unauthorized address
    let unauthorized: ContractAddress = 'unauthorized'.try_into().unwrap();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval for admin
    start_cheat_caller_address(erc20_address, admin);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create pool as admin
    start_cheat_caller_address(contract.contract_address, admin);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A pool for testing unauthorized updates",
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

    // Attempt unauthorized update - should panic with 'Caller not authorized'
    start_cheat_caller_address(contract.contract_address, unauthorized);
    contract.manually_update_pool_state(pool_id, 1); // This should panic
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
#[should_panic(expected: 'Invalid state transition')]
fn test_invalid_state_transition() {
    let (contract, _, _, user, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval for user
    start_cheat_caller_address(erc20_address, user);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create pool as user
    start_cheat_caller_address(contract.contract_address, user);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A pool for testing invalid transitions",
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

    start_cheat_caller_address(contract.contract_address, admin);
    // Update to Locked
    contract.manually_update_pool_state(pool_id, 1);

    // Try to revert back to Active - should fail with 'Invalid state transition'
    contract.manually_update_pool_state(pool_id, 0);
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_no_change_on_same_state() {
    let (contract, _, _, user, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval for user
    start_cheat_caller_address(erc20_address, user);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create pool as user
    start_cheat_caller_address(contract.contract_address, user);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A pool for testing same state updates",
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

    start_cheat_caller_address(contract.contract_address, admin);
    // Try to update to the same state (Active)
    let same_state = contract.manually_update_pool_state(pool_id, 0);
    stop_cheat_caller_address(contract.contract_address);

    assert(same_state == Status::Active, 'Should return same state');

    // Verify state remains unchanged
    let unchanged_pool = contract.get_pool(pool_id);
    assert(unchanged_pool.status == Status::Active, 'State should not change');
}

#[test]
#[should_panic(expected: 'Pool does not exist')]
fn test_manual_update_nonexistent_pool() {
    let (contract, _, _, admin, _) = deploy_predifi();

    // Try to update a nonexistent pool
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(999, 1); // This should panic
    stop_cheat_caller_address(contract.contract_address);
}


#[test]
fn test_track_user_participation() {
    // Deploy contracts
    let (contract, _, _, user1, erc20_address) = deploy_predifi();

    // Approve token spending for pool creation
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user1);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, user1);
    // Create a test pool
    let pool_id = create_default_pool(contract);

    // Check that user hasn't participated in any pools yet
    assert(contract.get_user_pool_count(user1) == 0, 'Should be 0');
    assert(!contract.has_user_participated_in_pool(user1, pool_id), 'No participation');

    // User votes in the pool
    contract.vote(pool_id, 'Team A', 200);

    // Check that participation is tracked
    assert(contract.get_user_pool_count(user1) == 1, 'Count should be 1');
    assert(contract.has_user_participated_in_pool(user1, pool_id), 'Should participate');

    // Create another pool
    let pool_id2 = create_default_pool(contract);

    // User votes in second pool
    contract.vote(pool_id2, 'Team A', 200);

    // Check count increased
    assert(contract.get_user_pool_count(user1) == 2, 'Count should be 2');

    stop_cheat_caller_address(contract.contract_address);
}


#[test]
fn test_get_user_pools() {
    let (contract, _, _, user, erc20_address) = deploy_predifi();

    // Approve token spending for pool creation
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, user);

    // Create three pools
    let pool_id1 = create_default_pool(contract);
    let pool_id2 = create_default_pool(contract);
    let pool_id3 = create_default_pool(contract);

    // User participates in pools 1 and 3
    contract.vote(pool_id1, 'Team A', 200);
    contract.vote(pool_id3, 'Team A', 200);

    // Get all participated pools
    let user_pools = contract.get_user_pools(user, Option::None);

    // Verify the user has participated in exactly 2 pools
    assert(user_pools.len() == 2, 'Should have 2 pools');

    // Check that pools 1 and 3 are in the array
    // We need to check each value manually
    let mut found_pool1 = false;
    let mut found_pool2 = false;
    let mut found_pool3 = false;

    let mut i = 0;
    while i < user_pools.len() {
        let pool_id = *user_pools.at(i);
        if pool_id == pool_id1 {
            found_pool1 = true;
        } else if pool_id == pool_id2 {
            found_pool2 = true;
        } else if pool_id == pool_id3 {
            found_pool3 = true;
        }
        i += 1;
    }

    assert(found_pool1, 'Pool 1 not found');
    assert(!found_pool2, 'Pool 2 found');
    assert(found_pool3, 'Pool 3 not found');

    stop_cheat_caller_address(contract.contract_address);
}


#[test]
fn test_stake_updates_participation() {
    // Deploy contracts
    let (contract, _, _, user, erc20_address) = deploy_predifi();

    // Approve token spending for pool creation
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, user);
    // Create a test pool
    let pool_id = create_default_pool(contract);
    // Verify user hasn't participated yet
    assert(contract.get_user_pool_count(user) == 0, 'Should be 0');

    // User stakes in the pool
    let stake_amount: u256 = 200_000_000_000_000_000_000;
    contract.stake(pool_id, stake_amount);

    // Check that participation is tracked
    assert(contract.get_user_pool_count(user) == 1, 'Count should be 1');
    assert(contract.has_user_participated_in_pool(user, pool_id), 'Should participate');

    stop_cheat_caller_address(contract.contract_address);
}


#[test]
fn test_multiple_actions_single_pool() {
    // Deploy contracts
    let (contract, _, _, user1, erc20_address) = deploy_predifi();

    // Approve token spending for pool creation

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user1);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, user1);
    // Create a test pool
    let pool_id = create_default_pool(contract);

    // User votes in the pool
    contract.vote(pool_id, 'Team A', 200);

    // Check participation count
    assert(contract.get_user_pool_count(user1) == 1, 'Count should be 1');

    // User also stakes in the same pool
    let stake_amount: u256 = 200_000_000_000_000_000_000;
    contract.stake(pool_id, stake_amount);

    // Count should still be 1 as it's the same pool
    assert(contract.get_user_pool_count(user1) == 1, 'Should still be 1');

    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_multiple_users_pool_tracking() {
    let (contract, _, _, admin, erc20_address) = deploy_predifi();

    // Create two additional users
    let user1 = 1.try_into().unwrap();
    let user2 = 2.try_into().unwrap();
    let admi: ContractAddress = 'admin'.try_into().unwrap();

    // Approve token spending for all users
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Mint some tokens for the users
    start_cheat_caller_address(erc20_address, admin);
    erc20.transfer(user1, 1000_000_000_000_000_000_000);
    erc20.transfer(user2, 1000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Approve for admin
    start_cheat_caller_address(erc20_address, admin);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Approve for user1
    start_cheat_caller_address(erc20_address, user1);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Approve for user2
    start_cheat_caller_address(erc20_address, user2);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Admin creates pools
    start_cheat_caller_address(contract.contract_address, admin);
    let pool_id1 = create_default_pool(contract);
    let pool_id2 = create_default_pool(contract);
    let pool_id3 = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // User1 participates in pools 1 and 2
    start_cheat_caller_address(contract.contract_address, user1);
    contract.vote(pool_id1, 'Team A', 200);
    contract.vote(pool_id2, 'Team A', 200);
    stop_cheat_caller_address(contract.contract_address);

    // User2 participates in pools 2 and 3
    start_cheat_caller_address(contract.contract_address, user2);
    contract.vote(pool_id2, 'Team B', 300);
    contract.vote(pool_id3, 'Team A', 300);
    stop_cheat_caller_address(contract.contract_address);

    // Check user1's pools
    let user1_pools = contract.get_user_pools(user1, Option::None);
    assert(user1_pools.len() == 2, 'User1 should have 2 pools');
    assert(contract.has_user_participated_in_pool(user1, pool_id1), 'User1 should be in pool 1');
    assert(contract.has_user_participated_in_pool(user1, pool_id2), 'User1 should be in pool 2');
    assert(
        !contract.has_user_participated_in_pool(user1, pool_id3), 'User1 should not be in pool 3',
    );

    // Check user2's pools
    let user2_pools = contract.get_user_pools(user2, Option::None);
    assert(user2_pools.len() == 2, 'User2 should have 2 pools');
    assert(
        !contract.has_user_participated_in_pool(user2, pool_id1), 'User2 should not be in pool 1',
    );
    assert(contract.has_user_participated_in_pool(user2, pool_id2), 'User2 should be in pool 2');
    assert(contract.has_user_participated_in_pool(user2, pool_id3), 'User2 should be in pool 3');

    // Admin changes status of pool 2 to locked
    start_cheat_caller_address(contract.contract_address, admi);
    contract.manually_update_pool_state(pool_id2, 1);
    stop_cheat_caller_address(contract.contract_address);

    // Check that pool status changes are reflected for both users
    let user1_active = contract.get_user_active_pools(user1);
    assert(user1_active.len() == 1, 'User1 should have 1 active pool');
    assert(*user1_active.at(0) == pool_id1, 'User1 active pool  1');

    let user1_locked = contract.get_user_locked_pools(user1);
    assert(user1_locked.len() == 1, 'User1 should have 1 locked pool');
    assert(*user1_locked.at(0) == pool_id2, 'User1 locked pool  2');

    let user2_active = contract.get_user_active_pools(user2);
    assert(user2_active.len() == 1, 'User2 should have 1 active pool');
    assert(*user2_active.at(0) == pool_id3, 'User2 active pool 3');

    let user2_locked = contract.get_user_locked_pools(user2);
    assert(user2_locked.len() == 1, 'User2 should have 1 locked pool');
    assert(*user2_locked.at(0) == pool_id2, 'User2 locked  pool 2');
}


#[test]
fn test_get_user_pools_by_status() {
    let (contract, _, _, user, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Approve token spending for pool creation and betting

    // Approve the contract to spend tokens
    start_cheat_caller_address(erc20_address, user);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, user);

    // Create three pools
    let pool_id1 = create_default_pool(contract);
    let pool_id2 = create_default_pool(contract);
    let pool_id3 = create_default_pool(contract);
    let pool_id4 = create_default_pool(contract);

    // User participates in all pools
    contract.vote(pool_id1, 'Team A', 200);
    contract.vote(pool_id2, 'Team A', 200);
    contract.vote(pool_id3, 'Team A', 200);
    contract.vote(pool_id4, 'Team A', 200);

    // All pools should be active by default
    let active_pools = contract.get_user_active_pools(user);
    assert(active_pools.len() == 4, 'Need 4 active pools');

    // No locked, settled, or closed pools yet
    let locked_pools = contract.get_user_locked_pools(user);
    assert(locked_pools.len() == 0, 'Need 0 locked pools');

    let settled_pools = contract.get_user_settled_pools(user);
    assert(settled_pools.len() == 0, 'Need 0 settled pools');
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, admin);

    // Transition pool 2 to Locked status
    contract.manually_update_pool_state(pool_id2, 1);

    // Transition pool 3 to Locked and then to Settled
    contract.manually_update_pool_state(pool_id3, 1);
    contract.manually_update_pool_state(pool_id3, 2);

    // Transition pool 4 through all states to Closed
    contract.manually_update_pool_state(pool_id4, 1);
    contract.manually_update_pool_state(pool_id4, 2);
    contract.manually_update_pool_state(pool_id4, 3);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user);

    // Check active pools - should only be pool 1
    let active_pools = contract.get_user_active_pools(user);
    assert(active_pools.len() == 1, 'Need 1 active pool');
    assert(*active_pools.at(0) == pool_id1, 'Wrong active pool ID');

    // Check locked pools - should only be pool 2
    let locked_pools = contract.get_user_locked_pools(user);
    assert(locked_pools.len() == 1, 'Need 1 locked pool');
    assert(*locked_pools.at(0) == pool_id2, 'Wrong locked pool ID');

    // Check settled pools - should only be pool 3
    let settled_pools = contract.get_user_settled_pools(user);
    assert(settled_pools.len() == 1, 'Need 1 settled pool');
    assert(*settled_pools.at(0) == pool_id3, 'Wrong settled pool ID');

    // Check all pools - should be all 4
    let all_pools = contract.get_user_pools(user, Option::None);
    assert(all_pools.len() == 4, 'Need 4 total pools');

    // Additional verification: Check if the user participation tracking is correct
    assert(contract.has_user_participated_in_pool(user, pool_id1), 'User should be in pool 1');
    assert(contract.has_user_participated_in_pool(user, pool_id2), 'User should be in pool 2');
    assert(contract.has_user_participated_in_pool(user, pool_id3), 'User should be in pool 3');
    assert(contract.has_user_participated_in_pool(user, pool_id4), 'User should be in pool 4');

    // Verify total user pool count
    assert(contract.get_user_pool_count(user) == 4, 'User should have 4 pools');

    stop_cheat_caller_address(contract.contract_address);
}


#[test]
fn test_user_pools_with_time_based_transitions() {
    let (contract, _, _, user, erc20_address) = deploy_predifi();

    // Approve token spending
    start_cheat_caller_address(erc20_address, user);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, user);

    // Get current timestamp
    let current_time = get_block_timestamp();

    // Create pools with different timestamps
    // Pool 1: Standard timeframes
    let pool_id1 = contract
        .create_pool(
            'Pool 1',
            0, // 0 = WinBet
            "First pool",
            "image1.jpg",
            "https://example.com/source1",
            current_time + 3600, // startTime: now + 1 hour
            current_time + 7200, // lockTime: now + 2 hours
            current_time + 10800, // endTime: now + 3 hours
            'Team A',
            'Team B',
            100, // minBetAmount
            1000, // maxBetAmount
            1, // creatorFee
            false, // isPrivate
            0,
        );

    // Pool 2: Shorter timeframes
    let pool_id2 = contract
        .create_pool(
            'Pool 2',
            0, // 0 = WinBet
            "Second pool",
            "image2.jpg",
            "https://example.com/source2",
            current_time + 1800, // startTime: now + 0.5 hour
            current_time + 3600, // lockTime: now + 1 hour
            current_time + 5400, // endTime: now + 1.5 hours
            'Option A',
            'Option B',
            100,
            1000,
            1,
            false,
            3,
        );

    // User participates in both pools
    contract.vote(pool_id1, 'Team A', 200);
    contract.vote(pool_id2, 'Option A', 200);
    stop_cheat_caller_address(contract.contract_address);

    // Initially all pools should be active
    let active_pools = contract.get_user_active_pools(user);
    assert(active_pools.len() == 2, 'Should have 2 active pools');

    // Time warp to when pool 2 should be locked but pool 1 still active
    // Now + 1.25 hours (4500 seconds)
    start_cheat_block_timestamp(contract.contract_address, current_time + 4500);

    // Update the pool states based on current time
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id1, 0);
    contract.manually_update_pool_state(pool_id2, 1);
    stop_cheat_caller_address(contract.contract_address);

    // Check statuses
    let active_pools = contract.get_user_active_pools(user);
    assert(active_pools.len() == 1, 'Should have 1 active pool');
    assert(*active_pools.at(0) == pool_id1, 'Pool 1 should be active');

    let locked_pools = contract.get_user_locked_pools(user);
    assert(locked_pools.len() == 1, 'Should have 1 locked pool');
    assert(*locked_pools.at(0) == pool_id2, 'Pool 2 should be locked');

    // Time warp to when pool 2 should be settled and pool 1 locked
    // Now + 2.5 hours (9000 seconds)
    start_cheat_block_timestamp(contract.contract_address, current_time + 9000);

    // Update the pool states
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id1, 1);
    contract.manually_update_pool_state(pool_id2, 2);
    stop_cheat_caller_address(contract.contract_address);

    // Check statuses
    let active_pools = contract.get_user_active_pools(user);
    assert(active_pools.len() == 0, 'Should have 0 active pools');

    let locked_pools = contract.get_user_locked_pools(user);
    assert(locked_pools.len() == 1, 'Should have 1 locked pool');
    assert(*locked_pools.at(0) == pool_id1, 'Pool 1 should be locked');

    let settled_pools = contract.get_user_settled_pools(user);
    assert(settled_pools.len() == 1, 'Should have 1 settled pool');
    assert(*settled_pools.at(0) == pool_id2, 'Pool 2 should be settled');

    // Time warp to when both pools should be settled
    // Now + 4 hours (14400 seconds)
    start_cheat_block_timestamp(contract.contract_address, current_time + 14400);

    // Update the pool states
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id1, 2);
    contract.manually_update_pool_state(pool_id2, 2);
    stop_cheat_caller_address(contract.contract_address);

    // Check statuses
    let settled_pools = contract.get_user_settled_pools(user);
    assert(settled_pools.len() == 2, 'Should have 2 settled pools');

    // Time warp to 24 hours after pool 2 ended (should transition to closed)
    start_cheat_block_timestamp(contract.contract_address, current_time + 5400 + 86401);

    // Update the pool states
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id2, 3);
    stop_cheat_caller_address(contract.contract_address);

    // The get_user_pools function should still return both pools
    let all_pools = contract.get_user_pools(user, Option::None);
    assert(all_pools.len() == 2, 'Should have 2 total pools');

    // Closed status isn't specifically queried in the contract, but we can check
    // that the pool doesn't appear in other statuses
    let settled_pools = contract.get_user_settled_pools(user);
    assert(settled_pools.len() == 1, 'Should have 1 settled pool');
    assert(*settled_pools.at(0) == pool_id1, 'Only pool 1 should be settled');

    stop_cheat_block_timestamp(contract.contract_address);
}


#[test]
fn test_multiple_users_with_status_transitions() {
    // Deploy contract and deploy_predifi users
    let (contract, _, _, admin, erc20_address) = deploy_predifi();
    let user1 = 1.try_into().unwrap();
    let user2 = 2.try_into().unwrap();
    let user3 = 3.try_into().unwrap();

    // Mint tokens to users
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Admin needs to mint/transfer tokens to the users
    start_cheat_caller_address(erc20_address, admin);
    erc20.transfer(user1, 1000_000_000_000_000_000_000);
    erc20.transfer(user2, 1000_000_000_000_000_000_000);
    erc20.transfer(user3, 1000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Approve token spending for all users
    start_cheat_caller_address(erc20_address, admin);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(erc20_address, user1);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(erc20_address, user2);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(erc20_address, user3);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Get current timestamp
    let current_time = get_block_timestamp();

    // Admin creates pools
    start_cheat_caller_address(contract.contract_address, admin);

    // Create Pool 1: Sports betting
    let pool_id1 = contract
        .create_pool(
            'Soccer Championship',
            0, // 0 = WinBet
            "Finals match",
            "soccer.jpg",
            "https://example.com/soccer",
            current_time + 3600, // startTime: now + 1 hour
            current_time + 7200, // lockTime: now + 2 hours
            current_time + 10800, // endTime: now + 3 hours
            'Team Red',
            'Team Blue',
            100, // minBetAmount
            1000, // maxBetAmount
            1, // creatorFee
            false, // isPrivate
            0,
        );

    // Create Pool 2: Crypto prediction
    let pool_id2 = contract
        .create_pool(
            'ETH Price Prediction',
            0, // 0 = WinBet
            "Price above or below $5000",
            "eth.jpg",
            "https://example.com/eth",
            current_time + 1800, // startTime: now + 0.5 hour
            current_time + 5400, // lockTime: now + 1.5 hours
            current_time + 7200, // endTime: now + 2 hours
            'Above $5000',
            'Below $5000',
            200, // minBetAmount
            2000, // maxBetAmount
            2, // creatorFee
            false, // isPrivate
            3,
        );
    stop_cheat_caller_address(contract.contract_address);

    // User 1 participates in both pools
    start_cheat_caller_address(contract.contract_address, user1);
    contract.vote(pool_id1, 'Team Red', 500);
    contract.vote(pool_id2, 'Above $5000', 600);
    stop_cheat_caller_address(contract.contract_address);

    // User 2 participates in both pools
    start_cheat_caller_address(contract.contract_address, user2);
    contract.vote(pool_id1, 'Team Blue', 300);
    contract.vote(pool_id2, 'Below $5000', 400);
    stop_cheat_caller_address(contract.contract_address);

    // User 3 participates only in pool 1
    start_cheat_caller_address(contract.contract_address, user3);
    contract.vote(pool_id1, 'Team Red', 250);
    stop_cheat_caller_address(contract.contract_address);

    // Check participation
    assert(contract.has_user_participated_in_pool(user1, pool_id1), 'U1 in P1');
    assert(contract.has_user_participated_in_pool(user1, pool_id2), 'U1 in P2');
    assert(contract.has_user_participated_in_pool(user2, pool_id1), 'U2 in P1');
    assert(contract.has_user_participated_in_pool(user2, pool_id2), 'U2 in P2');
    assert(contract.has_user_participated_in_pool(user3, pool_id1), 'U3 in P1');
    assert(!contract.has_user_participated_in_pool(user3, pool_id2), 'U3 not in P2');

    // Initial status check - all pools should be active for participating users
    assert(contract.get_user_active_pools(user1).len() == 2, 'U1: 2 active pools');
    assert(contract.get_user_active_pools(user2).len() == 2, 'U2: 2 active pools');
    assert(contract.get_user_active_pools(user3).len() == 1, 'U3: 1 active pool');

    // Time warp to when pool 2 should be locked but pool 1 still active
    // Now + 1.75 hours (6300 seconds)
    start_cheat_block_timestamp(contract.contract_address, current_time + 6300);

    // Update pool states
    let admin: ContractAddress = 'admin'.try_into().unwrap();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id1, 0);
    contract.manually_update_pool_state(pool_id2, 1);
    stop_cheat_caller_address(contract.contract_address);

    // Check user statuses - pool 2 should be locked for users 1 and 2
    let user1_active = contract.get_user_active_pools(user1);
    assert(user1_active.len() == 1, 'U1: 1 active pool');
    assert(*user1_active.at(0) == pool_id1, 'U1: P1 active');

    let user1_locked = contract.get_user_locked_pools(user1);
    assert(user1_locked.len() == 1, 'U1: 1 locked pool');
    assert(*user1_locked.at(0) == pool_id2, 'U1: P2 locked');

    let user2_active = contract.get_user_active_pools(user2);
    assert(user2_active.len() == 1, 'U2: 1 active pool');
    assert(*user2_active.at(0) == pool_id1, 'U2: P1 active');

    let user2_locked = contract.get_user_locked_pools(user2);
    assert(user2_locked.len() == 1, 'U2: 1 locked pool');
    assert(*user2_locked.at(0) == pool_id2, 'U2: P2 locked');

    // User 3 only has pool 1, which should still be active
    let user3_active = contract.get_user_active_pools(user3);
    assert(user3_active.len() == 1, 'U3: 1 active pool');
    assert(*user3_active.at(0) == pool_id1, 'U3: P1 active');

    // Time warp to when pool 2 should be settled and pool 1 locked
    // Now + 2.5 hours (9000 seconds)
    start_cheat_block_timestamp(contract.contract_address, current_time + 9000);

    // Update pool states
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id1, 1);
    contract.manually_update_pool_state(pool_id2, 2);
    stop_cheat_caller_address(contract.contract_address);

    // Check user statuses - pool 2 should be settled, pool 1 locked
    let user1_active = contract.get_user_active_pools(user1);
    assert(user1_active.len() == 0, 'U1: 0 active pools');

    let user1_locked = contract.get_user_locked_pools(user1);
    assert(user1_locked.len() == 1, 'U1: 1 locked pool');
    assert(*user1_locked.at(0) == pool_id1, 'U1: P1 locked');

    let user1_settled = contract.get_user_settled_pools(user1);
    assert(user1_settled.len() == 1, 'U1: 1 settled pool');
    assert(*user1_settled.at(0) == pool_id2, 'U1: P2 settled');

    // User 2 should have similar status
    let user2_locked = contract.get_user_locked_pools(user2);
    assert(user2_locked.len() == 1, 'U2: 1 locked pool');
    assert(*user2_locked.at(0) == pool_id1, 'U2: P1 locked');

    let user2_settled = contract.get_user_settled_pools(user2);
    assert(user2_settled.len() == 1, 'U2: 1 settled pool');
    assert(*user2_settled.at(0) == pool_id2, 'U2: P2 settled');

    // User 3 should only have pool 1 locked
    let user3_locked = contract.get_user_locked_pools(user3);
    assert(user3_locked.len() == 1, 'U3: 1 locked pool');
    assert(*user3_locked.at(0) == pool_id1, 'U3: P1 locked');

    // Time warp to when both pools should be settled
    // Now + 4 hours (14400 seconds)
    start_cheat_block_timestamp(contract.contract_address, current_time + 14400);

    // Update pool states
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id1, 2);
    contract.manually_update_pool_state(pool_id2, 2);
    stop_cheat_caller_address(contract.contract_address);

    // Check all users should have both pools settled
    let user1_settled = contract.get_user_settled_pools(user1);
    assert(user1_settled.len() == 2, 'U1: 2 settled pools');

    let user2_settled = contract.get_user_settled_pools(user2);
    assert(user2_settled.len() == 2, 'U2: 2 settled pools');

    let user3_settled = contract.get_user_settled_pools(user3);
    assert(user3_settled.len() == 1, 'U3: 1 settled pool');
    assert(*user3_settled.at(0) == pool_id1, 'U3: P1 settled');

    // Time warp to 24 hours after pool 2 ended (transition to closed for pool 2)
    start_cheat_block_timestamp(contract.contract_address, current_time + 7200 + 86401);

    // Update pool states
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id2, 3);
    stop_cheat_caller_address(contract.contract_address);

    // Check settled pools - pool 2 should no longer be in settled status
    let user1_settled = contract.get_user_settled_pools(user1);
    assert(user1_settled.len() == 1, 'U1: 1 settled pool');
    assert(*user1_settled.at(0) == pool_id1, 'U1: only P1 settled');

    let user2_settled = contract.get_user_settled_pools(user2);
    assert(user2_settled.len() == 1, 'U2: 1 settled pool');
    assert(*user2_settled.at(0) == pool_id1, 'U2: only P1 settled');

    // The get_user_pools function should still return all pools for each user
    let user1_all_pools = contract.get_user_pools(user1, Option::None);
    assert(user1_all_pools.len() == 2, 'U1: 2 total pools');

    let user2_all_pools = contract.get_user_pools(user2, Option::None);
    assert(user2_all_pools.len() == 2, 'U2: 2 total pools');

    let user3_all_pools = contract.get_user_pools(user3, Option::None);
    assert(user3_all_pools.len() == 1, 'U3: 1 total pool');

    stop_cheat_block_timestamp(contract.contract_address);
}

#[test]
fn test_bet_placed_event() {
    let (contract, _, _, user, erc20_address) = deploy_predifi();

    // Approve tokens for the user
    start_cheat_caller_address(erc20_address, user);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    // Create a pool
    start_cheat_caller_address(contract.contract_address, user);
    let pool_id = create_default_pool(contract);

    // Setup event spy before voting
    let mut spy = spy_events();

    // Place a bet (vote)
    contract.vote(pool_id, 'Team A', 200);

    // Check that BetPlaced event was emitted
    let events = spy.get_events();
    assert(events.events.len() > 0, 'No bet events');

    stop_cheat_caller_address(contract.contract_address);
}
