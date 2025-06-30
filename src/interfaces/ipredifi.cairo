use starknet::ContractAddress;
use crate::base::types::{Category, Pool, PoolDetails, PoolOdds, Status, UserStake};
#[starknet::interface]
pub trait IPredifi<TContractState> {
    // Pool Creation and Management

    /// @param poolName The name of the pool.
    /// @param poolType The type of the pool.
    /// @param poolDescription The description of the pool.
    /// @param poolImage The image of the pool.
    /// @param poolEventSourceUrl The event source URL.
    /// @param poolStartTime The start time of the pool.
    /// @param poolLockTime The lock time of the pool.
    /// @param poolEndTime The end time of the pool.
    /// @param option1 The first option for the pool.
    /// @param option2 The second option for the pool.
    /// @param minBetAmount The minimum bet amount.
    /// @param maxBetAmount The maximum bet amount.
    /// @param creatorFee The creator fee percentage.
    /// @param isPrivate Whether the pool is private.
    /// @param category The category of the pool.
    /// @return pool_id The ID of the created pool.
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
        category: Category,
    ) -> u256;

    /// @notice Cancels an existing pool.
    /// @param pool_id The ID of the pool to cancel.
    fn cancel_pool(ref self: TContractState, pool_id: u256);
    
    /// @notice Returns the total number of pools.
    /// @return count The number of pools.
    fn pool_count(self: @TContractState) -> u256;

    /// @notice Returns the odds for a given pool.
    /// @param pool_id The pool ID.
    /// @return odds The odds for the pool.
    fn pool_odds(self: @TContractState, pool_id: u256) -> PoolOdds;

    /// @notice Returns the details of a pool.
    /// @param pool_id The pool ID.
    /// @return details The pool details.
    fn get_pool(self: @TContractState, pool_id: u256) -> PoolDetails;

    /// @notice Vote on a pool option.
    /// @param pool_id The pool ID.
    /// @param option The option to vote for.
    /// @param amount The amount to vote with.
    fn vote(ref self: TContractState, pool_id: u256, option: felt252, amount: u256);

    /// @notice Stake tokens in a pool.
    /// @param pool_id The pool ID.
    /// @param amount The amount to stake.
    fn stake(ref self: TContractState, pool_id: u256, amount: u256);

    /// @notice Refunds a user's stake from a pool.
    /// @param pool_id The pool ID.
    fn refund_stake(ref self: TContractState, pool_id: u256);

    /// @notice Returns a user's stake in a pool.
    /// @param pool_id The pool ID.
    /// @param address The user's address.
    /// @return stake The user's stake.
    fn get_user_stake(self: @TContractState, pool_id: u256, address: ContractAddress) -> UserStake;

    /// @notice Returns the total stake in a pool.
    /// @param pool_id The pool ID.
    /// @return stake The total stake.
    fn get_pool_stakes(self: @TContractState, pool_id: u256) -> UserStake;

    /// @notice Returns whether a pool has been voted on.
    /// @param pool_id The pool ID.
    /// @return voted True if voted.
    fn get_pool_vote(self: @TContractState, pool_id: u256) -> bool;

    /// @notice Returns the total pool count.
    /// @return count The number of pools.
    fn get_pool_count(self: @TContractState) -> u256;

    /// @notice Retrieves a pool.
    /// @param pool_id The pool ID.
    /// @return exists True if the pool exists.
    fn retrieve_pool(self: @TContractState, pool_id: u256) -> bool;

    /// @notice Returns the creator of a pool.
    /// @param pool_id The pool ID.
    /// @return creator The creator's address.
    fn get_pool_creator(self: @TContractState, pool_id: u256) -> ContractAddress;

    /// @notice Returns the creator fee percentage for a pool.
    /// @param pool_id The pool ID.
    /// @return fee The fee percentage.
    fn get_creator_fee_percentage(self: @TContractState, pool_id: u256) -> u8;

    /// @notice Returns the validator fee percentage for a pool.
    /// @param pool_id The pool ID.
    /// @return fee The validator fee percentage.
    fn get_validator_fee_percentage(self: @TContractState, pool_id: u256) -> u8;

    /// @notice Collects the pool creation fee from the creator.
    /// @param creator The creator's address.
    fn collect_pool_creation_fee(ref self: TContractState, creator: ContractAddress);

    /// @notice Calculates the validator fee for a pool.
    /// @param pool_id The pool ID.
    /// @param total_amount The total amount in the pool.
    /// @return fee The calculated validator fee.
    fn calculate_validator_fee(ref self: TContractState, pool_id: u256, total_amount: u256) -> u256;

    /// @notice Distributes validator fees for a pool.
    /// @param pool_id The pool ID.
    fn distribute_validator_fees(ref self: TContractState, pool_id: u256);

    /// @notice Retrieves the validator fee for a pool.
    /// @param pool_id The pool ID.
    /// @return fee The validator fee.
    fn retrieve_validator_fee(self: @TContractState, pool_id: u256) -> u256;

    /// @notice Updates the state of a pool.
    /// @param pool_id The pool ID.
    /// @return status The updated status.
    fn update_pool_state(ref self: TContractState, pool_id: u256) -> Status;

    /// @notice Manually updates the state of a pool.
    /// @param pool_id The pool ID.
    /// @param new_status The new status to set.
    /// @return status The updated status.
    fn manually_update_pool_state(
        ref self: TContractState, pool_id: u256, new_status: Status,
    ) -> Status;

    /// @notice Returns the number of pools a user has participated in.
    /// @param user The user's address.
    /// @return count The number of pools.
    fn get_user_pool_count(self: @TContractState, user: ContractAddress) -> u256;

    /// @notice Checks if a user has participated in a pool.
    /// @param user The user's address.
    /// @param pool_id The pool ID.
    /// @return participated True if the user participated.
    fn check_user_participated(self: @TContractState, user: ContractAddress, pool_id: u256) -> bool;

    /// @notice Returns the pools a user has participated in, optionally filtered by status.
    /// @param user The user's address.
    /// @param status_filter Optional status filter.
    /// @return pools The list of pool IDs.
    fn get_user_pools(
        self: @TContractState, user: ContractAddress, status_filter: Option<Status>,
    ) -> Array<u256>;

    /// @notice Checks if a user has participated in a specific pool.
    /// @param user The user's address.
    /// @param pool_id The pool ID.
    /// @return participated True if the user participated.
    fn has_user_participated_in_pool(
        self: @TContractState, user: ContractAddress, pool_id: u256,
    ) -> bool;

    /// @notice Returns the active pools for a user.
    /// @param user The user's address.
    /// @return pools The list of active pool IDs.
    fn get_user_active_pools(self: @TContractState, user: ContractAddress) -> Array<u256>;

    /// @notice Returns the locked pools for a user.
    /// @param user The user's address.
    /// @return pools The list of locked pool IDs.
    fn get_user_locked_pools(self: @TContractState, user: ContractAddress) -> Array<u256>;

    /// @notice Returns the settled pools for a user.
    /// @param user The user's address.
    /// @return pools The list of settled pool IDs.
    fn get_user_settled_pools(self: @TContractState, user: ContractAddress) -> Array<u256>;

    /// @notice Returns the validators for a pool.
    /// @param pool_id The pool ID.
    /// @return validator1 The first validator address.
    /// @return validator2 The second validator address.
    fn get_pool_validators(
        self: @TContractState, pool_id: u256,
    ) -> (ContractAddress, ContractAddress);

    /// @notice Assigns random validators to a pool.
    /// @param pool_id The pool ID.
    fn assign_random_validators(ref self: TContractState, pool_id: u256);

    /// @notice Assigns specific validators to a pool.
    /// @param pool_id The pool ID.
    /// @param validator1 The first validator address.
    /// @param validator2 The second validator address.
    fn assign_validators(
        ref self: TContractState,
        pool_id: u256,
        validator1: ContractAddress,
        validator2: ContractAddress,
    );

    /// @notice Adds a validator.
    /// @param address The validator's address.
    fn add_validator(ref self: TContractState, address: ContractAddress);

    /// @notice Removes a validator.
    /// @param address The validator's address.
    fn remove_validator(ref self: TContractState, address: ContractAddress);

    /// @notice Checks if an address is a validator.
    /// @param address The address to check.
    /// @return is_validator True if the address is a validator.
    fn is_validator(self: @TContractState, address: ContractAddress) -> bool;

    /// @notice Returns all validators.
    /// @return validators The list of validator addresses.
    fn get_all_validators(self: @TContractState) -> Array<ContractAddress>;

    // Functions for filtering pools by status

    /// @notice Returns all active pools.
    /// @return pools The list of active pool details.
    fn get_active_pools(self: @TContractState) -> Array<PoolDetails>;

    /// @notice Returns all locked pools.
    /// @return pools The list of locked pool details.
    fn get_locked_pools(self: @TContractState) -> Array<PoolDetails>;

    /// @notice Returns all settled pools.
    /// @return pools The list of settled pool details.
    fn get_settled_pools(self: @TContractState) -> Array<PoolDetails>;

    /// @notice Returns all closed pools.
    /// @return pools The list of closed pool details.
    fn get_closed_pools(self: @TContractState) -> Array<PoolDetails>;

    //dispute functionality
    
    /// @notice Raises a dispute for a pool.
    /// @param pool_id The pool ID.
    fn raise_dispute(ref self: TContractState, pool_id: u256);

    /// @notice Resolves a dispute for a pool.
    /// @param pool_id The pool ID.
    /// @param winning_option The winning option.
    fn resolve_dispute(ref self: TContractState, pool_id: u256, winning_option: bool);

    /// @notice Returns the dispute count for a pool.
    /// @param pool_id The pool ID.
    /// @return count The dispute count.
    fn get_dispute_count(self: @TContractState, pool_id: u256) -> u256;

    /// @notice Returns the dispute threshold.
    /// @return threshold The dispute threshold.
    fn get_dispute_threshold(self: @TContractState) -> u256;

    /// @notice Checks if a user has disputed a pool.
    /// @param pool_id The pool ID.
    /// @param user The user's address.
    /// @return has_disputed True if the user has disputed.
    fn has_user_disputed(self: @TContractState, pool_id: u256, user: ContractAddress) -> bool;

    /// @notice Checks if a pool is suspended.
    /// @param pool_id The pool ID.
    /// @return suspended True if the pool is suspended.
    fn is_pool_suspended(self: @TContractState, pool_id: u256) -> bool;

    /// @notice Returns all suspended pools.
    /// @return pools The list of suspended pool details.
    fn get_suspended_pools(self: @TContractState) -> Array<PoolDetails>;

    /// @notice Validates the outcome of a pool.
    /// @param pool_id The pool ID.
    /// @param outcome The outcome to validate.
    fn validate_outcome(ref self: TContractState, pool_id: u256, outcome: bool);

    /// @notice Claims reward for a pool.
    /// @param pool_id The pool ID.
    /// @return reward The claimed reward amount.
    fn claim_reward(ref self: TContractState, pool_id: u256) -> u256;

    // Pool Validation functionality

    /// @notice Validates the result of a pool.
    /// @param pool_id The pool ID.
    /// @param selected_option The selected option.
    fn validate_pool_result(ref self: TContractState, pool_id: u256, selected_option: bool);

    /// @notice Returns the validation status of a pool.
    /// @param pool_id The pool ID.
    /// @return validation_count The number of validations.
    /// @return is_settled True if the pool is settled.
    /// @return final_outcome The final outcome.
    fn get_pool_validation_status(
        self: @TContractState, pool_id: u256,
    ) -> (u256, bool, bool);

    /// @notice Returns the validator's confirmation for a pool.
    /// @param pool_id The pool ID.
    /// @param validator The validator's address.
    /// @return has_validated True if the validator has validated.
    /// @return selected_option The selected option.
    fn get_validator_confirmation(
        self: @TContractState, pool_id: u256, validator: ContractAddress,
    ) -> (bool, bool);

    /// @notice Sets the required number of validator confirmations.
    /// @param count The required confirmation count.
    fn set_required_validator_confirmations(ref self: TContractState, count: u256);
}
