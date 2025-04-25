use contract::base::types::{Category, Pool, PoolDetails, Status};
use contract::interfaces::iUtils::{IUtilityDispatcher, IUtilityDispatcherTrait};
use contract::interfaces::ipredifi::{IPredifiDispatcher, IPredifiDispatcherTrait};
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
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

fn deploy_predifi() -> (IPredifiDispatcher, ContractAddress, ContractAddress) {
    let owner: ContractAddress = contract_address_const::<'owner'>();
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let validator: ContractAddress = contract_address_const::<'validator'>();



    // Deploy mock ERC20
    let erc20_class = declare("STARKTOKEN").unwrap().contract_class();
    let mut calldata = array![POOL_CREATOR.into(), owner.into(), 6];
    let (erc20_address, _) = erc20_class.deploy(@calldata).unwrap();


    let contract_class = declare("Predifi").unwrap().contract_class();

    let (contract_address, _) = contract_class.deploy(@array![erc20_address.into(), admin.into(), validator.into()]).unwrap();
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
/// testing if pragma price feed is accessible and returning values
// #[test]
// #[fork("SEPOLIA_LATEST")]
// fn test_get_strk_usd_price() {
//     let (utils_dispatcher, _) = deploy_utils();
//     let strk_in_usd = utils_dispatcher.get_strk_usd_price(); // accessing pragma price feeds
//     assert!(strk_in_usd > 0, "Price should be greater than 0");
// }


