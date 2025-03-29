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

// Constant for the pool creator's contract address
const POOL_CREATOR: ContractAddress = 0x123.try_into().expect("Invalid address");

#[starknet::interface]
trait IMockAccessControl<TContractState> {
    fn has_role(self: @TContractState, role: felt252, user: ContractAddress) -> bool;
}

fn owner() -> ContractAddress {
    'owner'.try_into().unwrap()
}

fn deploy_predifi() -> IPredifiDispatcher {
    let contract_class = declare("Predifi").unwrap().contract_class();
    let (contract_address, _) = contract_class.deploy(@array![].into()).unwrap();
    (IPredifiDispatcher { contract_address })
}

const ONE_STRK: u256 = 1_000_000_000_000_000_000;

// MODULARIZED POOL CREATION FUNCTION
fn create_test_pool(
    contract: IPredifiDispatcher,
    pool_name: felt252 = 'Default Pool',
    pool_type: Pool = Pool::WinBet,
    pool_description: ByteArray = "Default Description",
    pool_image: ByteArray = "default_image.jpg",
    pool_event_source_url: ByteArray = "https://example.com",
    pool_start_time: u64 = get_block_timestamp() + 86400,
    pool_lock_time: u64 = get_block_timestamp() + 172800,
    pool_end_time: u64 = get_block_timestamp() + 259200,
    option1: felt252 = 'Option A',
    option2: felt252 = 'Option B',
    min_bet_amount: u256 = ONE_STRK,
    max_bet_amount: u256 = 10 * ONE_STRK,
    creator_fee: u8 = 5,
    is_private: bool = false,
    category: Category = Category::Sports,
) -> u256 {
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = contract.create_pool(
        pool_name,
        pool_type,
        pool_description,
        pool_image,
        pool_event_source_url,
        pool_start_time,
        pool_lock_time,
        pool_end_time,
        option1,
        option2,
        min_bet_amount,
        max_bet_amount,
        creator_fee,
        is_private,
        category,
    );
    stop_cheat_caller_address(contract.contract_address);
    assert!(pool_id != 0, "Pool creation failed");
    pool_id
}

// REFACTORED TESTS USING NAMED ARGUMENTS
#[test]
fn test_create_pool() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    assert!(pool_id != 0, "not created");
}

#[test]
#[should_panic(expected: "Start time must be before lock time")]
fn test_invalid_time_sequence_start_after_lock() {
    let contract = deploy_predifi();
    let current_time = get_block_timestamp();
    create_test_pool(
        contract,
        pool_start_time: current_time + 3600,
        pool_lock_time: current_time + 1800,
    );
}

#[test]
#[should_panic(expected: "Minimum bet must be greater than 0")]
fn test_zero_min_bet() {
    let contract = deploy_predifi();
    create_test_pool(contract, min_bet_amount: 0);
}

#[test]
#[should_panic(expected: "Creator fee cannot exceed 5%")]
fn test_excessive_creator_fee() {
    let contract = deploy_predifi();
    create_test_pool(contract, creator_fee: 6);
}

#[test]
fn test_vote() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    contract.vote(pool_id, 'Team A', 200);
    let pool = contract.get_pool(pool_id);
    assert(pool.totalBetCount == 1, 'Total bet count should be 1');
    assert(pool.totalStakeOption1 == 200, 'Total stake should be 200');
    assert(pool.totalSharesOption1 == 199, 'Total share should be 199');
}

#[test]
fn test_vote_with_user_stake() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
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
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool1');
    let pool = contract.get_pool(pool_id);
    assert(pool.poolName == 'Example Pool1', 'Pool not found');
}

#[test]
#[should_panic(expected: 'Invalid Pool Option')]
fn test_when_invalid_option_is_pass() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    contract.vote(pool_id, 'Team C', 200);
}

#[test]
#[should_panic(expected: 'Amount is below minimum')]
fn test_when_min_bet_amount_less_than_required() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    contract.vote(pool_id, 'Team A', 10);
}

#[test]
#[should_panic(expected: 'Amount is above maximum')]
fn test_when_max_bet_amount_greater_than_required() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    contract.vote(pool_id, 'Team B', 1000000);
}

#[test]
fn test_get_pool_odds() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    contract.vote(pool_id, 'Team A', 100);
    let pool_odds = contract.pool_odds(pool_id);
    assert(pool_odds.option1_odds == 2500, 'Incorrect odds for option 1');
    assert(pool_odds.option2_odds == 7500, 'Incorrect odds for option 2');
}

#[test]
fn test_get_pool_stakes() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    contract.vote(pool_id, 'Team A', 200);
    let pool_stakes = contract.get_pool_stakes(pool_id);
    assert(pool_stakes.amount == 200, 'Incorrect pool stake amount');
    assert(pool_stakes.shares == 199, 'Incorrect pool stake shares');
    assert(!pool_stakes.option, 'Incorrect pool stake option');
}

#[test]
fn test_unique_pool_id() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    assert!(pool_id != 0, "not created");
    println!("Pool id: {}", pool_id);
}

#[test]
fn test_unique_pool_id_when_called_twice_in_the_same_execution() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    let pool_id1 = create_test_pool(contract, pool_name: 'Example Pool');
    assert!(pool_id != 0, "not created");
    assert!(pool_id != pool_id1, "they are the same");
    println!("Pool id: {}", pool_id);
    println!("Pool id: {}", pool_id1);
}

#[test]
fn test_get_pool_vote() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    contract.vote(pool_id, 'Team A', 200);
    let pool_vote = contract.get_pool_vote(pool_id);
    assert(!pool_vote, 'Incorrect pool vote');
}

#[test]
fn test_get_pool_count() {
    let contract = deploy_predifi();
    assert(contract.get_pool_count() == 0, 'Initial pool count should be 0');
    create_test_pool(contract, pool_name: 'Example Pool');
    assert(contract.get_pool_count() == 1, 'Pool count should be 1');
}

#[test]
fn test_stake_successful() {
    let contract = deploy_predifi();
    let caller = contract_address_const::<1>();
    let stake_amount: u256 = 200_000_000_000_000_000_000;
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    start_cheat_caller_address(contract.contract_address, caller);
    contract.stake(pool_id, stake_amount);
    stop_cheat_caller_address(contract.contract_address);
    assert(contract.get_user_stake(pool_id, caller).amount == stake_amount, 'Invalid stake amount');
    let access_control_dispatcher = IMockAccessControlDispatcher {
        contract_address: contract.contract_address,
    };
    assert(access_control_dispatcher.has_role(VALIDATOR_ROLE, caller), 'No role found');
}

#[test]
#[should_panic]
fn test_stake_unsuccessful_when_lower_than_min_amount() {
    let contract = deploy_predifi();
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    let caller = contract_address_const::<1>();
    let stake_amount: u256 = 10_000_000_000_000_000_000;
    start_cheat_caller_address(contract.contract_address, caller);
    contract.stake(pool_id, stake_amount); // should panic
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_get_pool_creator() {
    let contract = deploy_predifi();
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_test_pool(contract, pool_name: 'Example Pool');
    stop_cheat_caller_address(contract.contract_address);
    assert!(pool_id != 0, "not created");
    assert!(contract.get_pool_creator(pool_id) == POOL_CREATOR, "incorrect creator");
}

// ------ Utility Contract tests --------
fn deploy_utils() -> (IUtilityDispatcher, ContractAddress) {
    let utils_contract_class = declare("Utils")
        .unwrap()
        .contract_class();
    let owner: ContractAddress = get_caller_address();
    let pragma_address: ContractAddress =
        0x036031daa264c24520b11d93af622c848b2499b66b41d611bac95e13cfca131a
        .try_into()
        .unwrap();
    let mut constructor_calldata = array![];
    Serde::serialize(@owner, ref constructor_calldata);
    Serde::serialize(@pragma_address, ref constructor_calldata);
    let (utils_contract, _) = utils_contract_class
        .deploy(@constructor_calldata)
        .unwrap();
    let utils_dispatcher = IUtilityDispatcher { contract_address: utils_contract };
    return (utils_dispatcher, utils_contract);
}

#[test]
fn test_get_utils_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner);
    let retrieved_owner = state.get_owner();
    assert_eq!(retrieved_owner, owner);
}

#[test]
fn test_set_utils_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner);
    let initial_owner = state.owner.read();
    let new_owner: ContractAddress = contract_address_const::<'new_owner'>();
    let test_address: ContractAddress = test_address();
    start_cheat_caller_address(test_address, initial_owner);
    state.set_owner(new_owner);
    let retrieved_owner = state.owner.read();
    assert_eq!(retrieved_owner, new_owner);
}

#[test]
#[should_panic(expected: "Only the owner can set ownership")]
fn test_set_utils_wrong_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner);
    let new_owner: ContractAddress = contract_address_const::<'new_owner'>();
    let another_owner: ContractAddress = contract_address_const::<'another_owner'>();
    let test_address: ContractAddress = test_address();
    start_cheat_caller_address(test_address, another_owner);
    state.set_owner(new_owner); // expect to panic
}

#[test]
#[should_panic(expected: "Cannot change ownership to 0x0")]
fn test_set_utils_zero_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner);
    let initial_owner = state.owner.read();
    let zero_owner: ContractAddress = 0x0.try_into().unwrap(); // 0x0 address
    let test_address: ContractAddress = test_address();
    start_cheat_caller_address(test_address, initial_owner);
    state.set_owner(zero_owner); // expect to panic
}

#[test]
fn test_get_pragma_contract() {
    let mut state = Utils::contract_state_for_testing();
    let pragma: ContractAddress = contract_address_const::<'PRAGMA'>();
    state.pragma_contract.write(pragma);
    let retrieved_addr = state.get_pragma_contract_address();
    assert_eq!(retrieved_addr, pragma);
}

#[test]
fn test_set_pragma_contract() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner);
    let initial_owner = state.owner.read();
    let pragma: ContractAddress = contract_address_const::<'PRAGMA'>();
    state.pragma_contract.write(pragma);
    let test_address: ContractAddress = test_address();
    let new_pragma: ContractAddress = contract_address_const::<'NEW_PRAGMA'>();
    start_cheat_caller_address(test_address, initial_owner);
    state.set_pragma_contract_address(new_pragma);
    let retrieved_addr = state.pragma_contract.read();
    assert_eq!(retrieved_addr, new_pragma);
}

#[test]
#[should_panic(expected: "Only the owner can change contract address")]
fn test_set_pragma_contract_wrong_owner() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner);
    let initial_owner = state.owner.read();
    let pragma: ContractAddress = contract_address_const::<'PRAGMA'>();
    state.pragma_contract.write(pragma);
    let another_owner: ContractAddress = contract_address_const::<'another_owner'>();
    let test_address: ContractAddress = test_address();
    let new_pragma: ContractAddress = contract_address_const::<'NEW_PRAGMA'>();
    start_cheat_caller_address(test_address, another_owner);
    state.set_pragma_contract_address(new_pragma); // expect to panic
}

#[test]
#[should_panic(expected: "Cannot change contract address to 0x0")]
fn test_set_pragma_contract_zero_addr() {
    let mut state = Utils::contract_state_for_testing();
    let owner: ContractAddress = contract_address_const::<'owner'>();
    state.owner.write(owner);
    let initial_owner = state.owner.read();
    let pragma: ContractAddress = contract_address_const::<'PRAGMA'>();
    state.pragma_contract.write(pragma);
    let zero_addr: ContractAddress = 0x0.try_into().unwrap(); // 0x0 address
    let test_address: ContractAddress = test_address();
    start_cheat_caller_address(test_address, initial_owner);
    state.set_pragma_contract_address(zero_addr); // expect to panic
}

#[test]
#[fork("SEPOLIA_LATEST")]
fn test_get_strk_usd_price() {
    let (utils_dispatcher, _) = deploy_utils();
    let strk_in_usd = utils_dispatcher.get_strk_usd_price(); // accessing pragma price feeds
    assert!(strk_in_usd > 0, "Price should be greater than 0");
}