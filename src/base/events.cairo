// Import necessary types
use starknet::ContractAddress;
use crate::base::types::Status;

// Events module
pub mod Events {
    use super::{ContractAddress, Status};

    #[derive(Drop, starknet::Event)]
    pub struct BetPlaced {
        pub pool_id: u256,
        pub address: ContractAddress,
        pub option: felt252,
        pub amount: u256,
        pub shares: u256,
    }
    #[derive(Drop, starknet::Event)]
    pub struct UserStaked {
        pub pool_id: u256,
        pub address: ContractAddress,
        pub amount: u256,
    }
    #[derive(Drop, starknet::Event)]
    pub struct StakeRefunded {
        pub pool_id: u256,
        pub address: ContractAddress,
        pub amount: u256,
    }
    #[derive(Drop, starknet::Event)]
    pub struct FeesCollected {
        pub fee_type: felt252,
        pub pool_id: u256,
        pub recipient: ContractAddress,
        pub amount: u256,
    }

    #[derive(Drop, starknet::Event)]
    pub struct PoolStateTransition {
        pub pool_id: u256,
        pub previous_status: Status,
        pub new_status: Status,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct PoolResolved {
        pub pool_id: u256,
        pub winning_option: bool,
        pub total_payout: u256,
    }

    #[derive(Drop, starknet::Event)]
    pub struct FeeWithdrawn {
        pub fee_type: felt252,
        pub recipient: ContractAddress,
        pub amount: u256,
    }

    #[derive(Drop, starknet::Event)]
    pub struct ValidatorsAssigned {
        pub pool_id: u256,
        pub validator1: ContractAddress,
        pub validator2: ContractAddress,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct ValidatorAdded {
        pub account: ContractAddress,
        pub caller: ContractAddress,
    }

    #[derive(Drop, starknet::Event)]
    pub struct ValidatorRemoved {
        pub account: ContractAddress,
        pub caller: ContractAddress,
    }

    #[derive(Drop, starknet::Event)]
    pub struct DisputeRaised {
        pub pool_id: u256,
        pub user: ContractAddress,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct DisputeResolved {
        pub pool_id: u256,
        pub winning_option: bool,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct PoolSuspended {
        pub pool_id: u256,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct PoolCancelled {
        pub pool_id: u256,
        pub timestamp: u64,
    }

    // Validator event structs
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorResultSubmitted {
        pub pool_id: u256,
        pub validator: ContractAddress,
        pub selected_option: bool,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct PoolAutomaticallySettled {
        pub pool_id: u256,
        pub final_outcome: bool,
        pub total_validations: u256,
        pub timestamp: u64,
    }
}
