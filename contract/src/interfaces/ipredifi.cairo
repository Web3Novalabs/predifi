use starknet::ContractAddress;
use crate::base::types::{Category, Pool, PoolDetails, PoolOdds, Status, UserStake};
#[starknet::interface]
pub trait IPredifi<TContractState> {
    // Pool Creation and Management
    fn create_pool(
        ref self: TContractState,
        poolName: felt252,
        poolType: Pool,
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
    fn pool_count(self: @TContractState) -> u256;
    fn pool_odds(self: @TContractState, pool_id: u256) -> PoolOdds;
    fn get_pool(self: @TContractState, pool_id: u256) -> PoolDetails;
    fn vote(ref self: TContractState, pool_id: u256, option: felt252, amount: u256);
    fn stake(ref self: TContractState, pool_id: u256, amount: u256);
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
    fn add_validators(
        ref self: TContractState,
        validator1: ContractAddress,
        validator2: ContractAddress,
        validator3: ContractAddress,
        validator4: ContractAddress,
    ) -> Array<ContractAddress>;
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

    fn get_pool_validators(
        self: @TContractState, pool_id: u256,
    ) -> (ContractAddress, ContractAddress, ContractAddress);

    fn assign_random_validators(ref self: TContractState, pool_id: u256);
    fn assign_validators(
        ref self: TContractState,
        pool_id: u256,
        validator1: ContractAddress,
        validator2: ContractAddress,
    );
    // Functions for filtering pools by status
    fn get_active_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_locked_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_settled_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_closed_pools(self: @TContractState) -> Array<PoolDetails>;
}
