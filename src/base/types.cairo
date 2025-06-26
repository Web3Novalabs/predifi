// Import necessary types
use starknet::ContractAddress;

#[derive(Copy, Drop, Serde, PartialEq, starknet::Store, Debug)]
pub enum Pool {
    #[default]
    WinBet,
    VoteBet,
    OverUnderBet,
    ParlayPool,
}

#[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
pub enum Status {
    #[default]
    Active,
    Locked,
    Settled,
    Closed,
    Suspended,
}


#[derive(Copy, Drop, Serde, PartialEq, starknet::Store, Clone)]
pub struct UserStake {
    pub amount: u256,
    pub shares: u256,
    pub option: bool,
    pub timestamp: u64,
}

#[derive(Drop, Serde, PartialEq, starknet::Store, Clone)]
pub struct PoolOdds {
    pub option1_odds: u256, // Stored in basis points (10000 = 1.0)
    pub option2_odds: u256,
    pub option1_probability: u256, // Stored in basis points (10000 = 100%)
    pub option2_probability: u256,
    pub implied_probability1: u256,
    pub implied_probability2: u256,
}


fn StatusType(status: Status) -> felt252 {
    match status {
        Status::Active => 'active',
        Status::Locked => 'locked',
        Status::Settled => 'settled',
        Status::Closed => 'closed',
        Status::Suspended => 'suspended',
    }
}

fn PoolType(PoolType: Pool) -> felt252 {
    match PoolType {
        Pool::WinBet => 'win bet',
        Pool::VoteBet => 'vote bet',
        Pool::OverUnderBet => 'over under bet',
        Pool::ParlayPool => 'parlay pool',
    }
}


#[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
pub enum Category {
    #[default]
    Sports,
    Politics,
    Entertainment,
    Crypto,
    Other,
}

#[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
pub enum ValidateOptions {
    #[default]
    Win,
    Loss,
    Void,
}

pub fn ValidateOptionsType(validate_option: ValidateOptions) -> felt252 {
    match validate_option {
        ValidateOptions::Win => 'win',
        ValidateOptions::Loss => 'loss',
        ValidateOptions::Void => 'void',
    }
}


pub fn CategoryType(category: Category) -> felt252 {
    match category {
        Category::Sports => 'sports',
        Category::Politics => 'politics',
        Category::Entertainment => 'entertainment',
        Category::Crypto => 'crypto',
        Category::Other => 'other',
    }
}

#[derive(Drop, Serde, PartialEq, Debug, starknet::Store, Clone)]
pub struct ValidatorData {
    pub status: bool,
    pub preodifiTokenAmount: u256,
}


#[derive(Drop, Serde, PartialEq, Debug, starknet::Store, Clone)]
pub struct WinaAndLoss {
    pub win: u32,
    pub loss: u32,
    pub null: u32,
}


#[derive(Drop, Serde, PartialEq, Debug, starknet::Store, Clone)]
pub struct PoolDetails {
    // basic pool details
    pub pool_id: u256,
    pub address: starknet::ContractAddress,
    pub poolName: felt252,
    pub poolType: Pool,
    pub poolDescription: ByteArray,
    pub poolImage: ByteArray,
    // event url where users can see more event details and verify event
    pub poolEventSourceUrl: ByteArray,
    pub createdTimeStamp: u64,
    // pool timings: start time, lock time, end time
    pub poolStartTime: u64,
    pub poolLockTime: u64,
    pub poolEndTime: u64,
    // pool options, the options that users can bet on
    // pub option1: felt252,
    // pub option2: felt252,
    pub option1: felt252,
    pub option2: felt252,
    // betamounts in strk
    pub minBetAmount: u256,
    pub maxBetAmount: u256,
    // the fee that the creator gets
    pub creatorFee: u8,
    pub status: Status,
    pub isPrivate: bool,
    pub category: Category,
    pub totalBetAmountStrk: u256,
    pub totalBetCount: u8,
    pub totalStakeOption1: u256,
    pub totalStakeOption2: u256,
    pub totalSharesOption1: u256,
    pub totalSharesOption2: u256,
    pub initial_share_price: u16,
    pub exists: bool,
}

// Events module
pub mod Events {
    use super::{Status, ContractAddress};

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