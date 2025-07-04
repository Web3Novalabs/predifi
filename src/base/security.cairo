use starknet::ContractAddress;
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
use starknet::get_block_timestamp;

use crate::base::types::{PoolDetails, Status};
use crate::base::errors::Errors;

// Constants
const ONE_STRK: u256 = 1_000_000_000_000_000_000;
const MIN_STAKE_AMOUNT: u256 = 200_000_000_000_000_000_000;

/// @notice SecurityTrait defines all security assertions for the PrediFi contract
/// @dev This trait centralizes validation logic to improve code clarity and maintainability
pub trait SecurityTrait<TContractState> {
    // Pool existence and status assertions
    /// @notice Asserts that a pool exists
    /// @param pool The pool details to check
    fn assert_pool_exists(self: @TContractState, pool: @PoolDetails);
    
    /// @notice Asserts that a pool is in Active status
    /// @param pool The pool details to check
    fn assert_pool_active(self: @TContractState, pool: @PoolDetails);
    
    /// @notice Asserts that a pool is not suspended
    /// @param pool The pool details to check
    fn assert_pool_not_suspended(self: @TContractState, pool: @PoolDetails);
    
    /// @notice Asserts that a pool is in Locked status
    /// @param pool The pool details to check
    fn assert_pool_locked(self: @TContractState, pool: @PoolDetails);
    
    /// @notice Asserts that a pool is in Closed status
    /// @param pool The pool details to check
    fn assert_pool_closed(self: @TContractState, pool: @PoolDetails);
    
    /// @notice Asserts that a pool is in Suspended status
    /// @param pool The pool details to check
    fn assert_pool_suspended(self: @TContractState, pool: @PoolDetails);
    
    /// @notice Asserts that a pool is ready for validation (Locked status)
    /// @param pool The pool details to check
    fn assert_pool_ready_for_validation(self: @TContractState, pool: @PoolDetails);
    
    // Pool ownership assertions
    /// @notice Asserts that the caller is the pool owner
    /// @param pool The pool details to check
    /// @param caller The caller's address
    fn assert_pool_owner(self: @TContractState, pool: @PoolDetails, caller: ContractAddress);
    
    // Amount validation assertions
    /// @notice Asserts that amount is within specified limits
    /// @param amount The amount to check
    /// @param min_amount The minimum allowed amount
    /// @param max_amount The maximum allowed amount
    fn assert_amount_within_limits(self: @TContractState, amount: u256, min_amount: u256, max_amount: u256);
    
    /// @notice Asserts that stake amount meets minimum requirements
    /// @param amount The stake amount to check
    fn assert_min_stake_amount(self: @TContractState, amount: u256);
    
    /// @notice Asserts that user has sufficient balance
    /// @param token The token dispatcher
    /// @param user The user's address
    /// @param amount The required amount
    fn assert_sufficient_balance(self: @TContractState, token: IERC20Dispatcher, user: ContractAddress, amount: u256);
    
    /// @notice Asserts that user has sufficient allowance for spender
    /// @param token The token dispatcher
    /// @param user The user's address
    /// @param spender The spender's address
    /// @param amount The required amount
    fn assert_sufficient_allowance(self: @TContractState, token: IERC20Dispatcher, user: ContractAddress, spender: ContractAddress, amount: u256);
    
    /// @notice Asserts that creator meets pool creation fee requirements
    /// @param token The token dispatcher
    /// @param creator The creator's address
    /// @param contract_address The contract address
    fn assert_pool_creation_fee_requirements(self: @TContractState, token: IERC20Dispatcher, creator: ContractAddress, contract_address: ContractAddress);
    
    /// @notice Asserts that stake amount is non-zero
    /// @param amount The amount to check
    fn assert_non_zero_stake(self: @TContractState, amount: u256);
    
    // Pool timing assertions
    /// @notice Asserts that pool timing is valid
    /// @param start_time The pool start time
    /// @param lock_time The pool lock time
    /// @param end_time The pool end time
    fn assert_valid_pool_timing(self: @TContractState, start_time: u64, lock_time: u64, end_time: u64);
    
    /// @notice Asserts that start time is in the future
    /// @param start_time The start time to check
    fn assert_future_start_time(self: @TContractState, start_time: u64);
    
    // Pool configuration assertions
    /// @notice Asserts that bet amounts are valid
    /// @param min_bet The minimum bet amount
    /// @param max_bet The maximum bet amount
    fn assert_valid_bet_amounts(self: @TContractState, min_bet: u256, max_bet: u256);
    
    /// @notice Asserts that creator fee is within acceptable range
    /// @param fee The creator fee percentage
    fn assert_valid_creator_fee(self: @TContractState, fee: u8);
    
    /// @notice Asserts that the selected option is valid
    /// @param option The selected option
    /// @param option1 The first valid option
    /// @param option2 The second valid option
    fn assert_valid_pool_option(self: @TContractState, option: felt252, option1: felt252, option2: felt252);
    
    /// @notice Asserts that count is positive
    /// @param count The count to check
    fn assert_positive_count(self: @TContractState, count: u256);
    
    // Validator and dispute assertions
    /// @notice Asserts that validator has not already validated this pool
    /// @param has_validated Whether the validator has already validated
    fn assert_validator_not_already_validated(self: @TContractState, has_validated: bool);
    
    /// @notice Asserts that user has not already disputed this pool
    /// @param has_disputed Whether the user has already disputed
    fn assert_user_has_not_disputed(self: @TContractState, has_disputed: bool);
    
    // State transition assertions
    /// @notice Asserts that state transition is valid
    /// @param current_status The current pool status
    /// @param new_status The new pool status
    /// @param is_admin Whether the caller is an admin
    fn assert_valid_state_transition(self: @TContractState, current_status: Status, new_status: Status, is_admin: bool);
}

/// @notice Implementation of SecurityTrait
/// @dev Provides centralized assertion logic for the PrediFi contract
pub impl Security<TContractState> of SecurityTrait<TContractState> {
    /// @notice Asserts that a pool exists
    fn assert_pool_exists(self: @TContractState, pool: @PoolDetails) {
        assert(*pool.exists, Errors::POOL_DOES_NOT_EXIST);
    }
    
    /// @notice Asserts that a pool is in Active status
    fn assert_pool_active(self: @TContractState, pool: @PoolDetails) {
        assert(*pool.status == Status::Active, Errors::INACTIVE_POOL);
    }
    
    /// @notice Asserts that a pool is not suspended
    fn assert_pool_not_suspended(self: @TContractState, pool: @PoolDetails) {
        assert(*pool.status != Status::Suspended, Errors::POOL_SUSPENDED);
    }
    
    /// @notice Asserts that a pool is in Locked status
    fn assert_pool_locked(self: @TContractState, pool: @PoolDetails) {
        assert(*pool.status == Status::Locked, Errors::POOL_NOT_LOCKED);
    }
    
    /// @notice Asserts that a pool is in Closed status
    fn assert_pool_closed(self: @TContractState, pool: @PoolDetails) {
        assert(*pool.status == Status::Closed, Errors::POOL_NOT_CLOSED);
    }
    
    /// @notice Asserts that a pool is in Suspended status
    fn assert_pool_suspended(self: @TContractState, pool: @PoolDetails) {
        assert(*pool.status == Status::Suspended, Errors::POOL_NOT_SUSPENDED);
    }
    
    /// @notice Asserts that a pool is ready for validation (Locked status)
    fn assert_pool_ready_for_validation(self: @TContractState, pool: @PoolDetails) {
        assert(*pool.status == Status::Locked, Errors::POOL_NOT_READY_FOR_VALIDATION);
    }
    
    /// @notice Asserts that the caller is the pool owner
    fn assert_pool_owner(self: @TContractState, pool: @PoolDetails, caller: ContractAddress) {
        assert(caller == *pool.address, Errors::UNAUTHORIZED_CALLER);
    }
    
    /// @notice Asserts that amount is within specified limits
    fn assert_amount_within_limits(self: @TContractState, amount: u256, min_amount: u256, max_amount: u256) {
        assert(amount >= min_amount, Errors::AMOUNT_BELOW_MINIMUM);
        assert(amount <= max_amount, Errors::AMOUNT_ABOVE_MAXIMUM);
    }
    
    /// @notice Asserts that stake amount meets minimum requirements
    fn assert_min_stake_amount(self: @TContractState, amount: u256) {
        assert(amount >= MIN_STAKE_AMOUNT, Errors::STAKE_AMOUNT_TOO_LOW);
    }
    
    /// @notice Asserts that user has sufficient balance
    fn assert_sufficient_balance(self: @TContractState, token: IERC20Dispatcher, user: ContractAddress, amount: u256) {
        let user_balance = token.balance_of(user);
        assert(user_balance >= amount, Errors::INSUFFICIENT_BALANCE);
    }
    
    /// @notice Asserts that user has sufficient allowance for spender
    fn assert_sufficient_allowance(self: @TContractState, token: IERC20Dispatcher, user: ContractAddress, spender: ContractAddress, amount: u256) {
        let allowed_amount = token.allowance(user, spender);
        assert(allowed_amount >= amount, Errors::INSUFFICIENT_ALLOWANCE);
    }
    
    /// @notice Asserts that creator meets pool creation fee requirements
    fn assert_pool_creation_fee_requirements(self: @TContractState, token: IERC20Dispatcher, creator: ContractAddress, contract_address: ContractAddress) {
        let creator_balance = token.balance_of(creator);
        assert(creator_balance >= ONE_STRK, Errors::INSUFFICIENT_STRK_BALANCE);
        
        let allowed_amount = token.allowance(creator, contract_address);
        assert(allowed_amount >= ONE_STRK, Errors::INSUFFICIENT_ALLOWANCE);
    }
    
    /// @notice Asserts that stake amount is non-zero
    fn assert_non_zero_stake(self: @TContractState, amount: u256) {
        assert(amount > 0, Errors::ZERO_USER_STAKE);
    }
    
    /// @notice Asserts that pool timing is valid
    fn assert_valid_pool_timing(self: @TContractState, start_time: u64, lock_time: u64, end_time: u64) {
        assert(start_time < lock_time, Errors::INVALID_LOCK_TIME);
        assert(lock_time < end_time, Errors::INVALID_LOCK_TIME_TO_END_TIME);
    }
    
    /// @notice Asserts that start time is in the future
    fn assert_future_start_time(self: @TContractState, start_time: u64) {
        let current_time = get_block_timestamp();
        assert(current_time < start_time, Errors::INVALID_START_TIME);
    }
    
    /// @notice Asserts that bet amounts are valid
    fn assert_valid_bet_amounts(self: @TContractState, min_bet: u256, max_bet: u256) {
        assert(min_bet > 0, Errors::ZERO_MINIMUM_BET);
        assert(max_bet >= min_bet, Errors::INVALID_MAXIMUM_BET);
    }
    
    /// @notice Asserts that creator fee is within acceptable range
    fn assert_valid_creator_fee(self: @TContractState, fee: u8) {
        assert(fee <= 5, Errors::CREATOR_FEE_TOO_HIGH);
    }
    
    /// @notice Asserts that the selected option is valid
    fn assert_valid_pool_option(self: @TContractState, option: felt252, option1: felt252, option2: felt252) {
        assert(option == option1 || option == option2, Errors::INVALID_POOL_OPTION);
    }
    
    /// @notice Asserts that count is positive
    fn assert_positive_count(self: @TContractState, count: u256) {
        assert(count > 0, Errors::COUNT_MUST_BE_GREATER_THAN_ZERO);
    }
    
    /// @notice Asserts that validator has not already validated this pool
    fn assert_validator_not_already_validated(self: @TContractState, has_validated: bool) {
        assert(!has_validated, Errors::VALIDATOR_ALREADY_VALIDATED);
    }
    
    /// @notice Asserts that user has not already disputed this pool
    fn assert_user_has_not_disputed(self: @TContractState, has_disputed: bool) {
        assert(!has_disputed, Errors::DISPUTE_ALREADY_RAISED);
    }
    
    /// @notice Asserts that state transition is valid
    fn assert_valid_state_transition(self: @TContractState, current_status: Status, new_status: Status, is_admin: bool) {
        // Don't update if status is the same
        if new_status == current_status {
            return;
        }
        
        // Check for invalid transitions
        let is_valid_transition = if is_admin {
            !(current_status == Status::Locked && new_status == Status::Active)
                && !(current_status == Status::Settled
                    && (new_status == Status::Active || new_status == Status::Locked))
                && !(current_status == Status::Closed)
        } else {
            // Active -> Locked -> Settled -> Closed
            (current_status == Status::Active && new_status == Status::Locked)
                || (current_status == Status::Locked && new_status == Status::Settled)
                || (current_status == Status::Settled && new_status == Status::Closed)
        };
        
        assert(is_valid_transition, Errors::INVALID_STATE_TRANSITION);
    }
} 