use crate::base::types::{PoolDetails, Pool, Category, ValidateOptions, PoolOdds};
use starknet::ContractAddress;
use crate::base::types::{UserStake};

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
    ) -> bool;
    // fn validate_pool(ref self: TContractState, pool_id: u32, option: ValidateOptions) -> bool;
    // fn upgrade(ref self: TContractState, new_class_hash: starknet::class_hash::ClassHash);

    // Pool Queries
    fn get_all_pools(self: @TContractState) -> Array<PoolDetails>;
    // fn get_active_pools(self: @TContractState) -> Array<PoolDetails>;
    // fn get_locked_pools(self: @TContractState) -> Array<PoolDetails>;
    // fn get_closed_pools(self: @TContractState) -> Array<PoolDetails>;
    // fn get_pool_by_id(self: @TContractState, pool_id: u32) -> PoolDetails;
    // fn get_pools_by_contract_address(
    //     self: @TContractState, contract_address: ContractAddress,
    // ) -> Array<PoolDetails>;
    // fn get_pools_by_category(self: @TContractState, category: Category) -> Array<PoolDetails>;

    // // User Actions and Queries
    fn vote_in_pool(
        ref self: TContractState,
        pool_id: u256,
        amount: u256,
        option: u8,
        recepient: ContractAddress,
        token: ContractAddress,
    ) -> bool;
    // Function to update user stake
    fn update_user_stake(
        ref self: TContractState, user: ContractAddress, amount: u256, shares: u256, option: u8,
    );
    // Function to get user stake details
    fn get_user_stake(ref self: TContractState, user: ContractAddress) -> UserStake;
    fn calculate_user_shares(
        ref self: TContractState,
        user_stake: u256,
        total_stake: u256,
        total_shares: u256,
        odds: u16,
    ) -> u256;
    // fn claim(ref self: TContractState, pool_id: u32) -> bool;
// fn get_user_wins(self: @TContractState, user: ContractAddress) -> u32;
// fn get_user_losses(self: @TContractState, user: ContractAddress) -> u32;
// fn get_user_total_bets(self: @TContractState, user: ContractAddress) -> u32;
// fn get_all_pools_user_voted(self: @TContractState) -> Array<PoolDetails>;

    // // Pool Economics and Calculations
// fn get_pool_odds(self: @TContractState, pool_id: u32) -> PoolOdds;
// fn calculate_potential_payout(
//     self: @TContractState, pool_id: u32, stake_amount: u256, option: felt252,
// ) -> u256;
// fn get_share_price(self: @TContractState, pool_id: u32, option: felt252) -> u256;
// fn get_liquidity_depth(self: @TContractState, pool_id: u32, price_point: u256) -> (u256,
// u256);

    // fn get_strk_usd_price(self: @TContractState) -> (u128, u32);
}
// fn update_pool_result(ref self: TContractState, pool_id: u32, winning_option: felt252);
// many other get functions, get wins, get losses get total bet, more storage like that, a
// struct that has all info about the user, current pools hes active on, and many other things
// like that @martinvibes issue for you


