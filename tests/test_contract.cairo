use contract::base::events::Events::{
    BetPlaced, DisputeRaised, DisputeResolved, FeeWithdrawn, FeesCollected,
    PoolAutomaticallySettled, PoolCancelled, PoolResolved, PoolStateTransition, PoolSuspended,
    StakeRefunded, UserStaked, ValidatorAdded, ValidatorRemoved, ValidatorResultSubmitted,
    ValidatorsAssigned,
};
use contract::base::types::{Category, Pool, PoolDetails, Status};
use contract::interfaces::iUtils::{IUtilityDispatcher, IUtilityDispatcherTrait};
use contract::interfaces::ipredifi::{
    IPredifi, IPredifiDispatcher, IPredifiDispatcherTrait, IPredifiSafeDispatcher,
    IPredifiSafeDispatcherTrait,
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
use starknet::{
    ClassHash, ContractAddress, contract_address_const, get_block_timestamp, get_caller_address,
    get_contract_address,
};


// Validator role
const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");
// Pool creator address constant
const POOL_CREATOR: ContractAddress = 123.try_into().unwrap();

const USER_ONE: ContractAddress = 'User1'.try_into().unwrap();

fn deploy_predifi() -> (IPredifiDispatcher, ContractAddress, ContractAddress) {
    let owner: ContractAddress = contract_address_const::<'owner'>();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    // Deploy mock ERC20
    let erc20_class = declare("STARKTOKEN").unwrap().contract_class();
    let mut calldata = array![POOL_CREATOR.into(), owner.into(), 6];
    let (erc20_address, _) = erc20_class.deploy(@calldata).unwrap();

    let contract_class = declare("Predifi").unwrap().contract_class();

    let (contract_address, _) = contract_class
        .deploy(@array![erc20_address.into(), admin.into()])
        .unwrap();
    let dispatcher = IPredifiDispatcher { contract_address };
    (dispatcher, POOL_CREATOR, erc20_address)
}

// Helper function for creating pools with default parameters
fn create_default_pool(contract: IPredifiDispatcher) -> u256 {
    contract
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
            Category::Sports,
        )
}

// Helper function to declare Contract Class and return the Class Hash
fn declare_contract(name: ByteArray) -> ClassHash {
    let declare_result = declare(name);
    let declared_contract = declare_result.unwrap().contract_class();
    *declared_contract.class_hash
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
fn test_cancel_pool() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");

    contract.cancel_pool(pool_id);

    let fetched_pool = contract.get_pool(pool_id);

    assert(fetched_pool.status == Status::Closed, 'Pool not closed');
}

#[test]
fn test_cancel_pool_event_emission() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();
    let mut spy = spy_events();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");

    contract.cancel_pool(pool_id);

    let fetched_pool = contract.get_pool(pool_id);

    assert(fetched_pool.status == Status::Closed, 'Pool not closed');

    let expected_event = Predifi::Event::PoolCancelled(
        PoolCancelled { pool_id, timestamp: get_block_timestamp() },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_event)]);
}

#[test]
#[should_panic(expected: 'Unauthorized Caller')]
fn test_cancel_pool_by_unauthorized_caller() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "not created");

    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.cancel_pool(pool_id);
}

#[test]
#[should_panic(expected: 'Invalid lock time')]
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
#[should_panic(expected: 'Minimum bet cannot be zero')]
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
#[should_panic(expected: 'Creator fee cannot exceed 5%')]
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

#[test]
#[should_panic(expected: "Invalid pool type: must be 0-3")]
fn test_invalid_pool_type() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    let (
        poolName,
        _,
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
    ) =
        get_default_pool_params();

    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract
        .create_pool(
            poolName,
            99, // Invalid pool type
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
}

#[test]
fn test_valid_pool_types() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    let (
        poolName,
        _,
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
    ) =
        get_default_pool_params();

    start_cheat_caller_address(contract.contract_address, pool_creator);

    // Test all valid pool types (0-3)
    let pool_id_0 = contract
        .create_pool(
            'WinBet Pool',
            0, // WinBet
            poolDescription.clone(),
            poolImage.clone(),
            poolEventSourceUrl.clone(),
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

    let pool_id_1 = contract
        .create_pool(
            'VoteBet Pool',
            1, // VoteBet
            poolDescription.clone(),
            poolImage.clone(),
            poolEventSourceUrl.clone(),
            poolStartTime + 1,
            poolLockTime + 1,
            poolEndTime + 1,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );

    let pool_id_2 = contract
        .create_pool(
            'OverUnderBet Pool',
            2, // OverUnderBet
            poolDescription.clone(),
            poolImage.clone(),
            poolEventSourceUrl.clone(),
            poolStartTime + 2,
            poolLockTime + 2,
            poolEndTime + 2,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );

    let pool_id_3 = contract
        .create_pool(
            'ParlayPool Pool',
            3, // ParlayPool
            poolDescription,
            poolImage,
            poolEventSourceUrl,
            poolStartTime + 3,
            poolLockTime + 3,
            poolEndTime + 3,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        );

    // Verify all pools were created successfully
    assert!(pool_id_0 != 0, "WinBet pool not created");
    assert!(pool_id_1 != 0, "VoteBet pool not created");
    assert!(pool_id_2 != 0, "OverUnderBet pool not created");
    assert!(pool_id_3 != 0, "ParlayPool pool not created");

    // Verify the pool types are correctly stored
    let pool_0 = contract.get_pool(pool_id_0);
    let pool_1 = contract.get_pool(pool_id_1);
    let pool_2 = contract.get_pool(pool_id_2);
    let pool_3 = contract.get_pool(pool_id_3);

    assert(pool_0.poolType == Pool::WinBet, 'Wrong type for pool 0');
    assert(pool_1.poolType == Pool::VoteBet, 'Wrong type for pool 1');
    assert(pool_2.poolType == Pool::OverUnderBet, 'Wrong type for pool 2');
    assert(pool_3.poolType == Pool::ParlayPool, 'Wrong type for pool 3');
}

fn get_default_pool_params() -> (
    felt252,
    u8,
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
        0, // 0 = WinBet
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
    let admin: ContractAddress = contract_address_const::<'admin'>();

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

    let retrieved_owner = state
        .get_owner(); // retrieving the owner's address from contract storage
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
            Category::Sports,
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
#[should_panic(expected: 'Insufficient balance')]
fn test_insufficient_stark_balance() {
    let (dispatcher, _, erc20_address) = deploy_predifi();

    let test_addr: ContractAddress = contract_address_const::<'test'>();
    let erc20 = IERC20Dispatcher { contract_address: erc20_address };
    let balance = erc20.balance_of(test_addr);
    start_cheat_caller_address(erc20_address, test_addr);
    erc20.approve(dispatcher.contract_address, balance);
    stop_cheat_caller_address(erc20_address);

    // Test insufficient balance by trying to create a pool with insufficient funds
    start_cheat_caller_address(dispatcher.contract_address, test_addr);
    create_default_pool(dispatcher);
}

#[test]
#[should_panic(expected: 'Insufficient allowance')]
fn test_insufficient_stark_allowance() {
    let (dispatcher, POOL_CREATOR, erc20_address) = deploy_predifi();

    let erc20 = IERC20Dispatcher { contract_address: erc20_address };

    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(dispatcher.contract_address, 1_000_000);
    stop_cheat_caller_address(erc20_address);

    // Test insufficient allowance by trying to create a pool
    start_cheat_caller_address(dispatcher.contract_address, POOL_CREATOR);
    create_default_pool(dispatcher);
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

    // Test that pool creation collects the fee automatically
    start_cheat_caller_address(dispatcher.contract_address, POOL_CREATOR);
    create_default_pool(dispatcher);

    let user_balance_after = erc20.balance_of(POOL_CREATOR);
    assert(user_balance_after == balance - ONE_STRK, 'deduction failed');

    let contract_balance_after_collection = erc20.balance_of(dispatcher.contract_address);
    assert(contract_balance_after_collection == ONE_STRK, 'fee collection failed');
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
            0, // 0 = WinBet
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
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);
    start_cheat_caller_address(contract.contract_address, user);
    // Create a pool with specific timestamps
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
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
#[should_panic(expected: 'Unauthorized Caller')]
fn test_unauthorized_manual_update() {
    let (contract, admin, erc20_address) = deploy_predifi();

    // Random unauthorized address
    let unauthorized = contract_address_const::<'unauthorized'>();

    // Get current time
    let current_time = get_block_timestamp();

    // Add token approval for admin
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, admin);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
    let admin_role = contract_address_const::<'admin'>();
    start_cheat_caller_address(contract.contract_address, admin_role);
    contract.add_validator(validator);
    stop_cheat_caller_address(contract.contract_address);

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
fn test_track_user_participation() {
    // Deploy contracts
    let (contract, user1, erc20_address) = deploy_predifi();

    // Approve token spending for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user1);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
    let (contract, user, erc20_address) = deploy_predifi();

    // Approve token spending for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
    let (contract, user, erc20_address) = deploy_predifi();

    // Approve token spending for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
    let (contract, user1, erc20_address) = deploy_predifi();

    // Approve token spending for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, user1);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
    let (contract, admin, erc20_address) = deploy_predifi();

    // Create two additional users
    let user1 = contract_address_const::<1>();
    let user2 = contract_address_const::<2>();
    let admi: ContractAddress = contract_address_const::<'admin'>();

    // Approve token spending for all users
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Mint some tokens for the users
    start_cheat_caller_address(erc20_address, admin);
    erc20.transfer(user1, 1000_000_000_000_000_000_000);
    erc20.transfer(user2, 1000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Approve for admin
    start_cheat_caller_address(erc20_address, admin);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Approve for user1
    start_cheat_caller_address(erc20_address, user1);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Approve for user2
    start_cheat_caller_address(erc20_address, user2);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
    contract.manually_update_pool_state(pool_id2, Status::Locked);
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
    let (contract, user, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    // Approve token spending for pool creation and betting
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Approve the contract to spend tokens
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
    contract.manually_update_pool_state(pool_id2, Status::Locked);

    // Transition pool 3 to Locked and then to Settled
    contract.manually_update_pool_state(pool_id3, Status::Locked);
    contract.manually_update_pool_state(pool_id3, Status::Settled);

    // Transition pool 4 through all states to Closed
    contract.manually_update_pool_state(pool_id4, Status::Locked);
    contract.manually_update_pool_state(pool_id4, Status::Settled);
    contract.manually_update_pool_state(pool_id4, Status::Closed);
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
    let (contract, user, erc20_address) = deploy_predifi();

    // Approve token spending
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
            Category::Sports,
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
            Category::Crypto,
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
    contract.update_pool_state(pool_id1);
    contract.update_pool_state(pool_id2);

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
    contract.update_pool_state(pool_id1);
    contract.update_pool_state(pool_id2);

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
    contract.update_pool_state(pool_id1);
    contract.update_pool_state(pool_id2);

    // Check statuses
    let settled_pools = contract.get_user_settled_pools(user);
    assert(settled_pools.len() == 2, 'Should have 2 settled pools');

    // Time warp to 24 hours after pool 2 ended (should transition to closed)
    start_cheat_block_timestamp(contract.contract_address, current_time + 5400 + 86401);

    // Update the pool states
    contract.update_pool_state(pool_id2);

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
    let (contract, admin, erc20_address) = deploy_predifi();
    let user1 = contract_address_const::<1>();
    let user2 = contract_address_const::<2>();
    let user3 = contract_address_const::<3>();

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
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(erc20_address, user1);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(erc20_address, user2);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(erc20_address, user3);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
            Category::Sports,
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
            Category::Crypto,
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
    contract.update_pool_state(pool_id1);
    contract.update_pool_state(pool_id2);

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
    contract.update_pool_state(pool_id1);
    contract.update_pool_state(pool_id2);

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
    contract.update_pool_state(pool_id1);
    contract.update_pool_state(pool_id2);

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
    contract.update_pool_state(pool_id2);

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


//test for automated validation

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
    let admin = contract_address_const::<'admin'>();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator1);
    contract.add_validator(validator2);
    contract.add_validator(validator3);
    contract.add_validator(validator4);
    stop_cheat_caller_address(contract.contract_address);

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

    assert(is_valid_validator1, 'Invalid validator1 assigned');
    assert(is_valid_validator2, 'Invalid validator2 assigned');
}

#[test]
fn test_assign_exactly_two_validators() {
    // Deploy the contract
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Create exactly two validators with different addresses
    let validator1 = contract_address_const::<'validator1'>();
    let validator2 = contract_address_const::<'validator2'>();
    let zero_address: ContractAddress = contract_address_const::<'zero'>();

    // Add validators to the contract (overriding any existing validators)
    let admin = contract_address_const::<'admin'>();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator1);
    contract.add_validator(validator2);
    stop_cheat_caller_address(contract.contract_address);

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
        contract.assign_random_validators(pool_id);
        j += 1;
    }

    // Verify all pools have valid validators assigned
    let mut k: u8 = 0;
    while k < num_pools {
        let pool_id = *pool_ids.at(k.into());
        let (pool_validator1, pool_validator2) = contract.get_pool_validators(pool_id);

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
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Create multiple validators with different addresses
    let validator1 = contract_address_const::<'validator1'>();
    let validator2 = contract_address_const::<'validator2'>();
    let validator3 = contract_address_const::<'validator3'>();
    let validator4 = contract_address_const::<'validator4'>();

    // Add validators to the contract
    let admin = contract_address_const::<'admin'>();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator1);
    contract.add_validator(validator2);
    contract.add_validator(validator3);
    contract.add_validator(validator4);
    stop_cheat_caller_address(contract.contract_address);

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

#[test]
fn test_limited_validators_assignment() {
    // Deploy the contract
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Create just one validator
    let single_validator = contract_address_const::<'single_validator'>();

    // Add only one validator to the contract
    let admin = contract_address_const::<'admin'>();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(single_validator);
    stop_cheat_caller_address(contract.contract_address);

    // Set up token approval for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 1_000_000_000_000_000_000_000_000);
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

    // Check that all pools have the same validator assigned
    // Since there's only one validator, it should be assigned to all pools
    let mut i: u32 = 0;
    while i < pool_ids.len() {
        let pool_id = *pool_ids.at(i);
        let (assigned_validator1, assigned_validator2) = contract.get_pool_validators(pool_id);

        // Both validator1 and validator2 should be the single validator we added
        assert(assigned_validator1 == single_validator, 'Wrong validator1 assigned');
        assert(assigned_validator2 == single_validator, 'Wrong validator2 assigned');

        i += 1;
    }

    // Now add a second validator and verify it gets used for new pools
    let second_validator = contract_address_const::<'second_validator'>();

    // Add second validator to the contract
    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(second_validator);
    stop_cheat_caller_address(contract.contract_address);

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
            Category::Sports // category
        );
    stop_cheat_caller_address(contract.contract_address);

    // Assign validators to the new pool
    contract.assign_random_validators(new_pool_id);

    // Check that the new pool has different validators assigned
    let (new_assigned_validator1, new_assigned_validator2) = contract
        .get_pool_validators(new_pool_id);

    // At least one of the validators should be the second validator
    let has_second_validator = new_assigned_validator1 == second_validator
        || new_assigned_validator2 == second_validator;

    assert(has_second_validator, 'Second validator not used');
}


#[test]
fn test_assign_random_validators_initial_validator() {
    // Deploy the contract
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Get the validator that was added during deployment
    let expected_validator = contract_address_const::<'validator'>();

    // Explicitly add the validator to the validators list
    let admin = contract_address_const::<'admin'>();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(expected_validator);
    stop_cheat_caller_address(contract.contract_address);

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
    assert(assigned_validator1 == expected_validator, 'Should assign initial valdator');
    assert(assigned_validator2 == expected_validator, 'Should assign initial validator');
}

#[test]
fn test_add_validator() {
    let mut state = Predifi::contract_state_for_testing();
    let test_address: ContractAddress = test_address();

    let admin: ContractAddress = contract_address_const::<'admin'>();
    let validator: ContractAddress = contract_address_const::<'validator'>();

    // Initialize access control and grant DEFAULT_ADMIN_ROLE to admin
    AccessControlInternalTrait::initializer(ref state.accesscontrol);
    AccessControlInternalTrait::_grant_role(ref state.accesscontrol, DEFAULT_ADMIN_ROLE, admin);

    // Act as admin to add a validator
    start_cheat_caller_address(test_address, admin);
    let mut spy = spy_events();
    IPredifi::add_validator(ref state, validator);
    stop_cheat_caller_address(test_address);

    // Assert validator is added to the list
    let added_validator: ContractAddress = state.validators.at(0).read();
    assert(added_validator == validator, 'Validator not added');

    // Assert validator role is set
    let is_validator = IPredifi::is_validator(@state, validator);
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
    let validator: ContractAddress = contract_address_const::<'validator'>();

    AccessControlInternalTrait::initializer(ref state.accesscontrol);

    // Unauthorized caller attempt to add a new validator
    IPredifi::add_validator(ref state, validator);
}

#[test]
fn test_remove_validator_role() {
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let validator1: ContractAddress = contract_address_const::<'validator1'>();
    let validator2: ContractAddress = contract_address_const::<'validator2'>();

    let mut state = Predifi::contract_state_for_testing();
    let test_address: ContractAddress = test_address();

    // Initialize access control and grant DEFAULT_ADMIN_ROLE to admin
    AccessControlInternalTrait::initializer(ref state.accesscontrol);
    AccessControlInternalTrait::_grant_role(ref state.accesscontrol, DEFAULT_ADMIN_ROLE, admin);

    // Act as admin to add two validators
    start_cheat_caller_address(test_address, admin);
    IPredifi::add_validator(ref state, validator1);
    IPredifi::add_validator(ref state, validator2);
    stop_cheat_caller_address(test_address);

    // Act as admin to remove validator1
    start_cheat_caller_address(test_address, admin);
    let mut spy = spy_events();
    IPredifi::remove_validator(ref state, validator1);
    stop_cheat_caller_address(test_address);

    // Assert only one validator remains
    let validator_count = state.validators.len();
    assert(validator_count == 1, 'Expected only one validator');

    // Assert validator2 is the remaining validator
    let remaining_validator: ContractAddress = state.validators.at(0).read();
    assert(remaining_validator == validator2, 'Validator2 should remain');

    // Assert validator1 role is revoked
    let is_validator = IPredifi::is_validator(@state, validator1);
    assert(!is_validator, 'Validator1 was not revoked');

    // Assert correct event was emitted
    let expected_event = Predifi::Event::ValidatorRemoved(
        ValidatorRemoved { account: validator1, caller: admin },
    );
    spy.assert_emitted(@array![(test_address, expected_event)]);

    // Act as admin to remove the second validator
    start_cheat_caller_address(test_address, admin);
    IPredifi::remove_validator(ref state, validator2);
    stop_cheat_caller_address(test_address);

    // Assert no validators remain
    let validator_count = state.validators.len();
    assert(validator_count == 0, 'Expected zero validators');
}

#[test]
#[should_panic(expected: 'Caller is missing role')]
fn test_remove_validator_unauthorized() {
    let mut state = Predifi::contract_state_for_testing();
    let validator: ContractAddress = contract_address_const::<'validator'>();

    AccessControlInternalTrait::initializer(ref state.accesscontrol);

    // Unauthorized caller attempt to remove the validator role
    IPredifi::remove_validator(ref state, validator);
}

// Helper function to create a test pool
fn create_test_pool(
    dispatcher: IPredifiDispatcher,
    poolName: felt252,
    poolStartTime: u64,
    poolLockTime: u64,
    poolEndTime: u64,
) -> u256 {
    dispatcher
        .create_pool(
            poolName,
            0, // 0 = WinBet
            "Test Description",
            "Test Image",
            "Test URL",
            poolStartTime,
            poolLockTime,
            poolEndTime,
            'Option 1',
            'Option 2',
            100_u256,
            1000_u256,
            5,
            false,
            Category::Sports,
        )
}


fn pool_exists_in_array(pools: Array<PoolDetails>, pool_id: u256) -> bool {
    let mut i = 0;
    let len = pools.len();

    loop {
        if i >= len {
            break false;
        }

        let pool = pools.at(i);
        // Use the correct reference type for comparison
        if *pool.pool_id == pool_id {
            break true;
        }

        i += 1;
    }
}

#[test]
fn test_minimal_timing() {
    let (dispatcher, pool_creator, erc20_address) = deploy_predifi();

    let erc20_dispatcher: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    start_cheat_caller_address(erc20_address, pool_creator);
    erc20_dispatcher.approve(dispatcher.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    let t0 = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, t0);

    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool_id = create_test_pool(
        dispatcher, 'Test Pool', t0 + 1000, // 2000
        t0 + 2000, // 3000
        t0 + 3000 // 4000
    );
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_get_active_pools() {
    // Deploy the contract
    let (dispatcher, pool_creator, erc20_address) = deploy_predifi();

    let erc20_dispatcher: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Approve the dispatcher contract to spend tokens
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20_dispatcher.approve(dispatcher.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Set initial block timestamp
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
    dispatcher.update_pool_state(pool1_id);
    dispatcher.update_pool_state(pool2_id);

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
    let (dispatcher, pool_creator, erc20_address) = deploy_predifi();

    let erc20_dispatcher: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    start_cheat_caller_address(erc20_address, pool_creator);
    erc20_dispatcher.approve(dispatcher.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);

    // Pool 1
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher,
        'Locked Pool 1',
        initial_time + 1000, // start: 2000
        initial_time + 2000, // lock: 3000
        initial_time + 3000 // end: 4000
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Set block timestamp for Pool 2 creation
    let time_2 = initial_time + 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, time_2);

    // Pool 2 (start time strictly greater than block timestamp)
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool2_id = create_test_pool(
        dispatcher,
        'Locked Pool 2',
        time_2 + 1, // start: 2001
        time_2 + 1001, // lock: 3001
        time_2 + 2001 // end: 4001
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Advance time to just after both locks but before both ends
    let locked_time = time_2 + 1200; // 2200 > 2001 (lock), < 4001 (end)
    start_cheat_block_timestamp(dispatcher.contract_address, locked_time);

    dispatcher.update_pool_state(pool1_id);
    dispatcher.update_pool_state(pool2_id);

    let locked_pools = dispatcher.get_locked_pools();
    let pool1 = dispatcher.get_pool(pool1_id);
    let pool2 = dispatcher.get_pool(pool2_id);

    assert(pool1.status == Status::Locked, 'Pool 1 should be locked');
    assert(pool2.status == Status::Locked, 'Pool 2 should be locked');
    assert(locked_pools.len() == 2, 'Expected 2 locked pools');
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_get_settled_pools() {
    let (dispatcher, pool_creator, erc20_address) = deploy_predifi();

    let erc20_dispatcher: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    start_cheat_caller_address(erc20_address, pool_creator);
    erc20_dispatcher.approve(dispatcher.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);

    // Pool 1
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher,
        'Settled Pool 1',
        initial_time + 1000, // start: 2000
        initial_time + 2000, // lock: 3000
        initial_time + 3000 // end: 4000
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Set block timestamp for Pool 2 creation
    let time_2 = initial_time + 1500; // 2500
    start_cheat_block_timestamp(dispatcher.contract_address, time_2);

    // Pool 2 (start time strictly greater than block timestamp)
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool2_id = create_test_pool(
        dispatcher,
        'Settled Pool 2',
        time_2 + 100, // start: 2600
        time_2 + 1100, // lock: 3600
        time_2 + 2100 // end: 4600
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Advance time to after both ends
    let settled_time = initial_time + 5000; // 5000 > 4000 and 4600
    start_cheat_block_timestamp(dispatcher.contract_address, settled_time);

    dispatcher.update_pool_state(pool1_id);
    dispatcher.update_pool_state(pool2_id);

    let settled_pools = dispatcher.get_settled_pools();
    let pool1 = dispatcher.get_pool(pool1_id);
    let pool2 = dispatcher.get_pool(pool2_id);
    assert(pool1.status == Status::Settled, 'Pool 1 should be settled');
    assert(pool2.status == Status::Settled, 'Pool 2 should be settled');
    assert(settled_pools.len() == 2, 'Expected 2 settled pools');
    stop_cheat_block_timestamp(dispatcher.contract_address);
}

#[test]
fn test_get_closed_pools() {
    let (dispatcher, pool_creator, erc20_address) = deploy_predifi();

    let erc20_dispatcher: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    start_cheat_caller_address(erc20_address, pool_creator);
    erc20_dispatcher.approve(dispatcher.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    let initial_time = 1000;
    start_cheat_block_timestamp(dispatcher.contract_address, initial_time);

    // Pool 1
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool1_id = create_test_pool(
        dispatcher,
        'Closed Pool 1',
        initial_time + 1000, // start: 2000
        initial_time + 2000, // lock: 3000
        initial_time + 3000 // end: 4000
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Set block timestamp for Pool 2 creation
    let time_2 = initial_time + 1500; // 2500
    start_cheat_block_timestamp(dispatcher.contract_address, time_2);

    // Pool 2 (start time strictly greater than block timestamp)
    start_cheat_caller_address(dispatcher.contract_address, pool_creator);
    let pool2_id = create_test_pool(
        dispatcher,
        'Closed Pool 2',
        time_2 + 100, // start: 2600
        time_2 + 1100, // lock: 3600
        time_2 + 2100 // end: 4600
    );
    stop_cheat_caller_address(dispatcher.contract_address);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Assume pool1_id and pool2_id are created, and you have their end times
    let end_time_1 = 4000; // set to pool 1's end time
    let end_time_2 = 4600; // set to pool 2's end time
    let after_end = core::cmp::max(end_time_1, end_time_2) + 1;
    start_cheat_block_timestamp(dispatcher.contract_address, after_end);
    dispatcher.update_pool_state(pool1_id);
    dispatcher.update_pool_state(pool2_id);
    stop_cheat_block_timestamp(dispatcher.contract_address);

    // Now advance to after end_time + 86401 for the latest pool
    let after_closed = core::cmp::max(end_time_1, end_time_2) + 86401;
    start_cheat_block_timestamp(dispatcher.contract_address, after_closed);
    dispatcher.update_pool_state(pool1_id);
    dispatcher.update_pool_state(pool2_id);

    let closed_pools = dispatcher.get_closed_pools();
    let pool1 = dispatcher.get_pool(pool1_id);
    let pool2 = dispatcher.get_pool(pool2_id);
    println!("Pool 1 status: {:?}", pool1.status);
    println!("Pool 2 status: {:?}", pool2.status);
    println!("closed_pools.len(): {:?}", closed_pools.len());
    assert(pool1.status == Status::Closed, 'Pool 1 should be closed');
    assert(pool2.status == Status::Closed, 'Pool 2 should be closed');
    assert(closed_pools.len() == 2, 'Expected 2 closed pools');
}

// ========================================
// DISPUTE FUNCTIONALITY TESTS
// ========================================

// Helper function to create a user address
// fn create_user(index: felt252) -> ContractAddress {
//     contract_address_const::<index>();
// }

// Helper function to approve and fund a user for testing
fn setup_user_with_tokens(user: ContractAddress, erc20_address: ContractAddress, amount: u256) {
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(erc20_address, amount); // Approve the ERC20 contract itself to mint
    stop_cheat_caller_address(erc20_address);
}

#[test]
fn test_dispute_threshold_initial_value() {
    let (contract, _pool_creator, _erc20_address) = deploy_predifi();

    let threshold = contract.get_dispute_threshold();
    assert(threshold == 3, 'Default threshold should be 3');
}

#[test]
fn test_raise_dispute_success() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Create a user and raise dispute
    let user1 = contract_address_const::<'user1'>();

    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Verify dispute was raised
    let dispute_count = contract.get_dispute_count(pool_id);
    assert(dispute_count == 1, 'Dispute count should be 1');

    let has_disputed = contract.has_user_disputed(pool_id, user1);
    assert(has_disputed, 'User should have disputed');

    // Pool should still be active (threshold not reached)
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Active, 'Pool should still be active');
}

#[test]
fn test_raise_dispute_threshold_reached() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Create users and raise disputes to reach threshold
    let user1 = contract_address_const::<'user1'>();
    let user2 = contract_address_const::<'user2'>();
    let user3 = contract_address_const::<'user3'>();

    // First dispute
    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Second dispute
    start_cheat_caller_address(contract.contract_address, user2);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Third dispute - should trigger suspension
    start_cheat_caller_address(contract.contract_address, user3);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Verify pool is suspended
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Suspended, 'Pool should be suspended');

    let dispute_count = contract.get_dispute_count(pool_id);
    assert(dispute_count == 3, 'Dispute count should be 3');

    let is_suspended = contract.is_pool_suspended(pool_id);
    assert(is_suspended, 'Pool should be suspended');
}

#[test]
#[should_panic(expected: 'User already raised dispute')]
fn test_raise_dispute_already_disputed() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    let user1 = contract_address_const::<'user1'>();

    // Raise dispute first time
    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool_id);

    // Try to raise dispute again (should panic)
    contract.raise_dispute(pool_id);
}

#[test]
#[should_panic(expected: 'Pool is inactive')]
fn test_raise_dispute_nonexistent_pool() {
    let (contract, _pool_creator, _erc20_address) = deploy_predifi();

    let user1 = contract_address_const::<'user1'>();
    let nonexistent_pool_id = 999999;

    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(nonexistent_pool_id);
}

#[test]
#[should_panic(expected: 'Pool is suspended')]
fn test_raise_dispute_already_suspended() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup and create pool
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Reach threshold to suspend pool
    let user1 = contract_address_const::<'user1'>();
    let user2 = contract_address_const::<'user2'>();
    let user3 = contract_address_const::<'user3'>();
    let user4 = contract_address_const::<'user4'>();

    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user2);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user3);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Try to raise dispute on suspended pool
    start_cheat_caller_address(contract.contract_address, user4);
    contract.raise_dispute(pool_id);
}

#[test]
fn test_resolve_dispute_success() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Get the initial status
    let initial_pool = contract.get_pool(pool_id);
    let initial_status = initial_pool.status;

    // Suspend pool by reaching threshold
    let user1 = contract_address_const::<'user1'>();
    let user2 = contract_address_const::<'user2'>();
    let user3 = contract_address_const::<'user3'>();

    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user2);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user3);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Verify suspension
    let suspended_pool = contract.get_pool(pool_id);
    assert(suspended_pool.status == Status::Suspended, 'Pool should be suspended');

    // Admin resolves dispute
    let admin = contract_address_const::<'admin'>();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.resolve_dispute(pool_id, true);
    stop_cheat_caller_address(contract.contract_address);

    // Verify resolution
    let resolved_pool = contract.get_pool(pool_id);
    assert(resolved_pool.status == initial_status, 'Status should be restored');

    let dispute_count = contract.get_dispute_count(pool_id);
    assert(dispute_count == 0, 'Dispute count should be reset');

    let is_suspended = contract.is_pool_suspended(pool_id);
    assert(!is_suspended, 'Pool should not be suspended');
}

#[test]
#[should_panic(expected: 'Pool is not suspended')]
fn test_resolve_dispute_not_suspended() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Try to resolve dispute on non-suspended pool
    let admin = contract_address_const::<'admin'>();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.resolve_dispute(pool_id, true);
}

#[test]
fn test_get_suspended_pools() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create two pools
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool1_id = create_default_pool(contract);
    let pool2_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Suspend only first pool
    let user1 = contract_address_const::<'user1'>();
    let user2 = contract_address_const::<'user2'>();
    let user3 = contract_address_const::<'user3'>();

    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool1_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user2);
    contract.raise_dispute(pool1_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user3);
    contract.raise_dispute(pool1_id);
    stop_cheat_caller_address(contract.contract_address);

    // Check suspended pools
    let suspended_pools = contract.get_suspended_pools();
    assert(suspended_pools.len() == 1, 'Should have 1 suspended pool');

    let suspended_pool = suspended_pools.at(0);
    assert(*suspended_pool.pool_id == pool1_id, 'Wrong pool suspended');
}

#[test]
#[should_panic(expected: 'Pool is suspended')]
fn test_vote_on_suspended_pool() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create pool with proper timestamps
    let current_time = get_block_timestamp();
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // 0 = WinBet
            "A test pool for suspension",
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
            Category::Sports,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Verify pool exists and is active
    let initial_pool = contract.get_pool(pool_id);
    assert(initial_pool.exists, 'Pool should exist');
    assert(initial_pool.status == Status::Active, 'Pool should be active');

    // Suspend pool by raising enough disputes
    let user1 = contract_address_const::<'user1'>();
    let user2 = contract_address_const::<'user2'>();
    let user3 = contract_address_const::<'user3'>();

    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user2);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user3);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Verify pool is suspended
    let suspended_pool = contract.get_pool(pool_id);
    assert(suspended_pool.status == Status::Suspended, 'Pool should be suspended');

    // Try to vote on suspended pool (should panic)
    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract.vote(pool_id, 'Team A', 200);
}

#[test]
#[should_panic(expected: 'Pool is suspended')]
fn test_stake_on_suspended_pool() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Suspend pool
    let user1 = contract_address_const::<'user1'>();
    let user2 = contract_address_const::<'user2'>();
    let user3 = contract_address_const::<'user3'>();

    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user2);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user3);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Try to stake on suspended pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract.stake(pool_id, 200_000_000_000_000_000_000);
}

#[test]
fn test_refund_stake_successful() {
    let (contract, caller, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, caller);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, caller);
    let pool_id = create_default_pool(contract);
    let stake_amount: u256 = 200_000_000_000_000_000_000;

    contract.stake(pool_id, stake_amount);
    contract.cancel_pool(pool_id);

    contract.refund_stake(pool_id);
    assert(contract.get_user_stake(pool_id, caller).amount == 0, 'Invalid stake amount');
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
fn test_refund_stake_event_emission() {
    let (contract, caller, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let mut spy = spy_events();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, caller);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, caller);
    let pool_id = create_default_pool(contract);
    let stake_amount: u256 = 200_000_000_000_000_000_000;

    contract.stake(pool_id, stake_amount);
    contract.cancel_pool(pool_id);

    contract.refund_stake(pool_id);
    assert(contract.get_user_stake(pool_id, caller).amount == 0, 'Invalid stake amount');
    stop_cheat_caller_address(contract.contract_address);

    // Assert event emitted
    let expected_event = Predifi::Event::StakeRefunded(
        StakeRefunded { pool_id, address: caller, amount: stake_amount },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_event)]);
}

#[test]
#[should_panic(expected: 'Pool is not closed')]
fn test_refund_stake_on_open_pool() {
    let (contract, caller, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

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

    contract.refund_stake(pool_id);
    assert(contract.get_user_stake(pool_id, caller).amount == 0, 'Invalid stake amount');
}

#[test]
#[should_panic(expected: 'Zero user stake')]
fn test_refund_zero_stake() {
    let (contract, caller, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the DISPATCHER contract to spend tokens
    start_cheat_caller_address(erc20_address, caller);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, caller);
    let pool_id = create_default_pool(contract);
    let stake_amount: u256 = 200_000_000_000_000_000_000;
    contract.cancel_pool(pool_id);

    contract.refund_stake(pool_id);
    assert(contract.get_user_stake(pool_id, caller).amount == 0, 'Invalid stake amount');
    stop_cheat_caller_address(contract.contract_address);
}


#[test]
fn test_validate_outcome_success() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Get current time and create pool with relative timestamps
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
            Category::Sports,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Move time to after lock time but before end time
    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    contract.update_pool_state(pool_id);
    stop_cheat_block_timestamp(contract.contract_address);

    // Verify pool is locked
    let locked_pool = contract.get_pool(pool_id);
    assert(locked_pool.status == Status::Locked, 'Pool should be locked');

    // Add a validator and validate outcome
    let admin = contract_address_const::<'admin'>();
    let validator = contract_address_const::<'validator'>();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, validator);
    contract.validate_outcome(pool_id, true);
    stop_cheat_caller_address(contract.contract_address);

    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Locked, 'Pool should remain locked');
}

#[test]
#[should_panic(expected: 'Pool is suspended')]
fn test_validate_outcome_suspended_pool() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Suspend pool
    let user1 = contract_address_const::<'user1'>();
    let user2 = contract_address_const::<'user2'>();
    let user3 = contract_address_const::<'user3'>();

    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user2);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user3);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Try to validate suspended pool
    let admin = contract_address_const::<'admin'>();
    let validator = contract_address_const::<'admin'>();
    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, validator);
    contract.validate_outcome(pool_id, true);
}

#[test]
#[should_panic(expected: 'Pool is suspended')]
fn test_claim_reward_suspended_pool() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Suspend pool
    let user1 = contract_address_const::<'user1'>();
    let user2 = contract_address_const::<'user2'>();
    let user3 = contract_address_const::<'user3'>();

    start_cheat_caller_address(contract.contract_address, user1);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user2);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, user3);
    contract.raise_dispute(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Try to claim reward on suspended pool
    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract.claim_reward(pool_id);
}

#[test]
fn test_validate_pool_result_success() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup ERC20 approval
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
            Category::Sports,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Move time to lock the pool
    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    contract.update_pool_state(pool_id);
    stop_cheat_block_timestamp(contract.contract_address);

    // Verify pool is locked
    let locked_pool = contract.get_pool(pool_id);
    assert(locked_pool.status == Status::Locked, 'Pool should be locked');

    // Add validators
    let admin = contract_address_const::<'admin'>();
    let validator1 = contract_address_const::<'validator1'>();
    let validator2 = contract_address_const::<'validator2'>();

    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator1);
    contract.add_validator(validator2);
    stop_cheat_caller_address(contract.contract_address);

    // First validator validates - pool should remain locked
    start_cheat_caller_address(contract.contract_address, validator1);
    contract.validate_pool_result(pool_id, true); // Vote for option2
    stop_cheat_caller_address(contract.contract_address);

    let pool_after_first_validation = contract.get_pool(pool_id);
    assert(pool_after_first_validation.status == Status::Locked, 'Pool should still be locked');

    // Check validation status
    let (validation_count, is_settled, _) = contract.get_pool_validation_status(pool_id);
    assert(validation_count == 1, 'Should have 1 validation');
    assert(!is_settled, 'Pool should not be settled yet');

    // Second validator validates - pool should be settled
    start_cheat_caller_address(contract.contract_address, validator2);
    contract.validate_pool_result(pool_id, true); // Vote for option2
    stop_cheat_caller_address(contract.contract_address);

    let pool_after_second_validation = contract.get_pool(pool_id);
    assert(pool_after_second_validation.status == Status::Settled, 'Pool should be settled');

    // Check final validation status
    let (final_validation_count, final_is_settled, final_outcome) = contract
        .get_pool_validation_status(pool_id);
    assert(final_validation_count == 2, 'Should have 2 validations');
    assert(final_is_settled, 'Pool should be settled');
    assert(final_outcome, 'Option2 should win');
}

#[test]
#[should_panic(expected: 'Validator not authorized')]
fn test_validate_pool_result_unauthorized() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup and create pool
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Lock the pool
    let current_time = get_block_timestamp();
    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    contract.update_pool_state(pool_id);
    stop_cheat_block_timestamp(contract.contract_address);

    // Try to validate without being a validator
    let unauthorized_user = contract_address_const::<'unauthorized'>();
    start_cheat_caller_address(contract.contract_address, unauthorized_user);
    contract.validate_pool_result(pool_id, true);
}

#[test]
#[should_panic(expected: 'Validator already validated')]
fn test_validate_pool_result_double_validation() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
            Category::Sports,
        );
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    contract.update_pool_state(pool_id);
    stop_cheat_block_timestamp(contract.contract_address);

    // Add validator
    let admin = contract_address_const::<'admin'>();
    let validator = contract_address_const::<'validator'>();

    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator);
    stop_cheat_caller_address(contract.contract_address);

    // First validation
    start_cheat_caller_address(contract.contract_address, validator);
    contract.validate_pool_result(pool_id, true);

    // Try to validate again - should panic
    contract.validate_pool_result(pool_id, false);
}

#[test]
#[should_panic(expected: 'Pool not ready for validation')]
fn test_validate_pool_result_wrong_status() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create pool but don't lock it
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Add validator
    let admin = contract_address_const::<'admin'>();
    let validator = contract_address_const::<'validator'>();

    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator);
    stop_cheat_caller_address(contract.contract_address);

    // Try to validate active pool - should panic
    start_cheat_caller_address(contract.contract_address, validator);
    contract.validate_pool_result(pool_id, true);
}

#[test]
fn test_validation_consensus_majority_option1() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
            Category::Sports,
        );
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    contract.update_pool_state(pool_id);
    stop_cheat_block_timestamp(contract.contract_address);

    // Add 3 validators and set required confirmations to 3
    let admin = contract_address_const::<'admin'>();
    let validator1 = contract_address_const::<'validator1'>();
    let validator2 = contract_address_const::<'validator2'>();
    let validator3 = contract_address_const::<'validator3'>();

    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator1);
    contract.add_validator(validator2);
    contract.add_validator(validator3);
    contract.set_required_validator_confirmations(3);
    stop_cheat_caller_address(contract.contract_address);

    // Validators vote: 2 for option1 (false), 1 for option2 (true)
    start_cheat_caller_address(contract.contract_address, validator1);
    contract.validate_pool_result(pool_id, false); // Option1
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, validator2);
    contract.validate_pool_result(pool_id, false); // Option1
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, validator3);
    contract.validate_pool_result(pool_id, true); // Option2 - this triggers settlement
    stop_cheat_caller_address(contract.contract_address);

    // Check final outcome - should be option1 (false) since it got majority
    let (_, is_settled, final_outcome) = contract.get_pool_validation_status(pool_id);
    assert(is_settled, 'Pool should be settled');
    assert(!final_outcome, 'Option1 should win majority');

    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Settled, 'Pool should be settled');
}

#[test]
fn test_get_validator_confirmation() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup and create locked pool
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
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
            Category::Sports,
        );
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_block_timestamp(contract.contract_address, current_time + 250);
    contract.update_pool_state(pool_id);
    stop_cheat_block_timestamp(contract.contract_address);

    // Add validator
    let admin = contract_address_const::<'admin'>();
    let validator = contract_address_const::<'validator'>();

    start_cheat_caller_address(contract.contract_address, admin);
    contract.add_validator(validator);
    stop_cheat_caller_address(contract.contract_address);

    // Check before validation
    let (has_validated, _) = contract.get_validator_confirmation(pool_id, validator);
    assert(!has_validated, 'Should not have validated yet');

    // Validate
    start_cheat_caller_address(contract.contract_address, validator);
    contract.validate_pool_result(pool_id, true);
    stop_cheat_caller_address(contract.contract_address);

    // Check after validation
    let (has_validated_after, selected_option) = contract
        .get_validator_confirmation(pool_id, validator);
    assert(has_validated_after, 'Should have validated');
    assert(selected_option, 'Should have selected option2');
}

#[test]
#[should_panic(expected: 'Pausable: paused')]
fn test_predify_contract_pause_success() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract (by admin)
    start_cheat_caller_address(contract.contract_address, admin);
    contract.pause();
    stop_cheat_caller_address(contract.contract_address);

    // Try to create a pool while paused
    let pool1_id = create_default_pool(contract);
}

#[test]
#[should_panic(expected: 'Caller is missing role')]
fn test_non_admin_pause_predify_contract() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract by non-admin
    start_cheat_caller_address(contract.contract_address, pool_creator);
    contract.pause();
    stop_cheat_caller_address(contract.contract_address);
}


#[test]
#[should_panic(expected: 'Pausable: not paused')]
fn test_unpause_not_paused_predify_contract() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract
    start_cheat_caller_address(contract.contract_address, admin);
    contract.unpause();
}

#[test]
#[should_panic(expected: 'Pausable: paused')]
fn test_pause_paused_predify_contract() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract
    start_cheat_caller_address(contract.contract_address, admin);
    contract.pause();
    contract.pause();
}

#[test]
fn test_predify_contract_unpause_success() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Pause the contract
    start_cheat_caller_address(contract.contract_address, admin);
    contract.pause();
    stop_cheat_caller_address(contract.contract_address);

    // Unpause the contract
    start_cheat_caller_address(contract.contract_address, admin);
    contract.unpause();

    // Create a pool after unpausing
    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);
    assert!(pool_id != 0, "Pool not created successfully");
}

#[test]
#[feature("safe_dispatcher")]
fn test_multiple_functions_panic_when_paused() {
    let (contract, pool_creator, erc20_address) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let safe_dispatcher = IPredifiSafeDispatcher { contract_address: contract.contract_address };

    // Setup
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, pool_creator);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, admin);
    match safe_dispatcher.pause() {
        Result::Ok(_) => {}, // Expected: Pausing should succeed without panicking
        Result::Err(_) => {
            panic!("Pause function unexpectedly panicked");
        } // Unexpected: Pausing should not panic
    }

    // Test first paused function
    let result1 = safe_dispatcher.validate_pool_result(1, true);
    match result1 {
        Result::Ok(_) => panic!("Not panic when paused"), // Test fails if it didn't panic
        Result::Err(panic_data) => {
            let expected_panic_message = 'Pausable: paused';
            assert(*panic_data.at(0) == expected_panic_message, 'Wrong panic message');
        },
    }

    // Test second paused function
    let result2 = safe_dispatcher.claim_reward(1);
    match result2 {
        Result::Ok(_) => panic!("Not panic when paused"), // Test fails if it didn't panic
        Result::Err(panic_data) => {
            let expected_panic_message = 'Pausable: paused';
            assert(*panic_data.at(0) == expected_panic_message, 'Wrong panic message');
        },
    }

    // Test third paused function
    let result3 = safe_dispatcher.refund_stake(1);
    match result3 {
        Result::Ok(_) => panic!("Not panic when paused"), // Test fails if it didn't panic
        Result::Err(panic_data) => {
            let expected_panic_message = 'Pausable: paused';
            assert(*panic_data.at(0) == expected_panic_message, 'Wrong panic message');
        },
    };
}

#[test]
fn test_upgrade_by_admin() {
    let (contract, _, _) = deploy_predifi();
    let admin = contract_address_const::<'admin'>();
    let new_class_hash = declare_contract("STARKTOKEN");
    let mut spy = spy_events();

    // Set caller address to admin
    start_cheat_caller_address(contract.contract_address, admin);

    // Call the upgrade function as the admin
    contract.upgrade(new_class_hash);

    stop_cheat_caller_address(contract.contract_address);

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
    let (contract, pool_creator, _) = deploy_predifi();
    let new_class_hash = declare_contract("STARKTOKEN");

    // Set caller address to non-owner
    start_cheat_caller_address(contract.contract_address, pool_creator);

    // Attempt to call the upgrade function as a non-owner
    contract.upgrade(new_class_hash);
}

#[test]
#[should_panic(expected: 'Pausable: paused')]
fn test_upgrade_fails_when_paused() {
    let (contract, pool_creator, _) = deploy_predifi();
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let new_class_hash = declare_contract("STARKTOKEN");

    start_cheat_caller_address(contract.contract_address, admin);
    // Pause the contract
    contract.pause();

    contract.upgrade(new_class_hash);
}
