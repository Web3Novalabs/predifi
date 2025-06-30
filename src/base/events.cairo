/// @notice Event definitions for the PrediFi protocol.
/// @dev These events are emitted by the protocol contracts to signal important state changes and actions.

use starknet::ContractAddress;
use crate::base::types::Status;

pub mod Events {
    use super::{ContractAddress, Status};

    /// @notice Emitted when a user places a bet in a pool.
    #[derive(Drop, starknet::Event)]
    pub struct BetPlaced {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub address: ContractAddress,       /// @notice The address of the user placing the bet.
        pub option: felt252,                /// @notice The option selected by the user.
        pub amount: u256,                   /// @notice The amount bet.
        pub shares: u256,                   /// @notice The number of shares received.
    }

    /// @notice Emitted when a user stakes tokens in a pool.
    #[derive(Drop, starknet::Event)]
    pub struct UserStaked {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub address: ContractAddress,       /// @notice The address of the user staking.
        pub amount: u256,                   /// @notice The amount staked.
    }

    /// @notice Emitted when a user's stake is refunded.
    #[derive(Drop, starknet::Event)]
    pub struct StakeRefunded {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub address: ContractAddress,       /// @notice The address of the user refunded.
        pub amount: u256,                   /// @notice The amount refunded.
    }

    /// @notice Emitted when fees are collected for a pool.
    #[derive(Drop, starknet::Event)]
    pub struct FeesCollected {
        pub fee_type: felt252,              /// @notice The type of fee collected.
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub recipient: ContractAddress,     /// @notice The recipient of the fee.
        pub amount: u256,                   /// @notice The amount collected.
    }

    /// @notice Emitted when a pool changes state (e.g., active to locked).
    #[derive(Drop, starknet::Event)]
    pub struct PoolStateTransition {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub previous_status: Status,        /// @notice The previous status of the pool.
        pub new_status: Status,             /// @notice The new status of the pool.
        pub timestamp: u64,                 /// @notice The timestamp of the transition.
    }

    /// @notice Emitted when a pool is resolved and payouts are distributed.
    #[derive(Drop, starknet::Event)]
    pub struct PoolResolved {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub winning_option: bool,           /// @notice The winning option.
        pub total_payout: u256,             /// @notice The total payout distributed.
    }

    /// @notice Emitted when fees are withdrawn from the protocol.
    #[derive(Drop, starknet::Event)]
    pub struct FeeWithdrawn {
        pub fee_type: felt252,              /// @notice The type of fee withdrawn.
        pub recipient: ContractAddress,     /// @notice The recipient of the withdrawn fee.
        pub amount: u256,                   /// @notice The amount withdrawn.
    }

    /// @notice Emitted when validators are assigned to a pool.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorsAssigned {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub validator1: ContractAddress,    /// @notice The address of the first validator.
        pub validator2: ContractAddress,    /// @notice The address of the second validator.
        pub timestamp: u64,                 /// @notice The timestamp of assignment.
    }

    /// @notice Emitted when a validator is added to the protocol.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorAdded {
        pub account: ContractAddress,       /// @notice The address of the validator added.
        pub caller: ContractAddress,        /// @notice The address of the caller who added the validator.
    }

    /// @notice Emitted when a validator is removed from the protocol.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorRemoved {
        pub account: ContractAddress,       /// @notice The address of the validator removed.
        pub caller: ContractAddress,        /// @notice The address of the caller who removed the validator.
    }

    /// @notice Emitted when a user raises a dispute for a pool.
    #[derive(Drop, starknet::Event)]
    pub struct DisputeRaised {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub user: ContractAddress,          /// @notice The address of the user raising the dispute.
        pub timestamp: u64,                 /// @notice The timestamp of the dispute.
    }

    /// @notice Emitted when a dispute is resolved for a pool.
    #[derive(Drop, starknet::Event)]
    pub struct DisputeResolved {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub winning_option: bool,           /// @notice The outcome of the dispute.
        pub timestamp: u64,                 /// @notice The timestamp of the resolution.
    }

    /// @notice Emitted when a pool is suspended.
    #[derive(Drop, starknet::Event)]
    pub struct PoolSuspended {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub timestamp: u64,                 /// @notice The timestamp of suspension.
    }

    /// @notice Emitted when a pool is cancelled.
    #[derive(Drop, starknet::Event)]
    pub struct PoolCancelled {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub timestamp: u64,                 /// @notice The timestamp of cancellation.
    }

    /// @notice Emitted when a validator submits a result for a pool.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorResultSubmitted {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub validator: ContractAddress,     /// @notice The address of the validator.
        pub selected_option: bool,          /// @notice The option selected by the validator.
        pub timestamp: u64,                 /// @notice The timestamp of submission.
    }

    /// @notice Emitted when a pool is automatically settled by the protocol.
    #[derive(Drop, starknet::Event)]
    pub struct PoolAutomaticallySettled {
        pub pool_id: u256,                  /// @notice The ID of the pool.
        pub final_outcome: bool,            /// @notice The final outcome of the pool.
        pub total_validations: u256,        /// @notice The total number of validations.
        pub timestamp: u64,                 /// @notice The timestamp of settlement.
    }
}
