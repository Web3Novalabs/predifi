use contract::base::types::{Category, Pool, PoolDetails, Status};
use contract::interfaces::iUtils::{IUtilityDispatcher, IUtilityDispatcherTrait};
use contract::interfaces::ipredifi::{IPredifiDispatcher, IPredifiDispatcherTrait};
use contract::utils::Utils;
use contract::utils::Utils::InternalFunctionsTrait;
use core::array::ArrayTrait;
use core::felt252;
use core::serde::Serde;
use core::traits::{Into, TryInto};
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, declare, spy_events, start_cheat_block_timestamp,
    start_cheat_caller_address, stop_cheat_block_timestamp, stop_cheat_caller_address, test_address,
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

fn deploy_predifi() -> (IPredifiDispatcher, ContractAddress, ContractAddress) {
    let owner: ContractAddress = contract_address_const::<'owner'>();
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let validator: ContractAddress = contract_address_const::<'validator'>();

    // Deploy mock ERC20
    let erc20_class = declare("STARKTOKEN").unwrap().contract_class();
    let mut calldata = array![POOL_CREATOR.into(), owner.into(), 6];
    let (erc20_address, _) = erc20_class.deploy(@calldata).unwrap();

    let contract_class = declare("Predifi").unwrap().contract_class();

    let (contract_address, _) = contract_class
        .deploy(@array![erc20_address.into(), admin.into(), validator.into()])
        .unwrap();
    let dispatcher = IPredifiDispatcher { contract_address };
    (dispatcher, POOL_CREATOR, erc20_address)
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
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");
}

#[test]
#[should_panic(expected: "Start time must be before lock time")]
fn test_invalid_time_sequence_start_after_lock() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

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
#[should_panic(expected: "Minimum bet must be greater than 0")]
fn test_zero_min_bet() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

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
#[should_panic(expected: "Creator fee cannot exceed 5%")]
fn test_excessive_creator_fee() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

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
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);

    contract.vote(pool_id, 'Team A', 200);
    stop_cheat_caller_address(contract.contract_address);

    let pool = contract.get_pool(pool_id);
    assert(pool.totalBetCount == 1, 'Total bet count should be 1');
    assert(pool.totalStakeOption1 == 200, 'Total stake should be 200');
    assert(pool.totalSharesOption1 == 199, 'Total share should be 199');
}

#[test]
fn test_vote_with_user_stake() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);

    let pool = contract.get_pool(pool_id);
    contract.vote(pool_id, 'Team A', 200);
    stop_cheat_caller_address(contract.contract_address);

    let user_stake = contract.get_user_stake(pool_id, pool.address);
    assert(user_stake.amount == 200, 'Incorrect amount');
    assert(user_stake.shares == 199, 'Incorrect shares');
    assert(!user_stake.option, 'Incorrect option');
}

#[test]
fn test_successful_get_pool() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);
    let pool = contract.get_pool(pool_id);
    assert(pool.poolName == 'Example Pool', 'Pool not found');
}

#[test]
#[should_panic(expected: 'Invalid Pool Option')]
fn test_when_invalid_option_is_pass() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team C', 200);
}

#[test]
#[should_panic(expected: 'Amount is below minimum')]
fn test_when_min_bet_amount_less_than_required() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team A', 10);
}

#[test]
#[should_panic(expected: 'Amount is above maximum')]
fn test_when_max_bet_amount_greater_than_required() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team B', 1000000);
}

#[test]
fn test_get_pool_odds() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team A', 100);

    let pool_odds = contract.pool_odds(pool_id);
    assert(pool_odds.option1_odds == 2500, 'Incorrect odds for option 1');
    assert(pool_odds.option2_odds == 7500, 'Incorrect odds for option 2');
}

#[test]
fn test_get_pool_stakes() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team A', 200);

    let pool_stakes = contract.get_pool_stakes(pool_id);
    assert(pool_stakes.amount == 200, 'Incorrect pool stake amount');
    assert(pool_stakes.shares == 199, 'Incorrect pool stake shares');
    assert(!pool_stakes.option, 'Incorrect pool stake option');
}

#[test]
fn test_unique_pool_id() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");
}

#[test]
fn test_unique_pool_id_when_called_twice_in_the_same_execution() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);
    let pool_id1 = create_default_pool(contract);

    assert!(pool_id != 0, "not created");
    assert!(pool_id != pool_id1, "they are the same");
}

#[test]
fn test_get_pool_vote() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
    let pool_id = create_default_pool(contract);
    contract.vote(pool_id, 'Team A', 200);

    let pool_vote = contract.get_pool_vote(pool_id);
    assert(!pool_vote, 'Incorrect pool vote');
}

#[test]
fn test_get_pool_count() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);
    assert(contract.get_pool_count() == 0, 'Initial pool count should be 0');

    start_cheat_caller_address(contract.contract_address, voter);
    create_default_pool(contract);
    assert(contract.get_pool_count() == 1, 'Pool count should be 1');
}

#[test]
fn test_stake_successful() {
    let (contract, caller, erc20_address) = deploy_predifi();

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

    assert(contract.get_user_stake(pool_id, caller).amount == stake_amount, 'Invalid stake amount');
}

#[test]
fn test_get_pool_creator() {
    let (contract, POOL_CREATOR, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

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
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
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
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
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
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
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
    stop_cheat_caller_address(contract.contract_address);

    let creator_fee1 = contract.get_creator_fee_percentage(pool_id1);
    let creator_fee2 = contract.get_creator_fee_percentage(pool_id2);

    assert(creator_fee1 == 2, 'Pool 1 creator fee should be 2%');
    assert(creator_fee2 == 4, 'Pool 2 creator fee should be 4%');
}

#[test]
fn test_creator_and_validator_fee_for_same_pool() {
    let (contract, voter, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, voter);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, voter);
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
#[should_panic(expected: 'Insufficient STRK balance')]
fn test_insufficient_stark_balance() {
    let (dispatcher, _, erc20_address) = deploy_predifi();

    let test_addr: ContractAddress = contract_address_const::<'test'>();
    let erc20 = IERC20Dispatcher { contract_address: erc20_address };
    let balance = erc20.balance_of(test_addr);
    start_cheat_caller_address(erc20_address, test_addr);
    erc20.approve(dispatcher.contract_address, balance);
    stop_cheat_caller_address(erc20_address);

    dispatcher.collect_pool_creation_fee(test_addr);
}

#[test]
#[should_panic(expected: 'Insufficient allowance')]
fn test_insufficient_stark_allowance() {
    let (dispatcher, POOL_CREATOR, erc20_address) = deploy_predifi();

    let erc20 = IERC20Dispatcher { contract_address: erc20_address };

    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(dispatcher.contract_address, 1_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(dispatcher.contract_address, POOL_CREATOR);
    dispatcher.collect_pool_creation_fee(POOL_CREATOR);
}

#[test]
fn test_collect_creation_fee() {
    let (dispatcher, POOL_CREATOR, erc20_address) = deploy_predifi();

    let erc20 = IERC20Dispatcher { contract_address: erc20_address };

    let initial_contract_balance = erc20.balance_of(dispatcher.contract_address);
    assert(initial_contract_balance == 0, 'incorrect deployment details');

    let balance = erc20.balance_of(POOL_CREATOR);
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(dispatcher.contract_address, balance);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(dispatcher.contract_address, POOL_CREATOR);
    dispatcher.collect_pool_creation_fee(POOL_CREATOR);
    let user_balance_after = erc20.balance_of(POOL_CREATOR);
    assert(user_balance_after == balance - ONE_STRK, 'deduction failed');

    let contract_balance_after_collection = erc20.balance_of(dispatcher.contract_address);
    assert(contract_balance_after_collection == ONE_STRK, 'fee collection failed');
}


#[test]
fn test_collect_validation_fee() {
    let (dispatcher, STAKER, erc20_address) = deploy_predifi();

    let validation_fee = dispatcher.calculate_validator_fee(54, 10_000);
    assert(validation_fee == 500, 'invalid calculation');
}

#[test]
fn test_distribute_validation_fee() {
    let (mut dispatcher, POOL_CREATOR, erc20_address) = deploy_predifi();
    let validator1 = contract_address_const::<'validator1'>();
    let validator2 = contract_address_const::<'validator2'>();
    let validator3 = contract_address_const::<'validator3'>();
    let validator4 = contract_address_const::<'validator4'>();
    let zero_address: ContractAddress = contract_address_const::<'zero'>();

    let erc20 = IERC20Dispatcher { contract_address: erc20_address };
    let validators = dispatcher.add_validators(validator1, validator2, validator3, validator4);

    let initial_contract_balance = erc20.balance_of(dispatcher.contract_address);
    assert(initial_contract_balance == 0, 'incorrect deployment details');

    let balance = erc20.balance_of(POOL_CREATOR);
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(dispatcher.contract_address, balance);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(dispatcher.contract_address, POOL_CREATOR);
    dispatcher.collect_pool_creation_fee(POOL_CREATOR);

    dispatcher.calculate_validator_fee(18, 10_000);

    start_cheat_caller_address(dispatcher.contract_address, dispatcher.contract_address);
    dispatcher.distribute_validator_fees(18);

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
fn test_automatic_pool_state_transitions() {
    let (contract, admin, erc20_address) = deploy_predifi();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, admin);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, admin);
    // Create a pool with specific timestamps
    let active_pool_id = contract
        .create_pool(
            'Active Pool',
            Pool::WinBet,
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
            Category::Sports,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Verify initial state
    let pool = contract.get_pool(active_pool_id);
    assert(pool.status == Status::Active, 'Initial state should be Active');

    // Test no change when time hasn't reached lock time
    start_cheat_block_timestamp(contract.contract_address, current_time + 1500);
    let same_state = contract.update_pool_state(active_pool_id);
    assert(same_state == Status::Active, 'State should remain Active');

    // Check pool state is still Active
    let pool_after_check = contract.get_pool(active_pool_id);
    assert(pool_after_check.status == Status::Active, 'Status should not change');

    // Test transition: Active -> Locked
    // Set block timestamp to just after lock time
    start_cheat_block_timestamp(contract.contract_address, current_time + 2001);
    let new_state = contract.update_pool_state(active_pool_id);
    assert(new_state == Status::Locked, 'State should be Locked');

    // Verify state was actually updated in storage
    let locked_pool = contract.get_pool(active_pool_id);
    assert(locked_pool.status == Status::Locked, 'should be Locked in storage');

    // Try updating again - should stay in Locked state
    let same_locked_state = contract.update_pool_state(active_pool_id);
    assert(same_locked_state == Status::Locked, 'Should remain Locked');

    // Test transition: Locked -> Settled
    // Set block timestamp to just after end time
    start_cheat_block_timestamp(contract.contract_address, current_time + 3001);
    let new_state = contract.update_pool_state(active_pool_id);
    assert(new_state == Status::Settled, 'State should be Settled');

    // Verify state was updated in storage
    let settled_pool = contract.get_pool(active_pool_id);
    assert(settled_pool.status == Status::Settled, 'should be Settled in storage');

    // Test transition: Settled -> Closed
    // Set block timestamp to 24 hours + 1 second after end time
    start_cheat_block_timestamp(contract.contract_address, current_time + 3000 + 86401);
    let final_state = contract.update_pool_state(active_pool_id);
    assert(final_state == Status::Closed, 'State should be Closed');

    // Verify state was updated in storage
    let closed_pool = contract.get_pool(active_pool_id);
    assert(closed_pool.status == Status::Closed, 'should be Closed in storage');

    // Test that no further transitions occur once Closed
    // Set block timestamp to much later
    start_cheat_block_timestamp(contract.contract_address, current_time + 10000);
    let final_state = contract.update_pool_state(active_pool_id);
    assert(final_state == Status::Closed, 'Should remain Closed');

    // Reset block timestamp cheat
    stop_cheat_block_timestamp(contract.contract_address);
}

#[test]
#[should_panic(expected: 'Pool does not exist')]
fn test_nonexistent_pool_state_update() {
    let (contract, _, _) = deploy_predifi();

    // Attempt to update a pool that doesn't exist - should panic
    contract.update_pool_state(999);
}

#[test]
fn test_manual_pool_state_update() {
    let (contract, user, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval for user
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, user);
    // Create a pool with specific timestamps
    let pool_id = contract
        .create_pool(
            'Test Pool',
            Pool::WinBet,
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
            Category::Sports,
        );

    // Verify initial state
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Active, 'Initial state should be Active');

    // Manually update to Locked state
    start_cheat_caller_address(contract.contract_address, admin);
    let locked_state = contract.manually_update_pool_state(pool_id, Status::Locked);
    stop_cheat_caller_address(contract.contract_address);

    assert(locked_state == Status::Locked, 'State should be Locked');

    // Verify state change in storage
    let locked_pool = contract.get_pool(pool_id);
    assert(locked_pool.status == Status::Locked, 'should be Locked in storage');

    // Update to Settled state
    start_cheat_caller_address(contract.contract_address, admin);
    let settled_state = contract.manually_update_pool_state(pool_id, Status::Settled);
    stop_cheat_caller_address(contract.contract_address);

    assert(settled_state == Status::Settled, 'State should be Settled');

    // Verify state change in storage
    let settled_pool = contract.get_pool(pool_id);
    assert(settled_pool.status == Status::Settled, 'should be Settled in storage');

    // Update to Closed state
    start_cheat_caller_address(contract.contract_address, admin);
    let closed_state = contract.manually_update_pool_state(pool_id, Status::Closed);
    stop_cheat_caller_address(contract.contract_address);

    assert(closed_state == Status::Closed, 'State should be Closed');

    // Verify final state in storage
    let final_pool = contract.get_pool(pool_id);
    assert(final_pool.status == Status::Closed, 'should be Closed in storage');
}

#[test]
#[should_panic(expected: 'Caller not authorized')]
fn test_unauthorized_manual_update() {
    let (contract, admin, erc20_address) = deploy_predifi();

    // Random unauthorized address
    let unauthorized = contract_address_const::<'unauthorized'>();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval for user
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, admin);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create pool as admin
    start_cheat_caller_address(contract.contract_address, admin);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            Pool::WinBet,
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
            Category::Sports,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Attempt unauthorized update - should panic with 'Caller not authorized'
    start_cheat_caller_address(contract.contract_address, unauthorized);
    contract.manually_update_pool_state(pool_id, Status::Locked); // This should panic
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
#[should_panic(expected: 'Invalid state transition')]
fn test_invalid_state_transition() {
    let (contract, user, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval for user
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create pool as user
    start_cheat_caller_address(contract.contract_address, user);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            Pool::WinBet,
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
            Category::Sports,
        );

    start_cheat_caller_address(contract.contract_address, admin);
    // Update to Locked
    contract.manually_update_pool_state(pool_id, Status::Locked);

    // Try to revert back to Active - should fail with 'Invalid state transition'
    contract.manually_update_pool_state(pool_id, Status::Active);
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_no_change_on_same_state() {
    let (contract, user, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval for user
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create pool as user
    start_cheat_caller_address(contract.contract_address, user);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            Pool::WinBet,
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
            Category::Sports,
        );

    start_cheat_caller_address(contract.contract_address, admin);
    // Try to update to the same state (Active)
    let same_state = contract.manually_update_pool_state(pool_id, Status::Active);
    stop_cheat_caller_address(contract.contract_address);

    assert(same_state == Status::Active, 'Should return same state');

    // Verify state remains unchanged
    let unchanged_pool = contract.get_pool(pool_id);
    assert(unchanged_pool.status == Status::Active, 'State should not change');
}

#[test]
#[should_panic(expected: 'Pool does not exist')]
fn test_manual_update_nonexistent_pool() {
    let (contract, admin, _) = deploy_predifi();

    // Try to update a nonexistent pool
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(999, Status::Locked); // This should panic
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_validator_can_update_state() {
    let (mut contract, admin, erc20_address) = deploy_predifi();

    // Create a validator
    let validator = contract_address_const::<'validator'>();

    // Add token approval for admin
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, admin);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Add validators
    let validator_array = contract
        .add_validators(
            validator,
            contract_address_const::<'v2'>(),
            contract_address_const::<'v3'>(),
            contract_address_const::<'v4'>(),
        );

    // Get current time
    let current_time = get_block_timestamp();

    // Create a pool using admin
    start_cheat_caller_address(contract.contract_address, admin);
    let pool_id = contract
        .create_pool(
            'Validator Test Pool',
            Pool::WinBet,
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
            Category::Sports,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Validator updates state
    start_cheat_caller_address(contract.contract_address, validator);
    let updated_state = contract.manually_update_pool_state(pool_id, Status::Locked);
    stop_cheat_caller_address(contract.contract_address);

    assert(updated_state == Status::Locked, 'Validator update should succeed');

    // Verify state change
    let updated_pool = contract.get_pool(pool_id);
    assert(updated_pool.status == Status::Locked, 'should be updated by validator');
}

#[test]
fn test_assign_random_validators() {
    // Deploy the contract
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Create validators
    let validator1 = contract_address_const::<'validator1'>();
    let validator2 = contract_address_const::<'validator2'>();
    let validator3 = contract_address_const::<'validator3'>();
    let validator4 = contract_address_const::<'validator4'>();
    let zero_address: ContractAddress = contract_address_const::<'zero'>();

    // Add validators to the contract
    let _validators = contract.add_validators(validator1, validator2, validator3, validator4);

    // Set up token approval for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create a pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Assign random validators to the pool
    contract.assign_random_validators(pool_id);

    // Get the assigned validators
    let (assigned_validator1, assigned_validator2) = contract.get_pool_validators(pool_id);

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

    assert(is_valid_validator1, 'Validator1');
    assert(is_valid_validator2, 'Validator2');
}


#[test]
fn test_assign_random_validators_initial_validator() {
    // Deploy the contract
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Get the validator that was added during deployment
    let expected_validator = contract_address_const::<'validator'>();

    // Explicitly add the validator to the validators list
    contract
        .add_validators(
            expected_validator, expected_validator, expected_validator, expected_validator,
        );

    // Set up token approval for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create a pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Assign random validators to the pool
    contract.assign_random_validators(pool_id);

    // Get the assigned validators
    let (assigned_validator1, assigned_validator2) = contract.get_pool_validators(pool_id);

    // Verify that both assigned validators are the expected validator
    assert(assigned_validator1 == expected_validator, 'Should assign initial validator');
    assert(assigned_validator2 == expected_validator, 'Should assign initial validator');
}

#[test]
fn test_assign_multiple_validators() {
    // Deploy the contract
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Create multiple validators with different addresses
    let validator1 = contract_address_const::<'validator1'>();
    let validator2 = contract_address_const::<'validator2'>();
    let validator3 = contract_address_const::<'validator3'>();
    let validator4 = contract_address_const::<'validator4'>();

    // Add validators to the contract
    contract.add_validators(validator1, validator2, validator3, validator4);

    // Set up token approval for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create a pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Assign random validators to the pool
    contract.assign_random_validators(pool_id);

    // Get the assigned validators
    let (assigned_validator1, assigned_validator2) = contract.get_pool_validators(pool_id);

    // Verify that validators were assigned
    assert(assigned_validator1 != contract_address_const::<0>(), 'Validator1 not assigned');
    assert(assigned_validator2 != contract_address_const::<0>(), 'Validator2 not assigned');

    // Verify that different validators were assigned
    assert(assigned_validator1 != assigned_validator2, 'Same validator assigned twice');

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

    // Create a second pool and verify different validators can be assigned
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id2 = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Assign random validators to the second pool
    contract.assign_random_validators(pool_id2);

    // Get the assigned validators for the second pool
    let (assigned_validator1_pool2, assigned_validator2_pool2) = contract
        .get_pool_validators(pool_id2);

    // Verify that validators were assigned to the second pool
    assert(
        assigned_validator1_pool2 != contract_address_const::<0>(), 'Pool2 validator1 not assigned',
    );
    assert(
        assigned_validator2_pool2 != contract_address_const::<0>(), 'Pool2 validator2 not assigned',
    );
}

#[test]
fn test_validator_distribution() {
    // Deploy the contract
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Create multiple validators with different addresses
    let validator1 = contract_address_const::<'validator1'>();
    let validator2 = contract_address_const::<'validator2'>();
    let validator3 = contract_address_const::<'validator3'>();
    let validator4 = contract_address_const::<'validator4'>();

    // Add validators to the contract
    contract.add_validators(validator1, validator2, validator3, validator4);

    // Set up token approval for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 1_000_000_000_000_000_000_000_000);
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
                Pool::WinBet, // poolType
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
                Category::Sports // category
            );
        pool_ids.append(pool_id);
        i += 1;
    }
    stop_cheat_caller_address(contract.contract_address);

    // Assign validators to each pool
    let mut i: u32 = 0;
    while i < pool_ids.len() {
        let pool_id = *pool_ids.at(i);
        contract.assign_random_validators(pool_id);
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
        let (assigned_validator1, assigned_validator2) = contract.get_pool_validators(pool_id);

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

