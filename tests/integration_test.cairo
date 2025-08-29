// Integration tests for multi-step flows in the Predifi contract
use contract::base::events::Events::{
    DisputeResolved, PoolAutomaticallySettled, PoolCancelled, PoolSuspended, StakeRefunded,
    UserStaked,
};
use contract::base::types::{Category, Status};
use contract::interfaces::ipredifi::{
    IPredifiDispatcher, IPredifiDispatcherTrait, IPredifiDisputeDispatcher,
    IPredifiDisputeDispatcherTrait, IPredifiValidatorDispatcher, IPredifiValidatorDispatcherTrait,
};
use contract::predifi::Predifi;
use core::array::ArrayTrait;
use core::felt252;
use core::traits::{Into, TryInto};
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, EventSpyAssertionsTrait, declare, spy_events,
    start_cheat_block_timestamp, start_cheat_caller_address, stop_cheat_block_timestamp,
    stop_cheat_caller_address,
};
use starknet::{ContractAddress, get_block_timestamp};

// Validator role
const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");
// Pool creator address constant
const POOL_CREATOR: ContractAddress = 123.try_into().unwrap();
const USER_ONE: ContractAddress = 'User1'.try_into().unwrap();
const USER_TWO: ContractAddress = 'User2'.try_into().unwrap();
const USER_THREE: ContractAddress = 'User3'.try_into().unwrap();
const VALIDATOR_ONE: ContractAddress = 'Validator1'.try_into().unwrap();
const VALIDATOR_TWO: ContractAddress = 'Validator2'.try_into().unwrap();
const VALIDATOR_THREE: ContractAddress = 'Validator3'.try_into().unwrap();

const ONE_STRK: u256 = 1_000_000_000_000_000_000;
const MIN_STAKE_AMOUNT: u256 = 200_000_000_000_000_000_000;

fn deploy_predifi() -> (
    IPredifiDispatcher,
    IPredifiDisputeDispatcher,
    IPredifiValidatorDispatcher,
    ContractAddress,
    ContractAddress,
) {
    let owner: ContractAddress = 'owner'.try_into().unwrap();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Deploy mock ERC20
    let erc20_class = declare("STARKTOKEN").unwrap().contract_class();
    let mut calldata = array![POOL_CREATOR.into(), owner.into(), 6];
    let (erc20_address, _) = erc20_class.deploy(@calldata).unwrap();

    let contract_class = declare("Predifi").unwrap().contract_class();

    let (contract_address, _) = contract_class
        .deploy(@array![erc20_address.into(), admin.into()])
        .unwrap();

    let dispatcher = IPredifiDispatcher { contract_address };
    let dispute_dispatcher = IPredifiDisputeDispatcher { contract_address };
    let validator_dispatcher = IPredifiValidatorDispatcher { contract_address };
    (dispatcher, dispute_dispatcher, validator_dispatcher, POOL_CREATOR, erc20_address)
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
            0,
        )
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
    (
        'Test Pool',
        0, // WinBet
        "Test Description",
        "test.png",
        "test.com",
        1710000000,
        1710003600,
        1710007200,
        'Option1',
        'Option2',
        100,
        10000,
        5,
        false,
        Category::Sports,
    )
}

fn setup_tokens_and_approvals(
    erc20_address: ContractAddress, contract_address: ContractAddress, users: Span<ContractAddress>,
) {
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Distribute tokens and approve contract
    let mut i = 0;
    while i != users.len() {
        let user = *users.at(i);
        start_cheat_caller_address(erc20_address, POOL_CREATOR);

        // Transfer tokens to user
        erc20.transfer(user, 1000 * ONE_STRK);
        stop_cheat_caller_address(erc20_address);

        start_cheat_caller_address(erc20_address, user);
        erc20.approve(contract_address, 1000 * ONE_STRK);
        stop_cheat_caller_address(erc20_address);
        i += 1;
    };
}

fn setup_pool_with_users(erc20_address: ContractAddress) -> u256 {
    let (contract, _, _, _, _) = deploy_predifi();
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();

    // Setup initial token distribution and approvals
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    pool_id
}

// Enhanced setup function with validator management
fn setup_pool_with_validators(erc20_address: ContractAddress) -> u256 {
    let (contract, _, validator_contract, _, _) = deploy_predifi();
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup tokens first
    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Setup initial token distribution for pool creator
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Add validators as admin BEFORE creating pool
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(VALIDATOR_ONE);
    validator_contract.add_validator(VALIDATOR_TWO);
    validator_contract.add_validator(VALIDATOR_THREE);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    pool_id
}

// Pool creation => User stakes => Dispute raised => Dispute resolution
#[test]
fn test_pool_creation_staking_dispute_resolution_flow() {
    let (contract, dispute_contract, _, _, erc20_address) = deploy_predifi();
    let mut spy = spy_events();

    // Use the same contract instance for consistency
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();

    // Setup initial token distribution and approvals
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // USER_ONE stakes to become validator
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.stake(pool_id, MIN_STAKE_AMOUNT);
    stop_cheat_caller_address(contract.contract_address);

    // Verify staking event
    let expected_stake_event = Predifi::Event::UserStaked(
        UserStaked { pool_id, address: USER_ONE, amount: MIN_STAKE_AMOUNT },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_stake_event)]);

    // USER_TWO votes on the pool
    start_cheat_caller_address(contract.contract_address, USER_TWO);
    contract.vote(pool_id, 'Team A', 500);
    stop_cheat_caller_address(contract.contract_address);

    // Advance time to locked state
    start_cheat_block_timestamp(contract.contract_address, 1710003601);
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Verify pool is locked
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Locked, 'Pool should be locked');

    // Raise disputes from multiple users
    let disputers = array![USER_ONE, USER_TWO, USER_THREE];
    let mut i = 0;
    while i != disputers.len() {
        let user = *disputers.at(i);
        start_cheat_caller_address(dispute_contract.contract_address, user);
        dispute_contract.raise_dispute(pool_id);
        stop_cheat_caller_address(dispute_contract.contract_address);
        i += 1;
    }

    // Verify pool suspension
    assert(dispute_contract.is_pool_suspended(pool_id), 'Pool should be suspended');
    let expected_suspend_event = Predifi::Event::PoolSuspended(
        PoolSuspended { pool_id, timestamp: get_block_timestamp() },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_suspend_event)]);

    // Admin resolves dispute
    start_cheat_caller_address(dispute_contract.contract_address, admin);
    dispute_contract.resolve_dispute(pool_id, true); // Set Team B as winner
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Verify dispute resolution
    let expected_resolve_event = Predifi::Event::DisputeResolved(
        DisputeResolved { pool_id, winning_option: true, timestamp: get_block_timestamp() },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_resolve_event)]);

    // Verify pool returns to locked state
    let resolved_pool = contract.get_pool(pool_id);
    assert(resolved_pool.status == Status::Locked, 'Should revert to Locked state');
}

// Pool creation > Voting > Validator assignment > Outcome validation > Pool settlement
#[test]
fn test_pool_voting_validation_settlement_flow() {
    let (contract, _, validator_contract, _, erc20_address) = deploy_predifi();
    let mut spy = spy_events();

    // Use consistent setup
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup tokens first
    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Setup initial token distribution for pool creator
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Add validators as admin BEFORE creating pool
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(VALIDATOR_ONE);
    validator_contract.add_validator(VALIDATOR_TWO);
    validator_contract.add_validator(VALIDATOR_THREE);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // User votes on the pool
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.vote(pool_id, 'Team A', 1000);
    stop_cheat_caller_address(contract.contract_address);

    // Advance time to locked state
    start_cheat_block_timestamp(contract.contract_address, 1710003601);
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Verify pool is locked
    let pool = contract.get_pool(pool_id);
    assert(pool.status == Status::Locked, 'Pool should be locked');

    // Get assigned validators
    let (validator1, validator2) = validator_contract.get_pool_validators(pool_id);
    let zero_address: ContractAddress = 0.try_into().unwrap();
    assert(validator1 != zero_address, 'Validator1 not assigned');
    assert(validator2 != zero_address, 'Validator2 not assigned');

    // Validators submit validation results
    start_cheat_caller_address(validator_contract.contract_address, validator1);
    validator_contract.validate_pool_result(pool_id, true); // Vote for Team B
    stop_cheat_caller_address(validator_contract.contract_address);

    start_cheat_caller_address(validator_contract.contract_address, validator2);
    validator_contract.validate_pool_result(pool_id, true); // Vote for Team B
    stop_cheat_caller_address(validator_contract.contract_address);

    // Verify automatic settlement
    let (validation_count, is_settled, outcome) = validator_contract
        .get_pool_validation_status(pool_id);
    assert(validation_count == 2, 'Should have 2 validations');
    assert(is_settled, 'Pool should be settled');
    assert(outcome, 'Outcome should be Team B (true)');

    // Verify settlement event
    let expected_settlement_event = Predifi::Event::PoolAutomaticallySettled(
        PoolAutomaticallySettled {
            pool_id, final_outcome: true, total_validations: 2, timestamp: get_block_timestamp(),
        },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_settlement_event)]);
}

// Pool creation > Voting > Dispute threshold reached > Suspension > Resolution => Settlement
#[test]
fn test_full_lifecycle_with_dispute_and_settlement() {
    let (contract, dispute_contract, _, _, erc20_address) = deploy_predifi();

    // Use consistent setup
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();

    // Setup initial token distribution and approvals
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // User votes
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.vote(pool_id, 'Team A', 500);
    stop_cheat_caller_address(contract.contract_address);

    // Advance time to locked state
    start_cheat_block_timestamp(contract.contract_address, 1710003601);
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Raise disputes to reach threshold
    let disputers = array![USER_ONE, USER_TWO, USER_THREE];
    let mut i = 0;
    while i != disputers.len() {
        let user = *disputers.at(i);
        start_cheat_caller_address(dispute_contract.contract_address, user);
        dispute_contract.raise_dispute(pool_id);
        stop_cheat_caller_address(dispute_contract.contract_address);
        i += 1;
    }

    // Verify suspension
    assert(dispute_contract.is_pool_suspended(pool_id), 'Pool should be suspended');

    // Admin resolves dispute
    start_cheat_caller_address(dispute_contract.contract_address, admin);
    dispute_contract.resolve_dispute(pool_id, false); // Set Team A as winner
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Advance time to end time for settlement
    start_cheat_block_timestamp(contract.contract_address, 1710007201);
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 2);
    stop_cheat_caller_address(contract.contract_address);
    stop_cheat_block_timestamp(contract.contract_address);

    // Claim rewards
    start_cheat_caller_address(dispute_contract.contract_address, USER_ONE);
    dispute_contract.claim_reward(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    // Verify final state
    let final_pool = contract.get_pool(pool_id);
    assert(final_pool.status == Status::Settled, 'Should be settled');
}

// Pool creation > Multiple validations > Consensus reached > Automatic settlement
#[test]
fn test_validator_consensus_with_conflicting_votes() {
    let (contract, _, validator_contract, _, erc20_address) = deploy_predifi();
    let mut spy = spy_events();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Set required confirmations to 3 so to test tie-breaking scenario
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.set_required_validator_confirmations(3);
    validator_contract.add_validator(VALIDATOR_ONE);
    validator_contract.add_validator(VALIDATOR_TWO);
    validator_contract.add_validator(VALIDATOR_THREE);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Setup tokens and approvals
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();
    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Setup initial token distribution for pool creator
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0, // WinBet
            "Test Description",
            "test.png",
            "test.com",
            1710000000, // start time
            1710003600, // lock time (1 hour later)
            1710007200, // end time (2 hours after lock)
            'Team A',
            'Team B',
            100,
            10000,
            5,
            false,
            0,
        );
    stop_cheat_caller_address(contract.contract_address);

    // Add some votes to make the pool valid for validation
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.vote(pool_id, 'Team A', 500);
    stop_cheat_caller_address(contract.contract_address);

    // Advance time to locked state
    start_cheat_block_timestamp(contract.contract_address, 1710003601);
    start_cheat_caller_address(contract.contract_address, admin);
    let new_status = contract.manually_update_pool_state(pool_id, 1);
    assert(new_status == Status::Locked, 'Should return Locked status');

    // First validator votes for Team B (true)
    start_cheat_caller_address(validator_contract.contract_address, VALIDATOR_ONE);
    validator_contract.validate_pool_result(pool_id, true); // Team B
    stop_cheat_caller_address(validator_contract.contract_address);

    // Pool should still be locked after first validation
    let pool_after_first = contract.get_pool(pool_id);
    assert(pool_after_first.status == Status::Locked, 'Pool should still be locked');

    // Second validator votes for Team A (false) - creates conflict
    start_cheat_caller_address(validator_contract.contract_address, VALIDATOR_TWO);
    validator_contract.validate_pool_result(pool_id, false); // Team A
    stop_cheat_caller_address(validator_contract.contract_address);

    // Pool should still be locked after second validation (tie situation)
    let pool_after_second = contract.get_pool(pool_id);
    assert(pool_after_second.status == Status::Locked, 'Pool still be locked after tie');

    // Third validator breaks the tie by voting for Team B (true)
    start_cheat_caller_address(validator_contract.contract_address, VALIDATOR_THREE);
    validator_contract.validate_pool_result(pool_id, true); // Team B - breaks tie
    stop_cheat_caller_address(validator_contract.contract_address);

    stop_cheat_block_timestamp(contract.contract_address);

    // Now verify consensus reached (2 votes for Team B, 1 for Team A)
    let (count, is_settled, outcome) = validator_contract.get_pool_validation_status(pool_id);
    assert(count == 3, 'Should have 3 validations');
    assert(is_settled, 'Should be settled');
    assert(outcome, 'Consensus should be Team B');

    // Verify automatic settlement event
    let expected_event = Predifi::Event::PoolAutomaticallySettled(
        PoolAutomaticallySettled {
            pool_id, final_outcome: true, total_validations: 3, timestamp: 1710003601,
        },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_event)]);
}

// Test validator consensus with default confirmations (2) and no conflicts
#[test]
fn test_validator_consensus_with_default_confirmations() {
    let (contract, _, validator_contract, _, erc20_address) = deploy_predifi();
    let mut spy = spy_events();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup tokens and approvals
    let users = array![USER_ONE, USER_TWO, VALIDATOR_ONE, VALIDATOR_TWO].span();
    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Setup initial token distribution for pool creator
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Add validators (default confirmations = 2)
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(VALIDATOR_ONE);
    validator_contract.add_validator(VALIDATOR_TWO);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0,
            "Test Description",
            "test.png",
            "test.com",
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
    stop_cheat_caller_address(contract.contract_address);

    // Add votes
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.vote(pool_id, 'Team A', 500);
    stop_cheat_caller_address(contract.contract_address);

    // Advance time and lock pool
    start_cheat_block_timestamp(contract.contract_address, 1710003601);
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);

    // First validator votes for Team B
    start_cheat_caller_address(validator_contract.contract_address, VALIDATOR_ONE);
    validator_contract.validate_pool_result(pool_id, true); // Team B
    stop_cheat_caller_address(validator_contract.contract_address);

    // Pool should still be locked (need 2 confirmations)
    let pool_after_first = contract.get_pool(pool_id);
    assert(pool_after_first.status == Status::Locked, 'Pool should still be locked');

    // Second validator also votes for Team B (creates consensus)
    start_cheat_caller_address(validator_contract.contract_address, VALIDATOR_TWO);
    validator_contract.validate_pool_result(pool_id, true); // Team B - consensus reached
    stop_cheat_caller_address(validator_contract.contract_address);

    stop_cheat_block_timestamp(contract.contract_address);

    // Verify consensus reached with 2 confirmations
    let (count, is_settled, outcome) = validator_contract.get_pool_validation_status(pool_id);
    assert(count == 2, 'Should have 2 validations');
    assert(is_settled, 'Should be settled');
    assert(outcome, 'Consensus should be Team B');

    // Verify automatic settlement event
    let expected_event = Predifi::Event::PoolAutomaticallySettled(
        PoolAutomaticallySettled {
            pool_id, final_outcome: true, total_validations: 2, timestamp: 1710003601,
        },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_event)]);
}

// Test validator with two confirmations
#[test]
fn test_validator_conflict_with_two_confirmations() {
    let (contract, _, validator_contract, _, erc20_address) = deploy_predifi();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup tokens and approvals
    let users = array![USER_ONE, VALIDATOR_ONE, VALIDATOR_TWO].span();
    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Add validators (default confirmations = 2)
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(VALIDATOR_ONE);
    validator_contract.add_validator(VALIDATOR_TWO);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = contract
        .create_pool(
            'Test Pool',
            0,
            "Test Description",
            "test.png",
            "test.com",
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
    stop_cheat_caller_address(contract.contract_address);

    // Add votes
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.vote(pool_id, 'Team A', 500);
    stop_cheat_caller_address(contract.contract_address);

    // Lock pool
    start_cheat_block_timestamp(contract.contract_address, 1710003601);
    start_cheat_caller_address(contract.contract_address, admin);
    contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);

    // First validator votes Team B
    start_cheat_caller_address(validator_contract.contract_address, VALIDATOR_ONE);
    validator_contract.validate_pool_result(pool_id, true); // Team B
    stop_cheat_caller_address(validator_contract.contract_address);

    // Second validator votes Team A (conflict)
    start_cheat_caller_address(validator_contract.contract_address, VALIDATOR_TWO);
    validator_contract.validate_pool_result(pool_id, false); // Team A
    stop_cheat_caller_address(validator_contract.contract_address);

    stop_cheat_block_timestamp(contract.contract_address);

    // Check what happens with conflicting votes
    let (count, is_settled, _) = validator_contract.get_pool_validation_status(pool_id);
    assert(count == 2, 'Should have 2 validations');

    // The pool should settle based on the consensus calculation
    // This will depend on how calculate_validation_consensus works
    // It might use the first vote, last vote, or some other tie-breaking logic
    assert(is_settled, 'settled even with conflict');
}

//  Pool creation => Staking => Pool cancellation => Stake refund
// This tests the flow where a pool is cancelled after users have staked
// and verifies that the stake is properly refunded to the users.
#[test]
fn test_pool_cancellation_and_stake_refund() {
    let (contract, _, _, _, erc20_address) = deploy_predifi();
    let mut spy = spy_events();

    // Use consistent setup
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();

    // Setup initial token distribution and approvals
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    let erc20 = IERC20Dispatcher { contract_address: erc20_address };

    // USER_ONE stakes on the pool
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.stake(pool_id, MIN_STAKE_AMOUNT);
    stop_cheat_caller_address(contract.contract_address);

    // Record initial balance
    let initial_balance = erc20.balance_of(USER_ONE);

    // Pool creator cancels the pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    contract.cancel_pool(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Verify cancellation
    let cancelled_pool = contract.get_pool(pool_id);
    assert(cancelled_pool.status == Status::Closed, 'Pool should be closed');

    // Verify cancellation event
    let expected_cancel_event = Predifi::Event::PoolCancelled(
        PoolCancelled { pool_id, timestamp: get_block_timestamp() },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_cancel_event)]);

    // Refund stake
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.refund_stake(pool_id);
    stop_cheat_caller_address(contract.contract_address);

    // Verify refund
    let final_balance = erc20.balance_of(USER_ONE);
    assert(final_balance == initial_balance + MIN_STAKE_AMOUNT, 'Stake not properly refunded');

    // Verify refund event
    let expected_refund_event = Predifi::Event::StakeRefunded(
        StakeRefunded { pool_id, address: USER_ONE, amount: MIN_STAKE_AMOUNT },
    );
    spy.assert_emitted(@array![(contract.contract_address, expected_refund_event)]);
}

//  Pool creation > Multiple users vote > Time progression > Settlement
// This tests the natural settlement flow where multiple users vote and the pool is automatically
// settled
#[test]
fn test_multiple_users_voting_and_natural_settlement() {
    let (contract, dispute_contract, validator_contract, _, erc20_address) = deploy_predifi();

    // Use consistent setup
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup tokens first
    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Setup initial token distribution for pool creator
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Add validators as admin BEFORE creating pool
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(VALIDATOR_ONE);
    validator_contract.add_validator(VALIDATOR_TWO);
    validator_contract.add_validator(VALIDATOR_THREE);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Multiple users vote on different options
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.vote(pool_id, 'Team A', 1000);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, USER_TWO);
    contract.vote(pool_id, 'Team B', 800);
    stop_cheat_caller_address(contract.contract_address);

    start_cheat_caller_address(contract.contract_address, USER_THREE);
    contract.vote(pool_id, 'Team A', 500);
    stop_cheat_caller_address(contract.contract_address);

    // Advance time to locked state
    start_cheat_block_timestamp(contract.contract_address, 1710003601);
    start_cheat_caller_address(contract.contract_address, admin);
    let status = contract.manually_update_pool_state(pool_id, 1);
    stop_cheat_caller_address(contract.contract_address);
    assert(status == Status::Locked, 'Should be locked');
    stop_cheat_block_timestamp(contract.contract_address);

    // Validators validate results
    let (validator1, validator2) = validator_contract.get_pool_validators(pool_id);

    start_cheat_caller_address(validator_contract.contract_address, validator1);
    validator_contract.validate_pool_result(pool_id, false); // Team A wins
    stop_cheat_caller_address(validator_contract.contract_address);

    start_cheat_caller_address(validator_contract.contract_address, validator2);
    validator_contract.validate_pool_result(pool_id, false); // Team A wins
    stop_cheat_caller_address(validator_contract.contract_address);

    // Verify settlement
    let (count, is_settled, outcome) = validator_contract.get_pool_validation_status(pool_id);
    assert(count == 2, 'Should have 2 validations');
    assert(is_settled, 'Should be automatically settled');
    assert(!outcome, 'Team A should win (false)');

    // Advance to settlement time
    start_cheat_block_timestamp(contract.contract_address, 1710007201);
    start_cheat_caller_address(contract.contract_address, admin);
    let final_status = contract.manually_update_pool_state(pool_id, 2);
    stop_cheat_caller_address(contract.contract_address);
    assert(final_status == Status::Settled, 'Should be settled');
    stop_cheat_block_timestamp(contract.contract_address);

    // Winners can claim rewards
    start_cheat_caller_address(dispute_contract.contract_address, USER_ONE);
    dispute_contract.claim_reward(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);

    start_cheat_caller_address(dispute_contract.contract_address, USER_THREE);
    dispute_contract.claim_reward(pool_id);
    stop_cheat_caller_address(dispute_contract.contract_address);
}


#[test]
#[should_panic(expected: 'INVALID NUMBER IS ZERO')]
fn test_voting_on_invalid_amount() {
    let (contract, dispute_contract, validator_contract, _, erc20_address) = deploy_predifi();

    // Use consistent setup
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup tokens first
    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Setup initial token distribution for pool creator
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Add validators as admin BEFORE creating pool
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(VALIDATOR_ONE);
    validator_contract.add_validator(VALIDATOR_TWO);
    validator_contract.add_validator(VALIDATOR_THREE);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Multiple users vote on different options with some Invalid params

    // Invalid amount
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.vote(pool_id, 'Team A', 0);
    stop_cheat_caller_address(contract.contract_address);
    // Invalid option
    start_cheat_caller_address(contract.contract_address, USER_TWO);
    contract.vote(pool_id, '', 800);
    stop_cheat_caller_address(contract.contract_address);

    // not existed pool id
    start_cheat_caller_address(contract.contract_address, USER_THREE);
    contract.vote(pool_id + 21, 'Team A', 500);
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
#[should_panic(expected: 'EMPTY FELT252')]
fn test_voting_on_empty_option() {
    let (contract, dispute_contract, validator_contract, _, erc20_address) = deploy_predifi();

    // Use consistent setup
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup tokens first
    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Setup initial token distribution for pool creator
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Add validators as admin BEFORE creating pool
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(VALIDATOR_ONE);
    validator_contract.add_validator(VALIDATOR_TWO);
    validator_contract.add_validator(VALIDATOR_THREE);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // Invalid option
    start_cheat_caller_address(contract.contract_address, USER_TWO);
    contract.vote(pool_id, '', 800);
    stop_cheat_caller_address(contract.contract_address);
}

#[test]
#[should_panic(expected: 'Pool does not exist')]
fn test_voting_on_invalid_pool() {
    let (contract, dispute_contract, validator_contract, _, erc20_address) = deploy_predifi();

    // Use consistent setup
    let users = array![
        USER_ONE, USER_TWO, USER_THREE, VALIDATOR_ONE, VALIDATOR_TWO, VALIDATOR_THREE,
    ]
        .span();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Setup tokens first
    setup_tokens_and_approvals(erc20_address, contract.contract_address, users);

    // Setup initial token distribution for pool creator
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, POOL_CREATOR);
    erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
    stop_cheat_caller_address(erc20_address);

    // Add validators as admin BEFORE creating pool
    start_cheat_caller_address(validator_contract.contract_address, admin);
    validator_contract.add_validator(VALIDATOR_ONE);
    validator_contract.add_validator(VALIDATOR_TWO);
    validator_contract.add_validator(VALIDATOR_THREE);
    stop_cheat_caller_address(validator_contract.contract_address);

    // Create pool
    start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
    let pool_id = create_default_pool(contract);
    stop_cheat_caller_address(contract.contract_address);

    // not existed pool id
    start_cheat_caller_address(contract.contract_address, USER_THREE);
    contract.vote(pool_id + 21, 'Team A', 500);
    stop_cheat_caller_address(contract.contract_address);
}
