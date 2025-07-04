/// @notice Enum representing the types of pools available.
#[derive(Copy, Drop, Serde, PartialEq, starknet::Store, Debug)]
pub enum Pool {
    #[default]
    /// @notice A standard win bet pool.
    WinBet,
    /// @notice A pool where users vote on an outcome.
    VoteBet,
    /// @notice An over/under bet pool.
    OverUnderBet,
    /// @notice A parlay pool (multiple bets combined).
    ParlayPool,
}

/// @notice Enum representing the status of a pool.
#[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
pub enum Status {
    #[default]
    /// @notice The pool is active and open for participation.
    Active,
    /// @notice The pool is locked and no longer accepts new bets.
    Locked,
    /// @notice The pool has been settled.
    Settled,
    /// @notice The pool is closed.
    Closed,
    /// @notice The pool is suspended due to a dispute or issue.
    Suspended,
}

/// @notice Struct representing a user's stake in a pool.
#[derive(Copy, Drop, Serde, PartialEq, starknet::Store, Clone)]
pub struct UserStake {
    /// @notice The amount staked by the user.
    pub amount: u256,
    /// @notice The number of shares received for the stake.
    pub shares: u256,
    /// @notice The option the user selected (true = option2, false = option1).
    pub option: bool,
    /// @notice The timestamp of the stake.
    pub timestamp: u64,
}

/// @notice Struct representing the odds for a pool.
#[derive(Drop, Serde, PartialEq, starknet::Store, Clone)]
pub struct PoolOdds {
    /// @notice Odds for option 1 (in basis points, 10000 = 1.0).
    pub option1_odds: u256,
    /// @notice Odds for option 2 (in basis points, 10000 = 1.0).
    pub option2_odds: u256,
    /// @notice Probability for option 1 (in basis points, 10000 = 100%).
    pub option1_probability: u256,
    /// @notice Probability for option 2 (in basis points, 10000 = 100%).
    pub option2_probability: u256,
    /// @notice Implied probability for option 1.
    pub implied_probability1: u256,
    /// @notice Implied probability for option 2.
    pub implied_probability2: u256,
}

/// @notice Converts a Status enum to its string representation.
/// @param status The status to convert.
/// @return The string representation as felt252.
fn StatusType(status: Status) -> felt252 {
    match status {
        Status::Active => 'active',
        Status::Locked => 'locked',
        Status::Settled => 'settled',
        Status::Closed => 'closed',
        Status::Suspended => 'suspended',
    }
}

/// @notice Converts a Pool enum to its string representation.
/// @param PoolType The pool type to convert.
/// @return The string representation as felt252.
fn PoolType(PoolType: Pool) -> felt252 {
    match PoolType {
        Pool::WinBet => 'win bet',
        Pool::VoteBet => 'vote bet',
        Pool::OverUnderBet => 'over under bet',
        Pool::ParlayPool => 'parlay pool',
    }
}

/// @notice Converts a u8 to a Pool enum.
/// @param pool_type The pool type as u8.
/// @return The Pool enum value.
pub fn u8_to_pool(pool_type: u8) -> Pool {
    match pool_type {
        0 => Pool::WinBet,
        1 => Pool::VoteBet,
        2 => Pool::OverUnderBet,
        3 => Pool::ParlayPool,
        _ => panic!("Invalid pool type: must be 0-3"),
    }
}

/// @notice Enum representing the category of a pool.
#[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
pub enum Category {
    #[default]
    /// @notice Sports category.
    Sports,
    /// @notice Politics category.
    Politics,
    /// @notice Entertainment category.
    Entertainment,
    /// @notice Crypto category.
    Crypto,
    /// @notice Other category.
    Other,
}

/// @notice Enum representing validation options for a pool.
#[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
pub enum ValidateOptions {
    #[default]
    /// @notice Win option.
    Win,
    /// @notice Loss option.
    Loss,
    /// @notice Void option.
    Void,
}

/// @notice Converts a ValidateOptions enum to its string representation.
/// @param validate_option The validation option.
/// @return The string representation as felt252.
pub fn ValidateOptionsType(validate_option: ValidateOptions) -> felt252 {
    match validate_option {
        ValidateOptions::Win => 'win',
        ValidateOptions::Loss => 'loss',
        ValidateOptions::Void => 'void',
    }
}

/// @notice Converts a Category enum to its string representation.
/// @param category The category to convert.
/// @return The string representation as felt252.
pub fn CategoryType(category: Category) -> felt252 {
    match category {
        Category::Sports => 'sports',
        Category::Politics => 'politics',
        Category::Entertainment => 'entertainment',
        Category::Crypto => 'crypto',
        Category::Other => 'other',
    }
}

/// @notice Struct representing validator data.
#[derive(Drop, Serde, PartialEq, Debug, starknet::Store, Clone)]
pub struct ValidatorData {
    /// @notice Validator status (active/inactive).
    pub status: bool,
    /// @notice Amount of Predifi tokens staked by the validator.
    pub preodifiTokenAmount: u256,
}

/// @notice Struct representing win/loss/null statistics.
#[derive(Drop, Serde, PartialEq, Debug, starknet::Store, Clone)]
pub struct WinaAndLoss {
    /// @notice Number of wins.
    pub win: u32,
    /// @notice Number of losses.
    pub loss: u32,
    /// @notice Number of null results.
    pub null: u32,
}

/// @notice Struct representing all details of a pool.
#[derive(Drop, Serde, PartialEq, Debug, starknet::Store, Clone)]
pub struct PoolDetails {
    /// @notice The unique pool ID.
    pub pool_id: u256,
    /// @notice The contract address of the pool.
    pub address: starknet::ContractAddress,
    /// @notice The name of the pool.
    pub poolName: felt252,
    /// @notice The type of the pool.
    pub poolType: Pool,
    /// @notice The description of the pool.
    pub poolDescription: ByteArray,
    /// @notice The image URL for the pool.
    pub poolImage: ByteArray,
    /// @notice The event source URL for the pool.
    pub poolEventSourceUrl: ByteArray,
    /// @notice The timestamp when the pool was created.
    pub createdTimeStamp: u64,
    /// @notice The start time of the pool.
    pub poolStartTime: u64,
    /// @notice The lock time of the pool.
    pub poolLockTime: u64,
    /// @notice The end time of the pool.
    pub poolEndTime: u64,
    /// @notice The first option for the pool.
    pub option1: felt252,
    /// @notice The second option for the pool.
    pub option2: felt252,
    /// @notice The minimum bet amount.
    pub minBetAmount: u256,
    /// @notice The maximum bet amount.
    pub maxBetAmount: u256,
    /// @notice The fee percentage for the pool creator.
    pub creatorFee: u8,
    /// @notice The current status of the pool.
    pub status: Status,
    /// @notice Whether the pool is private.
    pub isPrivate: bool,
    /// @notice The category of the pool.
    pub category: Category,
    /// @notice The total bet amount in STRK.
    pub totalBetAmountStrk: u256,
    /// @notice The total number of bets placed.
    pub totalBetCount: u8,
    /// @notice The total stake for option 1.
    pub totalStakeOption1: u256,
    /// @notice The total stake for option 2.
    pub totalStakeOption2: u256,
    /// @notice The total shares for option 1.
    pub totalSharesOption1: u256,
    /// @notice The total shares for option 2.
    pub totalSharesOption2: u256,
    /// @notice The initial share price.
    pub initial_share_price: u16,
    /// @notice Whether the pool exists.
    pub exists: bool,
}
