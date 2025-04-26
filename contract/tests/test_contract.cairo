use contract::base::types::{Category, Pool, PoolDetails, Status};
use contract::interfaces::iUtils::{IUtilityDispatcher, IUtilityDispatcherTrait};
use contract::interfaces::ipredifi::{IPredifiDispatcher, IPredifiDispatcherTrait};
use contract::utils::Utils;
use contract::utils::Utils::InternalFunctionsTrait;
use core::array::ArrayTrait;
use core::felt252;
use core::serde::Serde;
use core::traits::{Into, TryInto};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, declare, start_cheat_caller_address,
    stop_cheat_caller_address, test_address,
};
use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
use starknet::{
    ClassHash, ContractAddress, contract_address_const, get_block_timestamp, get_caller_address,
    get_contract_address,
};

// Validator role
const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");
// Pool creator address constant
const POOL_CREATOR: ContractAddress = 123.try_into().unwrap();

fn deploy_predifi() -> IPredifiDispatcher {
    let contract_class = declare("Predifi").unwrap().contract_class();

    let (contract_address, _) = contract_class.deploy(@array![].into()).unwrap();
    (IPredifiDispatcher { contract_address })
}

// Helper function for creating pools with default parameters
fn create_default_pool(contract: IPredifiDispatcher) -> u256 {
    contract
        .create_pool(
            'Example Pool',
            Pool::WinBet,
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
            Category::Sports,
        )
}

const ONE_STRK: u256 = 1_000_000_000_000_000_000;

#[test]
fn test_create_pool() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");
}

#[test]
#[should_panic(expected: "Start time must be before lock time")]
fn test_invalid_time_sequence_start_after_lock() {
    let contract = deploy_predifi();
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
#[should_panic(expected: "Minimum bet must be greater than 0")]
fn test_zero_min_bet() {
    let contract = deploy_predifi();
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
#[should_panic(expected: "Creator fee cannot exceed 5%")]
fn test_excessive_creator_fee() {
    let contract = deploy_predifi();
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

fn get_default_pool_params() -> (
    felt252,
    Pool,
    ByteArray,
    ByteArray,
    ByteArray,
    u64,
    u64,
    u64,
    felt252,
    felt252,
    u256,
    u256,
    u8,
    bool,
    Category,
) {
    let current_time = get_block_timestamp();
    (
        'Default Pool',
        Pool::WinBet,
        "Default Description",
        "default_image.jpg",
        "https://example.com",
        current_time + 86400,
        current_time + 172800,
        current_time + 259200,
        'Option A',
        'Option B',
        1_000_000_000_000_000_000,
        10_000_000_000_000_000_000,
        5,
        false,
        Category::Sports,
    )
}

#[test]
fn test_vote() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team A', 200);

    let pool = contract.get_pool(pool_id);
    assert(pool.totalBetCount == 1, 'Total bet count should be 1');
    assert(pool.totalStakeOption1 == 200, 'Total stake should be 200');
    assert(pool.totalSharesOption1 == 199, 'Total share should be 199');
}

#[test]
fn test_vote_with_user_stake() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);

    let pool = contract.get_pool(pool_id);
    contract.vote(pool_id, 'Team A', 200);

    let user_stake = contract.get_user_stake(pool_id, pool.address);
    assert(user_stake.amount == 200, 'Incorrect amount');
    assert(user_stake.shares == 199, 'Incorrect shares');
    assert(!user_stake.option, 'Incorrect option');
}

#[test]
fn test_successful_get_pool() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    let pool = contract.get_pool(pool_id);
    assert(pool.poolName == 'Example Pool', 'Pool not found');
}

#[test]
#[should_panic(expected: 'Invalid Pool Option')]
fn test_when_invalid_option_is_pass() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team C', 200);
}

#[test]
#[should_panic(expected: 'Amount is below minimum')]
fn test_when_min_bet_amount_less_than_required() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team A', 10);
}

#[test]
#[should_panic(expected: 'Amount is above maximum')]
fn test_when_max_bet_amount_greater_than_required() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team B', 1000000);
}

#[test]
fn test_get_pool_odds() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team A', 100);

    let pool_odds = contract.pool_odds(pool_id);
    assert(pool_odds.option1_odds == 2500, 'Incorrect odds for option 1');
    assert(pool_odds.option2_odds == 7500, 'Incorrect odds for option 2');
}

#[test]
fn test_get_pool_stakes() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team A', 200);

    let pool_stakes = contract.get_pool_stakes(pool_id);
    assert(pool_stakes.amount == 200, 'Incorrect pool stake amount');
    assert(pool_stakes.shares == 199, 'Incorrect pool stake shares');
    assert(!pool_stakes.option, 'Incorrect pool stake option');
}

#[test]
fn test_unique_pool_id() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");
}

#[test]
fn test_unique_pool_id_when_called_twice_in_the_same_execution() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    let pool_id1 = create_default_pool(contract);

    assert!(pool_id != 0, "not created");
    assert!(pool_id != pool_id1, "they are the same");
}

#[test]
fn test_get_pool_vote() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team A', 200);

    let pool_vote = contract.get_pool_vote(pool_id);
    assert(!pool_vote, 'Incorrect pool vote');
}

#[test]
fn test_get_pool_count() {
    let contract = deploy_predifi();
    assert(contract.get_pool_count() == 0, 'Initial pool count should be 0');
    create_default_pool(contract);
    assert(contract.get_pool_count() == 1, 'Pool count should be 1');
}

#[test]
fn test_stake_successful() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);
    let caller = contract_address_const::<1>();
    let stake_amount: u256 = 200_000_000_000_000_000_000;

    start_cheat_caller_address(contract.contract_address, caller);
    contract.stake(pool_id, stake_amount);
    stop_cheat_caller_address(contract.contract_address);

    assert(contract.get_user_stake(pool_id, caller).amount == stake_amount, 'Invalid stake amount');
}

#[test]
fn test_get_pool_creator() {
    let contract = deploy_predifi();

    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    assert!(pool_id != 0, "not created");
    assert!(contract.get_pool_creator(pool_id) == POOL_CREATOR, "incorrect creator");
}

fn deploy_utils() -> (IUtilityDispatcher, ContractAddress) {
    let utils_contract_class = declare("Utils")
        .unwrap()
        .contract_class(); // contract class declaration

    let owner: ContractAddress = get_caller_address(); //setting the current owner's address
    let pragma_address: ContractAddress =
        0x036031daa264c24520b11d93af622c848b2499b66b41d611bac95e13cfca131a
        .try_into()
        .unwrap(); // pragma contract address - Starknet Sepolia testnet

    let mut constructor_calldata =
        array![]; // constructor call data as an array of felt252 elements
    Serde::serialize(@owner, ref constructor_calldata);
    Serde::serialize(@pragma_address, ref constructor_calldata);

    let (utils_contract, _) = utils_contract_class
        .deploy(@constructor_calldata)
        .unwrap(); //deployment process
    let utils_dispatcher = IUtilityDispatcher { contract_address: utils_contract };

    return (utils_dispatcher, utils_contract); // dispatcher and deployed contract adddress
}

/// testing access of owner's address value
#[test]
fn test_get_utils_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner); // setting the current owner's addrees

    let retrieved_owner = state.get_owner(); // retrieving the owner's address from contract storage
    assert_eq!(retrieved_owner, owner);
}

///  testing contract owner updation by the current contract owner
#[test]
fn test_set_utils_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner); // setting the current owner's addrees

    let initial_owner = state.owner.read(); // current owner of Utils contract
    let new_owner: ContractAddress = contract_address_const::<'new_owner'>();

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
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner); // setting the current owner's addrees

    let new_owner: ContractAddress = contract_address_const::<'new_owner'>();
    let another_owner: ContractAddress = contract_address_const::<'another_owner'>();

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
    let owner: ContractAddress = contract_address_const::<'owner'>();
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
    let pragma: ContractAddress = contract_address_const::<'PRAGMA'>();
    state.pragma_contract.write(pragma);

    let retrieved_addr = state
        .get_pragma_contract_address(); // reading the pragma contract address from contract storage
    assert_eq!(retrieved_addr, pragma);
}

/// testing pragma contract address updation by owner
#[test]
fn test_set_pragma_contract() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner); // setting the current owner's addrees

    let initial_owner = state.owner.read(); // current owner of Utils contract

    let pragma: ContractAddress = contract_address_const::<'PRAGMA'>();
    state.pragma_contract.write(pragma); // setting the current pragma contract address

    let test_address: ContractAddress = test_address();
    let new_pragma: ContractAddress = contract_address_const::<'NEW_PRAGMA'>();

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
    let contract = deploy_predifi();

    let pool_id = contract
        .create_pool(
            'Example Pool',
            Pool::WinBet,
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
            Category::Sports,
        );

    let creator_fee = contract.get_creator_fee_percentage(pool_id);

    assert(creator_fee == 3, 'Creator fee should be 3%');
}

#[test]
fn test_get_validator_fee_percentage() {
    let contract = deploy_predifi();

    let pool_id = contract
        .create_pool(
            'Example Pool',
            Pool::WinBet,
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
            Category::Sports,
        );

    let validator_fee = contract.get_validator_fee_percentage(pool_id);

    assert(validator_fee == 10, 'Validator fee should be 10%');
}

#[test]
fn test_creator_fee_multiple_pools() {
    let contract = deploy_predifi();

    let pool_id1 = contract
        .create_pool(
            'Pool One',
            Pool::WinBet,
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
            Category::Sports,
        );

    let pool_id2 = contract
        .create_pool(
            'Pool Two',
            Pool::WinBet,
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
            Category::Sports,
        );

    let creator_fee1 = contract.get_creator_fee_percentage(pool_id1);
    let creator_fee2 = contract.get_creator_fee_percentage(pool_id2);

    assert(creator_fee1 == 2, 'Pool 1 creator fee should be 2%');
    assert(creator_fee2 == 4, 'Pool 2 creator fee should be 4%');
}

#[test]
fn test_creator_and_validator_fee_for_same_pool() {
    let contract = deploy_predifi();

    let pool_id = contract
        .create_pool(
            'Example Pool',
            Pool::WinBet,
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
            Category::Sports,
        );

    let creator_fee = contract.get_creator_fee_percentage(pool_id);
    let validator_fee = contract.get_validator_fee_percentage(pool_id);

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

    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner); // setting the current owner's addrees

    let initial_owner = state.owner.read(); // current owner of Utils contract

    let pragma: ContractAddress = contract_address_const::<'PRAGMA'>();
    state.pragma_contract.write(pragma); // setting the current pragma contract address

    let another_owner: ContractAddress = contract_address_const::<'another_owner'>();

    let test_address: ContractAddress = test_address();
    let new_pragma: ContractAddress = contract_address_const::<'NEW_PRAGMA'>();

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
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner); // setting the current owner's addrees

    let initial_owner = state.owner.read(); // current owner of Utils contract

    let pragma: ContractAddress = contract_address_const::<'PRAGMA'>();
    state.pragma_contract.write(pragma); // setting the current pragma contract address

    let zero_addr: ContractAddress = 0x0.try_into().unwrap(); // 0x0 address

    let test_address: ContractAddress = test_address();

    start_cheat_caller_address(test_address, initial_owner);

    state.set_pragma_contract_address(zero_addr); // expect to panic
}

#[test]
fn test_track_user_participation() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);

    let user = contract_address_const::<42>();
    start_cheat_caller_address(contract.contract_address, user);

    // Check that user hasn't participated in any pools yet
    assert(contract.get_user_pool_count(user) == 0, 'Should be 0');
    assert(!contract.has_user_participated_in_pool(user, pool_id), 'No participation');

    // User votes in the pool
    contract.vote(pool_id, 'Team A', 200);

    // Check that participation is tracked
    assert(contract.get_user_pool_count(user) == 1, 'Count should be 1');
    assert(contract.has_user_participated_in_pool(user, pool_id), 'Should participate');

    // Create another pool
    let pool_id2 = create_default_pool(contract);

    // User votes in second pool
    contract.vote(pool_id2, 'Team A', 200);

    // Check count increased
    assert(contract.get_user_pool_count(user) == 2, 'Count should be 2');

    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_get_user_pools() {
    let contract = deploy_predifi();

    let user = contract_address_const::<42>();
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
fn test_get_user_pools_by_status() {
    let contract = deploy_predifi();

    let user = contract_address_const::<42>();
    start_cheat_caller_address(contract.contract_address, user);

    // Create three pools
    let pool_id1 = create_default_pool(contract);
    let pool_id2 = create_default_pool(contract);
    let pool_id3 = create_default_pool(contract);

    // User participates in all pools
    contract.vote(pool_id1, 'Team A', 200);
    contract.vote(pool_id2, 'Team A', 200);
    contract.vote(pool_id3, 'Team A', 200);

    // All pools should be active by default
    let active_pools = contract.get_user_active_pools(user);
    assert(active_pools.len() == 3, 'Need 3 active');

    // No locked or settled pools yet
    let locked_pools = contract.get_user_locked_pools(user);
    assert(locked_pools.len() == 0, 'Need 0 locked');

    let settled_pools = contract.get_user_settled_pools(user);
    assert(settled_pools.len() == 0, 'Need 0 settled');

    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_stake_updates_participation() {
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);

    let user = contract_address_const::<42>();
    start_cheat_caller_address(contract.contract_address, user);

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
    let contract = deploy_predifi();
    let pool_id = create_default_pool(contract);

    let user = contract_address_const::<42>();
    start_cheat_caller_address(contract.contract_address, user);

    // User votes in the pool
    contract.vote(pool_id, 'Team A', 200);

    // Check participation count
    assert(contract.get_user_pool_count(user) == 1, 'Count should be 1');

    // User also stakes in the same pool
    let stake_amount: u256 = 200_000_000_000_000_000_000;
    contract.stake(pool_id, stake_amount);

    // Count should still be 1 as it's the same pool
    assert(contract.get_user_pool_count(user) == 1, 'Should still be 1');

    stop_cheat_caller_address(contract.contract_address);
}
