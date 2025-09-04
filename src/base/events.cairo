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

    // Emergency Events

    /// @notice Emitted when an emergency withdrawal is made from a pool.
    #[derive(Drop, starknet::Event)]
    pub struct EmergencyWithdrawal {
        pub pool_id: u256,
        pub user: ContractAddress,
        pub amount: u256,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is frozen due to emergency.
    #[derive(Drop, starknet::Event)]
    pub struct PoolEmergencyFrozen {
        pub pool_id: u256,
        pub admin: ContractAddress,
        pub reason: felt252,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is unfrozen from emergency state.
    #[derive(Drop, starknet::Event)]
    pub struct PoolEmergencyUnfrozen {
        pub pool_id: u256,
        pub admin: ContractAddress,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is resolved through emergency resolution.
    #[derive(Drop, starknet::Event)]
    pub struct PoolEmergencyResolved {
        pub pool_id: u256,
        pub admin: ContractAddress,
        pub winning_option: bool,
        pub timestamp: u64,
    }

    /// @notice Emitted when an emergency action is scheduled with timelock.
    #[derive(Drop, starknet::Event)]
    pub struct EmergencyActionScheduled {
        pub action_id: u256,
        pub action_type: u8,
        pub pool_id: u256,
        pub admin: ContractAddress,
        pub execution_time: u64,
        pub timestamp: u64,
    }

    /// @notice Emitted when a scheduled emergency action is executed.
    #[derive(Drop, starknet::Event)]
    pub struct EmergencyActionExecuted {
        pub action_id: u256,
        pub admin: ContractAddress,
        pub timestamp: u64,
    }

    /// @notice Emitted when a scheduled emergency action is cancelled.
    #[derive(Drop, starknet::Event)]
    pub struct EmergencyActionCancelled {
        pub action_id: u256,
        pub admin: ContractAddress,
        pub timestamp: u64,
    }
    #[derive(Drop,starknet::Event)]
    pub struct ValidatorSlashed{
        #[key]
        pub validator:ContractAddress,
        pub amount:u256,
        pub reputation_after:u256,
    }
    #[derive(Drop,starknet::Event)]
    pub struct ValidatorPerformanceUpdated{
        #[key]
        pub validator:ContractAddress,
        pub success:bool,
        pub reputation_after:u256,
    }
    #[derive(Drop,starknet::Event)]
    pub struct FeesDistributed{
        pub pool_id:u256,
        pub total_distributed: u256,
    }
}
