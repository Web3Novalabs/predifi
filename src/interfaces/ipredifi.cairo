use starknet::ContractAddress;
use crate::base::types::{Category, Pool, PoolDetails, PoolOdds, Status, UserStake};
#[starknet::interface]
pub trait IPredifi<TContractState> {
    // Pool Creation and Management
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
    fn cancel_pool(ref self: TContractState, pool_id: u256);
    fn pool_count(self: @TContractState) -> u256;
    fn pool_odds(self: @TContractState, pool_id: u256) -> PoolOdds;
    fn get_pool(self: @TContractState, pool_id: u256) -> PoolDetails;
    fn vote(ref self: TContractState, pool_id: u256, option: felt252, amount: u256);
    fn stake(ref self: TContractState, pool_id: u256, amount: u256);
    fn refund_stake(ref self: TContractState, pool_id: u256);
    fn get_user_stake(self: @TContractState, pool_id: u256, address: ContractAddress) -> UserStake;
    fn get_pool_stakes(self: @TContractState, pool_id: u256) -> UserStake;
    fn get_pool_vote(self: @TContractState, pool_id: u256) -> bool;
    fn get_pool_count(self: @TContractState) -> u256;
    fn retrieve_pool(self: @TContractState, pool_id: u256) -> bool;
    fn get_pool_creator(self: @TContractState, pool_id: u256) -> ContractAddress;
    fn get_creator_fee_percentage(self: @TContractState, pool_id: u256) -> u8;
    fn get_validator_fee_percentage(self: @TContractState, pool_id: u256) -> u8;
    fn collect_pool_creation_fee(ref self: TContractState, creator: ContractAddress);
    fn calculate_validator_fee(ref self: TContractState, pool_id: u256, total_amount: u256) -> u256;
    fn distribute_validator_fees(ref self: TContractState, pool_id: u256);
    fn retrieve_validator_fee(self: @TContractState, pool_id: u256) -> u256;
    fn update_pool_state(ref self: TContractState, pool_id: u256) -> Status;
    fn manually_update_pool_state(
        ref self: TContractState, pool_id: u256, new_status: Status,
    ) -> Status;

    fn get_user_pool_count(self: @TContractState, user: ContractAddress) -> u256;
    fn check_user_participated(self: @TContractState, user: ContractAddress, pool_id: u256) -> bool;
    fn get_user_pools(
        self: @TContractState, user: ContractAddress, status_filter: Option<Status>,
    ) -> Array<u256>;
    fn has_user_participated_in_pool(
        self: @TContractState, user: ContractAddress, pool_id: u256,
    ) -> bool;

    fn get_user_active_pools(self: @TContractState, user: ContractAddress) -> Array<u256>;

    fn get_user_locked_pools(self: @TContractState, user: ContractAddress) -> Array<u256>;

    fn get_user_settled_pools(self: @TContractState, user: ContractAddress) -> Array<u256>;

    // Functions for filtering pools by status
    fn get_active_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_locked_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_settled_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_closed_pools(self: @TContractState) -> Array<PoolDetails>;
}

#[starknet::interface]
pub trait IPredifiDispute<TContractState> {
    //dispute functionality
    fn raise_dispute(ref self: TContractState, pool_id: u256);
    fn resolve_dispute(ref self: TContractState, pool_id: u256, winning_option: bool);
    fn get_dispute_count(self: @TContractState, pool_id: u256) -> u256;
    fn get_dispute_threshold(self: @TContractState) -> u256;
    fn has_user_disputed(self: @TContractState, pool_id: u256, user: ContractAddress) -> bool;
    fn is_pool_suspended(self: @TContractState, pool_id: u256) -> bool;
    fn get_suspended_pools(self: @TContractState) -> Array<PoolDetails>;
    fn validate_outcome(ref self: TContractState, pool_id: u256, outcome: bool);
    fn claim_reward(ref self: TContractState, pool_id: u256) -> u256;
}

#[starknet::interface]
pub trait IPredifiValidator<TContractState> {
    // Pool Validation functionality
    fn validate_pool_result(ref self: TContractState, pool_id: u256, selected_option: bool);
    fn get_pool_validation_status(
        self: @TContractState, pool_id: u256,
    ) -> (u256, bool, bool); // (validation_count, is_settled, final_outcome)
    fn get_validator_confirmation(
        self: @TContractState, pool_id: u256, validator: ContractAddress,
    ) -> (bool, bool); // (has_validated, selected_option)
    fn set_required_validator_confirmations(ref self: TContractState, count: u256);
    fn get_pool_validators(
        self: @TContractState, pool_id: u256,
    ) -> (ContractAddress, ContractAddress);

    fn assign_random_validators(ref self: TContractState, pool_id: u256);
    fn assign_validators(
        ref self: TContractState,
        pool_id: u256,
        validator1: ContractAddress,
        validator2: ContractAddress,
    );
    fn add_validator(ref self: TContractState, address: ContractAddress);
    fn remove_validator(ref self: TContractState, address: ContractAddress);
    fn is_validator(self: @TContractState, address: ContractAddress) -> bool;
    fn get_all_validators(self: @TContractState) -> Array<ContractAddress>;
}
