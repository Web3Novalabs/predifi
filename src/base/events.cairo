// Import necessary types
use starknet::{ContractAddress, ClassHash};
use crate::base::types::Status;

// Events module
pub mod Events {
    use super::{ContractAddress, ClassHash, Status};

    /// @notice Emitted when a bet is placed on a pool.
    /// @param pool_id Pool where bet was placed
    /// @param address Address of the user placing the bet
    /// @param option The outcome option that was bet on
    /// @param amount Amount of tokens bet
    /// @param shares Number of shares received for the bet
    #[derive(Drop, starknet::Event)]
    pub struct BetPlaced {
        /// @notice The pool ID.
        #[key]
        pub pool_id: u256,
        /// @notice The address of the user who placed the bet.
        #[key]
        pub address: ContractAddress,
        /// @notice The option selected by the user.
        pub option: felt252,
        /// @notice The amount bet by the user.
        pub amount: u256,
        /// @notice The number of shares received for the bet.
        pub shares: u256,
    }

    /// @notice Emitted when a user stakes tokens to become a validator.
    /// @param pool_id Pool for which the user is staking
    /// @param address Address of the user staking tokens
    /// @param amount Amount of tokens staked
    #[derive(Drop, starknet::Event)]
    pub struct UserStaked {
        #[key]
        pub pool_id: u256,
        #[key]
        pub address: ContractAddress,
        pub amount: u256,
    }

    /// @notice Emitted when a user's stake is refunded.
    /// @param pool_id Pool from which stake is refunded
    /// @param address Address of the user receiving the refund
    /// @param amount Amount of tokens refunded
    #[derive(Drop, starknet::Event)]
    pub struct StakeRefunded {
        pub pool_id: u256,
        pub address: ContractAddress,
        pub amount: u256,
    }

    /// @notice Emitted when fees are collected.
    /// @param fee_type Type of fee collected (creator_fee, protocol_fee)
    /// @param pool_id Pool from which fees were collected
    /// @param recipient Address receiving the collected fees
    /// @param amount Amount of fees collected
    #[derive(Drop, starknet::Event)]
    pub struct FeesCollected {
        pub fee_type: felt252,
        pub pool_id: u256,
        pub recipient: ContractAddress,
        pub amount: u256,
    }

    /// @notice Emitted when a pool changes state.\
    /// @param pool_id Unique identifier of the pool
    /// @param previous_status The previous status of the pool
    /// @param new_status The new status of the pool
    /// @param timestamp Time of the state transition
    #[derive(Drop, starknet::Event)]
    pub struct PoolStateTransition {
        #[key]
        pub pool_id: u256,
        pub previous_status: Status,
        pub new_status: Status,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is resolved.
    /// @param pool_id Unique identifier of the created pool
    /// @param creator Address of the pool creator
    /// @param category Pool category (Sports, Finance, etc.)
    /// @param end_time Timestamp when betting closes
    /// @param min_bet_amount Minimum bet requirement
    #[derive(Drop, starknet::Event)]
    pub struct PoolResolved {
        #[key]
        pub pool_id: u256,
        pub winning_option: bool,
        pub total_payout: u256,
    }

    /// @notice Emitted when fees are withdrawn.
    /// @param fee_type Type of fee being withdrawn (creator_fee, protocol_fee)
    /// @param recipient Address receiving the withdrawn fees
    /// @param amount Amount of fees withdrawn
    #[derive(Drop, starknet::Event)]
    pub struct FeeWithdrawn {
        pub fee_type: felt252,
        pub recipient: ContractAddress,
        pub amount: u256,
    }

    /// @notice Emitted when validators are assigned to a pool.
    /// @param pool_id Unique identifier of the pool
    /// @param validator1 Address of the first assigned validator
    /// @param validator2 Address of the second assigned validator
    /// @param timestamp Time of assignment
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorsAssigned {
        pub pool_id: u256,
        pub validator1: ContractAddress,
        pub validator2: ContractAddress,
        pub timestamp: u64,
    }

    /// @notice Emitted when a validator is added.
    /// @param account Address of the new validator
    /// @param caller Address of the admin who performed the addition
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorAdded {
        #[key]
        pub account: ContractAddress,
        #[key]
        pub caller: ContractAddress,
    }

    /// @notice Emitted when a validator is removed.
    /// @param account Address of the removed validator
    /// @param caller Address of the admin who performed the removal
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorRemoved {
        #[key]
        pub account: ContractAddress,
        #[key]
        pub caller: ContractAddress,
    }

    /// @notice Emitted when a dispute is raised for a pool.
    /// @param pool_id Unique identifier of the disputed pool
    /// @param user Address of the user raising the dispute
    /// @param timestamp Time of dispute initiation
    #[derive(Drop, starknet::Event)]
    pub struct DisputeRaised {
        #[key]
        pub pool_id: u256,
        #[key]
        pub user: ContractAddress,
        pub timestamp: u64,
    }

    /// @notice Emitted when a dispute is resolved.
    /// @param pool_id Unique identifier of the disputed pool
    /// @param winning_option The option that won the dispute (true = option2, false = option1)
    /// @param timestamp Time of dispute resolution
    #[derive(Drop, starknet::Event)]
    pub struct DisputeResolved {
        pub pool_id: u256,
        pub winning_option: bool,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is suspended.
    /// @param pool_id Unique identifier of the suspended pool
    /// @param timestamp Time of suspension
    #[derive(Drop, starknet::Event)]
    pub struct PoolSuspended {
        pub pool_id: u256,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is cancelled.
    /// @param pool_id Unique identifier of the cancelled pool
    /// @param timestamp Time of cancellation
    #[derive(Drop, starknet::Event)]
    pub struct PoolCancelled {
        pub pool_id: u256,
        pub timestamp: u64,
    }

    /// @notice Emitted when a validator submits a result for a pool.
    /// @param pool_id Pool being validated
    /// @param validator Address of the validator
    /// @param selected_option Option selected by the validator
    /// @param timestamp Time of submission
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorResultSubmitted {
        pub pool_id: u256,
        pub validator: ContractAddress,
        pub selected_option: bool,
        pub timestamp: u64,
    }

    /// @notice Emitted when a pool is automatically settled.
    /// @param pool_id Pool that was settled
    /// @param final_outcome The final outcome determined
    /// @param total_validations Number of validations received
    /// @param timestamp Time of settlement
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

    // Configuration Events

    /// @notice Emitted when the required validator confirmations count is updated.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorConfirmationsUpdated {
        pub previous_count: u256,
        pub new_count: u256,
        #[key]
        pub admin: ContractAddress,
        pub timestamp: u64,
    }

    /// @notice Emitted when the dispute threshold is updated.
    #[derive(Drop, starknet::Event)]
    pub struct DisputeThresholdUpdated {
        pub previous_threshold: u256,
        pub new_threshold: u256,
        pub admin: ContractAddress,
        pub timestamp: u64,
    }

    // Pool Lifecycle Events

    /// @notice Emitted when a new pool is created.
    #[derive(Drop, starknet::Event)]
    pub struct PoolCreated {
        #[key]
        pub pool_id: u256,
        #[key]
        pub creator: ContractAddress,
        pub pool_name: felt252,
        #[key]
        pub category: felt252,
        pub end_time: u64,
        pub min_bet_amount: u256,
        pub max_bet_amount: u256,
        pub creator_fee: u8,
        pub timestamp: u64,
    }

    // Contract Management Events

    /// @notice Emitted when the contract is paused.
    #[derive(Drop, starknet::Event)]
    pub struct ContractPaused {
        #[key]
        pub admin: ContractAddress,
        pub timestamp: u64,
    }

    /// @notice Emitted when the contract is unpaused.
    #[derive(Drop, starknet::Event)]
    pub struct ContractUnpaused {
        #[key]
        pub admin: ContractAddress,
        pub timestamp: u64,
    }


    // Fee Collection Events

    /// @notice Emitted when protocol fees are collected.
    #[derive(Drop, starknet::Event)]
    pub struct ProtocolFeesCollected {
        pub pool_id: u256,
        pub amount: u256,
        pub recipient: ContractAddress,
        pub timestamp: u64,
    }

    /// @notice Emitted when creator fees are collected.
    #[derive(Drop, starknet::Event)]
    pub struct CreatorFeesCollected {
        pub pool_id: u256,
        pub creator: ContractAddress,
        pub amount: u256,
        pub timestamp: u64,
    }

    /// @notice Emitted when validator fees are distributed.
    #[derive(Drop, starknet::Event)]
    pub struct ValidatorFeesDistributed {
        pub pool_id: u256,
        pub total_amount: u256,
        pub validator_count: u256,
        pub timestamp: u64,
    }

    /// @notice Emitted when pool creation fee is collected.
    #[derive(Drop, starknet::Event)]
    pub struct PoolCreationFeeCollected {
        pub pool_id: u256,
        pub creator: ContractAddress,
        pub amount: u256,
        pub timestamp: u64,
    }
}
