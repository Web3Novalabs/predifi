// Import necessary types
use starknet::ContractAddress;
use crate::base::types::Status;

// Events module
pub mod Events {
    use super::{ContractAddress, Status};

    /// @notice Emitted when a bet is placed on a pool.
    #[derive(Drop, starknet::Event)]
    pub struct BetPlaced {
        /// @notice The pool ID.
        pub pool_id: u256,
        /// @notice The address of the user who placed the bet.
        pub address: ContractAddress,
        /// @notice The option selected by the user.
        pub option: felt252,
        /// @notice The amount bet by the user.
        pub amount: u256,
        /// @notice The number of shares received for the bet.
        pub shares: u256,
    }

    /// @notice Emitted when a user stakes tokens to become a validator.
    #[derive(Drop, starknet::Event)]
    pub struct UserStaked {
        pub pool_id: u256,
        pub address: ContractAddress,
        pub amount: u256,
    }

    /// @notice Emitted when a user's stake is refunded.
    #[derive(Drop, starknet::Event)]
    pub struct StakeRefunded {
        pub pool_id: u256,
        pub address: ContractAddress,
        pub amount: u256,
    }

    /// @notice Emitted when fees are collected.
    #[derive(Drop, starknet::Event)]
    pub struct FeesCollected {
        pub fee_type: felt252,
        pub pool_id: u256,
        pub recipient: ContractAddress,
        pub amount: u256,
    }

    /// @notice Emitted when a pool changes state.
    #[derive(Drop, starknet::Event)]
    pub struct PoolStateTransition {
        pub pool_id: u256,
        pub previous_status: Status,
        pub new_status: Status,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is resolved.
    #[derive(Drop, starknet::Event)]
    pub struct PoolResolved {
        pub pool_id: u256,
        pub winning_option: bool,
        pub total_payout: u256,
    }

    /// @notice Emitted when fees are withdrawn.
    #[derive(Drop, starknet::Event)]
    pub struct FeeWithdrawn {
        pub fee_type: felt252,
        pub recipient: ContractAddress,
        pub amount: u256,
    }

    /// @notice Emitted when validators are assigned to a pool.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorsAssigned {
        pub pool_id: u256,
        pub validator1: ContractAddress,
        pub validator2: ContractAddress,
        pub timestamp: u64,
    }

    /// @notice Emitted when a validator is added.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorAdded {
        pub account: ContractAddress,
        pub caller: ContractAddress,
    }

    /// @notice Emitted when a validator is removed.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorRemoved {
        pub account: ContractAddress,
        pub caller: ContractAddress,
    }

    /// @notice Emitted when a dispute is raised for a pool.
    #[derive(Drop, starknet::Event)]
    pub struct DisputeRaised {
        pub pool_id: u256,
        pub user: ContractAddress,
        pub timestamp: u64,
    }

    /// @notice Emitted when a dispute is resolved.
    #[derive(Drop, starknet::Event)]
    pub struct DisputeResolved {
        pub pool_id: u256,
        pub winning_option: bool,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is suspended.
    #[derive(Drop, starknet::Event)]
    pub struct PoolSuspended {
        pub pool_id: u256,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is cancelled.
    #[derive(Drop, starknet::Event)]
    pub struct PoolCancelled {
        pub pool_id: u256,
        pub timestamp: u64,
    }

    /// @notice Emitted when a validator submits a result for a pool.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorResultSubmitted {
        pub pool_id: u256,
        pub validator: ContractAddress,
        pub selected_option: bool,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is automatically settled.
    #[derive(Drop, starknet::Event)]
    pub struct PoolAutomaticallySettled {
        pub pool_id: u256,
        pub final_outcome: bool,
        pub total_validations: u256,
        pub timestamp: u64,
    }
}
