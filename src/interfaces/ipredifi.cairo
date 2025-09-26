use core::byte_array::ByteArray;
use core::integer::u256;
use starknet::{ClassHash, ContractAddress};
use crate::base::types::{PoolDetails, PoolOdds, Status, UserStake};

#[starknet::interface]
pub trait IPredifi<TContractState> {
    // Pool Creation and Management

    /// @notice Creates a new prediction pool.
    /// @dev Validates parameters, collects pool creation fee, and assigns validators.
    /// @param poolName The name of the pool.
    /// @param poolType The type of the pool (as u8).
    /// @param poolDescription The description of the pool.
    /// @param poolImage The image URL for the pool.
    /// @param poolEventSourceUrl The event source URL.
    /// @param poolStartTime The start time of the pool.
    /// @param poolLockTime The lock time of the pool.
    /// @param poolEndTime The end time of the pool.
    /// @param option1 The first option for the pool.
    /// @param option2 The second option for the pool.
    /// @param minBetAmount The minimum bet amount.
    /// @param maxBetAmount The maximum bet amount.
    /// @param creatorFee The fee percentage for the pool creator.
    /// @param isPrivate Whether the pool is private.
    /// @param category The category of the pool.
    /// @return The unique pool ID.
    fn create_pool(
        ref self: TContractState,
        poolName: felt252,
        poolType: u8,
        poolDescription: ByteArray,
        poolImage: ByteArray,
        poolEventSourceUrl: ByteArray,
        poolStartTime: u64,
        poolLockTime: u64,
        poolEndTime: u64,
        option1: felt252,
        option2: felt252,
        minBetAmount: u256,
        maxBetAmount: u256,
        creatorFee: u8,
        isPrivate: bool,
        category: u8,
    ) -> u256;

    /// @notice Cancels a pool. Only the pool creator can cancel.
    /// @dev Emits a PoolCancelled event.
    /// @param pool_id The ID of the pool to cancel.
    fn cancel_pool(ref self: TContractState, pool_id: u256);

    /// @notice Returns the total number of pools.
    /// @return The pool count.
    fn pool_count(self: @TContractState) -> u256;

    /// @notice Returns the odds for a given pool.
    /// @param pool_id The pool ID.
    /// @return The PoolOdds struct.
    fn pool_odds(self: @TContractState, pool_id: u256) -> PoolOdds;

    /// @notice Returns the details of a given pool.
    /// @param pool_id The pool ID.
    /// @return The PoolDetails struct.
    fn get_pool(self: @TContractState, pool_id: u256) -> PoolDetails;

    /// @notice Places a bet on a pool.
    /// @dev Transfers tokens from user, updates odds, and emits BetPlaced event.
    /// @param pool_id The pool ID.
    /// @param option The option to bet on.
    /// @param amount The amount to bet.
    fn vote(ref self: TContractState, pool_id: u256, option: felt252, amount: u256);

    /// @notice Stakes tokens to become a validator for a pool.
    /// @dev Transfers tokens, grants validator role, and emits UserStaked event.
    /// @param pool_id The pool ID.
    /// @param amount The amount to stake.
    fn stake(ref self: TContractState, pool_id: u256, amount: u256);

    /// @notice Refunds the user's stake for a closed pool.
    /// @dev Transfers tokens back to user and emits StakeRefunded event.
    /// @param pool_id The pool ID.
    fn refund_stake(ref self: TContractState, pool_id: u256);

    /// @notice Returns the user's stake for a given pool.
    /// @param pool_id The pool ID.
    /// @param address The user's address.
    /// @return The UserStake struct.
    fn get_user_stake(self: @TContractState, pool_id: u256, address: ContractAddress) -> UserStake;

    /// @notice Returns the stake for a given pool.
    /// @param pool_id The pool ID.
    /// @return The UserStake struct.
    fn get_pool_stakes(self: @TContractState, pool_id: u256) -> UserStake;

    /// @notice Returns the vote for a given pool.
    /// @param pool_id The pool ID.
    /// @return True if option2, false if option1.
    fn get_pool_vote(self: @TContractState, pool_id: u256) -> bool;

    /// @notice Returns the total pool count.
    /// @return The pool count.
    fn get_pool_count(self: @TContractState) -> u256;

    /// @notice Returns true if the pool exists.
    /// @param pool_id The pool ID.
    /// @return True if the pool exists.
    fn retrieve_pool(self: @TContractState, pool_id: u256) -> bool;

    /// @notice Returns the creator address of a given pool.
    /// @param pool_id The pool ID.
    /// @return The creator's contract address.
    fn get_pool_creator(self: @TContractState, pool_id: u256) -> ContractAddress;

    /// @notice Returns the creator fee percentage for a pool.
    /// @param pool_id The pool ID.
    /// @return The creator fee percentage.
    fn get_creator_fee_percentage(self: @TContractState, pool_id: u256) -> u8;

    /// @notice Collects the pool creation fee from the creator.
    /// @dev Transfers 1 STRK from creator to contract.
    /// @param creator The creator's address.
    /// @param pool_id The pool ID for which the fee is being collected.
    fn collect_pool_creation_fee(ref self: TContractState, creator: ContractAddress, pool_id: u256);

    /// @notice Manually updates the state of a pool.
    /// @dev Only callable by admin or validator. Enforces valid state transitions.
    /// @param pool_id The pool ID.
    /// @param new_status The new status to set.
    /// @return The updated status.
    fn manually_update_pool_state(
        ref self: TContractState, pool_id: u256, new_status: u8,
    ) -> Status;

    /// @notice Returns the number of pools a user has participated in.
    /// @param user The user's address.
    /// @return The number of pools.
    fn get_user_pool_count(self: @TContractState, user: ContractAddress) -> u256;

    /// @notice Checks if a user has participated in a specific pool.
    /// @param user The user's address.
    /// @param pool_id The pool ID.
    /// @return True if participated, false otherwise.
    fn check_user_participated(self: @TContractState, user: ContractAddress, pool_id: u256) -> bool;

    /// @notice Returns a list of pool IDs the user has participated in, filtered by status.
    /// @param user The user's address.
    /// @param status_filter Optional status filter.
    /// @return Array of pool IDs.
    fn get_user_pools(
        self: @TContractState, user: ContractAddress, status_filter: Option<Status>,
    ) -> Array<u256>;

    /// @notice Returns whether a user has participated in a specific pool.
    /// @param user The user's address.
    /// @param pool_id The pool ID.
    /// @return True if the user has participated, false otherwise.
    fn has_user_participated_in_pool(
        self: @TContractState, user: ContractAddress, pool_id: u256,
    ) -> bool;

    /// @notice Returns a list of active pools the user has participated in.
    /// @param user The user's address.
    /// @return Array of pool IDs.
    fn get_user_active_pools(self: @TContractState, user: ContractAddress) -> Array<u256>;

    /// @notice Returns a list of locked pools the user has participated in.
    /// @param user The user's address.
    /// @return Array of pool IDs.
    fn get_user_locked_pools(self: @TContractState, user: ContractAddress) -> Array<u256>;

    /// @notice Returns a list of settled pools the user has participated in.
    /// @param user The user's address.
    /// @return Array of pool IDs.
    fn get_user_settled_pools(self: @TContractState, user: ContractAddress) -> Array<u256>;

    // Functions for filtering pools by status

    /// @notice Returns all active pools.
    /// @return Array of PoolDetails.
    fn get_active_pools(self: @TContractState) -> Array<PoolDetails>;

    /// @notice Returns all locked pools.
    /// @return Array of PoolDetails.
    fn get_locked_pools(self: @TContractState) -> Array<PoolDetails>;

    /// @notice Returns all settled pools.
    /// @return Array of PoolDetails.
    fn get_settled_pools(self: @TContractState) -> Array<PoolDetails>;

    /// @notice Returns all closed pools.
    /// @return Array of PoolDetails.
    fn get_closed_pools(self: @TContractState) -> Array<PoolDetails>;

    // Emergency Functions

    /// @notice Emergency withdrawal function for problematic pools.
    /// @dev Allows users to withdraw funds from pools in emergency state.
    /// @param pool_id The pool ID to withdraw from.
    fn emergency_withdraw(ref self: TContractState, pool_id: u256);

    /// @notice Schedules an emergency action with timelock.
    /// @dev Only callable by admin. Schedules emergency action for execution after delay.
    /// @param action_type The type of emergency action.
    /// @param pool_id The pool ID for the action.
    /// @param action_data Additional data for the action.
    /// @return The unique action ID.
    fn schedule_emergency_action(
        ref self: TContractState, action_type: u8, pool_id: u256, action_data: felt252,
    ) -> u256;

    /// @notice Executes a scheduled emergency action after timelock delay.
    /// @dev Only callable by admin. Executes action if delay has passed.
    /// @param action_id The ID of the scheduled action.
    fn execute_emergency_action(ref self: TContractState, action_id: u256);

    /// @notice Cancels a scheduled emergency action.
    /// @dev Only callable by admin. Cancels action before execution.
    /// @param action_id The ID of the scheduled action.
    fn cancel_emergency_action(ref self: TContractState, action_id: u256);

    /// @notice Returns the status of a scheduled emergency action.
    /// @param action_id The ID of the scheduled action.
    /// @return The action status and execution time.
    fn get_emergency_action_status(self: @TContractState, action_id: u256) -> (u8, u64);

    /// @notice Returns whether a pool is in emergency state.
    /// @param pool_id The pool ID.
    /// @return True if pool is in emergency state, false otherwise.
    fn is_pool_emergency_state(self: @TContractState, pool_id: u256) -> bool;

    /// @notice Returns all pools in emergency state.
    /// @return Array of PoolDetails for emergency pools.
    fn get_emergency_pools(self: @TContractState) -> Array<PoolDetails>;
}

#[starknet::interface]
pub trait IPredifiDispute<TContractState> {
    //dispute functionality

    /// @notice Raises a dispute for a pool.
    /// @dev Emits DisputeRaised and may suspend the pool if threshold is met.
    /// @param pool_id The pool ID.
    fn raise_dispute(ref self: TContractState, pool_id: u256);

    /// @notice Resolves a dispute and restores pool status.
    /// @dev Only callable by admin. Emits DisputeResolved and PoolStateTransition.
    /// @param pool_id The pool ID.
    /// @param winning_option The winning option (true = option2, false = option1).
    fn resolve_dispute(ref self: TContractState, pool_id: u256, winning_option: bool);

    /// @notice Returns the dispute count for a pool.
    /// @param pool_id The pool ID.
    /// @return The dispute count.
    fn get_dispute_count(self: @TContractState, pool_id: u256) -> u256;

    /// @notice Returns the dispute threshold.
    /// @return The dispute threshold.
    fn get_dispute_threshold(self: @TContractState) -> u256;

    /// @notice Returns whether a user has disputed a pool.
    /// @param pool_id The pool ID.
    /// @param user The user's address.
    /// @return True if user has disputed, false otherwise.
    fn has_user_disputed(self: @TContractState, pool_id: u256, user: ContractAddress) -> bool;

    /// @notice Returns whether a pool is suspended.
    /// @param pool_id The pool ID.
    /// @return True if suspended, false otherwise.
    fn is_pool_suspended(self: @TContractState, pool_id: u256) -> bool;

    /// @notice Returns all suspended pools.
    /// @return Array of PoolDetails.
    fn get_suspended_pools(self: @TContractState) -> Array<PoolDetails>;

    /// @notice Validates an outcome for a pool.
    /// @param pool_id The pool ID.
    /// @param outcome The outcome to validate.
    fn validate_outcome(ref self: TContractState, pool_id: u256, outcome: bool);

    /// @notice Claims reward for a pool.
    /// @param pool_id The pool ID.
    /// @return The claimed reward amount.
    fn claim_reward(ref self: TContractState, pool_id: u256) -> u256;
}

#[starknet::interface]
pub trait IPredifiValidator<TContractState> {
    // Pool Validation functionality

    /// @notice Validates the result of a pool.
    /// @dev Only callable by validators. Emits ValidatorResultSubmitted and may settle the pool.
    /// @param pool_id The pool ID.
    /// @param selected_option The selected option (true = option2, false = option1).
    fn validate_pool_result(ref self: TContractState, pool_id: u256, selected_option: bool);

    /// @notice Gets pool validation status.
    /// @param pool_id The ID of the pool to check.
    /// @return (validation count, is settled, final outcome).
    fn get_pool_validation_status(
        self: @TContractState, pool_id: u256,
    ) -> (u256, bool, bool); // (validation_count, is_settled, final_outcome)

    /// @notice Gets validator confirmation status.
    /// @param pool_id The ID of the pool to check.
    /// @param validator The address of the validator to check.
    /// @return (has confirmed, selected option).
    fn get_validator_confirmation(
        self: @TContractState, pool_id: u256, validator: ContractAddress,
    ) -> (bool, bool); // (has_validated, selected_option)

    /// @notice Sets the required number of validator confirmations for a pool.
    /// @dev Only callable by admin.
    /// @param count The number of confirmations required.
    fn set_required_validator_confirmations(ref self: TContractState, count: u256);

    /// @notice Gets the validators assigned to a pool.
    /// @param pool_id The pool ID.
    /// @return (validator1, validator2).
    fn get_pool_validators(
        self: @TContractState, pool_id: u256,
    ) -> (ContractAddress, ContractAddress);

    /// @notice Assigns random validators to a pool.
    /// @dev Internal function.
    /// @param pool_id The pool ID.
    fn assign_random_validators(ref self: TContractState, pool_id: u256);

    /// @notice Assigns specific validators to a pool.
    /// @dev Internal function.
    /// @param pool_id The pool ID.
    /// @param validator1 The first validator.
    /// @param validator2 The second validator.
    fn assign_validators(
        ref self: TContractState,
        pool_id: u256,
        validator1: ContractAddress,
        validator2: ContractAddress,
    );

    /// @notice Adds a validator.
    /// @dev Only callable by admin.
    /// @param address The validator's address.
    fn add_validator(ref self: TContractState, address: ContractAddress);

    /// @notice Removes a validator.
    /// @dev Only callable by admin.
    /// @param address The validator's address.
    fn remove_validator(ref self: TContractState, address: ContractAddress);

    /// @notice Checks if an address is a validator.
    /// @param address The address to check.
    /// @return True if validator, false otherwise.
    fn is_validator(self: @TContractState, address: ContractAddress) -> bool;

    /// @notice Returns all validators.
    /// @return Array of validator addresses.
    fn get_all_validators(self: @TContractState) -> Array<ContractAddress>;

    /// @notice Calculates the validator fee for a pool.
    /// @param pool_id The pool ID.
    /// @param total_amount The total amount to calculate fee from.
    /// @return The validator fee.
    fn calculate_validator_fee(ref self: TContractState, pool_id: u256, total_amount: u256) -> u256;

    /// @notice Distributes validator fees for a pool.
    /// @param pool_id The pool ID.
    fn distribute_validator_fees(ref self: TContractState, pool_id: u256);

    /// @notice Retrieves the validator fee for a pool.
    /// @param pool_id The pool ID.
    /// @return The validator fee.
    fn retrieve_validator_fee(self: @TContractState, pool_id: u256) -> u256;

    /// @notice Gets the validator fee percentage for a pool.
    /// @param pool_id The pool ID.
    /// @return The validator fee percentage.
    fn get_validator_fee_percentage(self: @TContractState, pool_id: u256) -> u8;

    // Upgradeability

    /// @notice Upgrades the contract implementation.
    /// @param new_class_hash The class hash of the new implementation.
    /// @dev Can only be called by admin when contract is not paused.
    fn upgrade(ref self: TContractState, new_class_hash: ClassHash);

    // Pausable functionality

    /// @notice Pauses all state-changing operations in the contract.
    /// @dev Can only be called by admin. Emits Paused event on success.
    fn pause(ref self: TContractState);

    /// @notice Unpauses the contract and resumes normal operations.
    /// @dev Can only be called by admin. Emits Unpaused event on success.
    fn unpause(ref self: TContractState);
}
