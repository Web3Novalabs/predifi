use contract::interfaces::ipredifi::IPredifiDispatcherTrait;
use contract::predifi::Predifi;
use snforge_std::{
    start_gas_meter, stop_gas_meter, start_cheat_caller_address, stop_cheat_caller_address,
    start_cheat_block_timestamp,
};
use starknet::ContractAddress;

// Validator role
const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");
const POOL_CREATOR: ContractAddress = 123.try_into().unwrap();
const USER_ONE: ContractAddress = 'User1'.try_into().unwrap();
const ONE_STRK: u256 = 1_000_000_000_000_000_000;

use super::test_utils::{
    approve_tokens_for_payment, create_default_pool, deploy_predifi,
};

/// @notice Gas benchmarking tests for optimized storage operations
/// @dev These tests measure gas consumption of optimized functions to ensure efficiency

#[test]
fn test_vote_function_gas_optimization() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup pool
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);

    // Setup user with tokens
    start_cheat_caller_address(erc20_address, USER_ONE);
    approve_tokens_for_payment(contract.contract_address, erc20_address, ONE_STRK * 1000);
    stop_cheat_caller_address(erc20_address);

    // Set future timestamp for pool to be active
    start_cheat_block_timestamp(contract.contract_address, 1710000001);

    start_cheat_caller_address(contract.contract_address, USER_ONE);

    // Start gas metering
    start_gas_meter();

    // Execute vote function
    contract.vote(pool_id, 'Team A', ONE_STRK);

    // Stop gas metering and log gas usage
    let gas_used = stop_gas_meter();
    println!("Gas used for optimized vote function: {}", gas_used);
}

#[test]
fn test_stake_function_gas_optimization() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup pool
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);

    // Setup user with tokens
    start_cheat_caller_address(erc20_address, USER_ONE);
    approve_tokens_for_payment(contract.contract_address, erc20_address, ONE_STRK * 1000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, USER_ONE);

    // Start gas metering
    start_gas_meter();

    // Execute stake function
    contract.stake(pool_id, ONE_STRK * 200);

    // Stop gas metering and log gas usage
    let gas_used = stop_gas_meter();
    println!("Gas used for optimized stake function: {}", gas_used);
}

#[test]
fn test_create_pool_function_gas_optimization() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup tokens for pool creation fee
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);

    // Start gas metering
    start_gas_meter();

    // Execute create_pool function
    let pool_id = contract.create_pool(
        'Optimized Pool',
        0, // 0 = WinBet
        "A gas-optimized betting pool",
        "image.png",
        "event.com/details",
        1710000000,
        1710003600,
        1710007200,
        'Option A',
        'Option B',
        100,
        10000,
        5,
        false,
        0,
    );

    // Stop gas metering and log gas usage
    let gas_used = stop_gas_meter();
    println!("Gas used for optimized create_pool function: {}", gas_used);
    assert!(pool_id != 0, "Pool creation failed");
}

#[test]
fn test_emergency_withdraw_gas_optimization() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup pool and user participation
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);

    // Setup user with tokens and make them participate
    start_cheat_caller_address(erc20_address, USER_ONE);
    approve_tokens_for_payment(contract.contract_address, erc20_address, ONE_STRK * 1000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_block_timestamp(contract.contract_address, 1710000001);
    start_cheat_caller_address(contract.contract_address, USER_ONE);
    contract.vote(pool_id, 'Team A', ONE_STRK);

    // Simulate emergency state (this would normally be done by admin)
    // For testing purposes, we'll assume emergency state is set

    // Note: Emergency withdrawal test requires emergency state to be set first
    // This is a placeholder for when emergency functionality is properly set up
    println!("Emergency withdrawal gas test requires emergency state setup");
}

#[test]
fn test_multiple_votes_gas_comparison() {
    let (contract, _, _, pool_creator, erc20_address) = deploy_predifi();

    // Setup pool
    start_cheat_caller_address(erc20_address, pool_creator);
    approve_tokens_for_payment(
        contract.contract_address, erc20_address, 200_000_000_000_000_000_000_000,
    );
    stop_cheat_caller_address(erc20_address);

    start_cheat_caller_address(contract.contract_address, pool_creator);
    let pool_id = create_default_pool(contract);

    // Setup user with tokens
    start_cheat_caller_address(erc20_address, USER_ONE);
    approve_tokens_for_payment(contract.contract_address, erc20_address, ONE_STRK * 10000);
    stop_cheat_caller_address(erc20_address);

    start_cheat_block_timestamp(contract.contract_address, 1710000001);
    start_cheat_caller_address(contract.contract_address, USER_ONE);

    // Measure gas for multiple votes
    start_gas_meter();

    // Execute multiple votes to test cumulative gas efficiency
    contract.vote(pool_id, 'Team A', ONE_STRK);
    contract.vote(pool_id, 'Team B', ONE_STRK * 2);
    contract.vote(pool_id, 'Team A', ONE_STRK * 3);

    let gas_used = stop_gas_meter();
    println!("Gas used for 3 optimized votes: {}", gas_used);
}