/// @notice Represents the type of prediction pool.
#[derive(Copy, Drop, Serde, PartialEq, starknet::Store, Debug)]
pub enum Pool {
    #[default]
    WinBet,         /// @notice Standard win bet pool.
    VoteBet,        /// @notice Pool where users vote on outcomes.
    OverUnderBet,   /// @notice Over/Under style prediction pool.
    ParlayPool,     /// @notice Parlay (multi-event) pool.
}

/// @notice Status of a prediction pool.
#[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
pub enum Status {
    #[default]
    Active,     /// @notice Pool is active and open for participation.
    Locked,     /// @notice Pool is locked and no longer accepts new bets.
    Settled,    /// @notice Pool has been settled and payouts distributed.
    Closed,     /// @notice Pool is closed.
    Suspended,  /// @notice Pool is suspended due to dispute or admin action.
}

/// @notice Represents a user's stake in a pool.
#[derive(Copy, Drop, Serde, PartialEq, starknet::Store, Clone)]
pub struct UserStake {
    pub amount: u256,      /// @notice Amount staked by the user.
    pub shares: u256,      /// @notice Shares received for the stake.
    pub option: bool,      /// @notice Option chosen by the user.
    pub timestamp: u64,    /// @notice Timestamp of the stake.
}

/// @notice Odds and probabilities for pool options.
#[derive(Drop, Serde, PartialEq, starknet::Store, Clone)]
pub struct PoolOdds {
    pub option1_odds: u256,           /// @notice Odds for option 1 (basis points).
    pub option2_odds: u256,           /// @notice Odds for option 2 (basis points).
    pub option1_probability: u256,    /// @notice Probability for option 1 (basis points).
    pub option2_probability: u256,    /// @notice Probability for option 2 (basis points).
    pub implied_probability1: u256,   /// @notice Implied probability for option 1.
    pub implied_probability2: u256,   /// @notice Implied probability for option 2.
}

/// @notice Converts a Status enum to its string representation.
/// @param status The status to convert.
/// @return The string representation of the status.
fn StatusType(status: Status) -> felt252 { ... }

/// @notice Converts a Pool enum to its string representation.
/// @param PoolType The pool type to convert.
/// @return The string representation of the pool type.
fn PoolType(PoolType: Pool) -> felt252 { ... }

/// @notice Converts a u8 to a Pool enum.
/// @param pool_type The pool type as u8.
/// @return The corresponding Pool enum variant.
pub fn u8_to_pool(pool_type: u8) -> Pool { ... }

/// @notice Category of the prediction pool.
#[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
pub enum Category {
    #[default]
    Sports,         /// @notice Sports-related pool.
    Politics,       /// @notice Politics-related pool.
    Entertainment,  /// @notice Entertainment-related pool.
    Crypto,         /// @notice Crypto-related pool.
    Other,          /// @notice Other category.
}

/// @notice Validation options for pool outcomes.
#[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
pub enum ValidateOptions {
    #[default]
    Win,    /// @notice Win outcome.
    Loss,   /// @notice Loss outcome.
    Void,   /// @notice Void outcome.
}

/// @notice Converts a ValidateOptions enum to its string representation.
/// @param validate_option The validation option.
/// @return The string representation.
pub fn ValidateOptionsType(validate_option: ValidateOptions) -> felt252 { ... }

/// @notice Converts a Category enum to its string representation.
/// @param category The category.
/// @return The string representation.
pub fn CategoryType(category: Category) -> felt252 { ... }

/// @notice Data for a validator in the protocol.
#[derive(Drop, Serde, PartialEq, Debug, starknet::Store, Clone)]
pub struct ValidatorData {
    pub status: bool,                 /// @notice Validator's status (active/inactive).
    pub preodifiTokenAmount: u256,    /// @notice Amount of tokens staked by the validator.
}

/// @notice Tracks win/loss/null statistics.
#[derive(Drop, Serde, PartialEq, Debug, starknet::Store, Clone)]
pub struct WinaAndLoss {
    pub win: u32,     /// @notice Number of wins.
    pub loss: u32,    /// @notice Number of losses.
    pub null: u32,    /// @notice Number of null results.
}

/// @notice Detailed information about a prediction pool.
#[derive(Drop, Serde, PartialEq, Debug, starknet::Store, Clone)]
pub struct PoolDetails {
    pub pool_id: u256,                    /// @notice Unique pool identifier.
    pub address: starknet::ContractAddress, /// @notice Pool contract address.
    pub poolName: felt252,                 /// @notice Name of the pool.
    pub poolType: Pool,                    /// @notice Type of the pool.
    pub poolDescription: ByteArray,        /// @notice Description of the pool.
    pub poolImage: ByteArray,              /// @notice Image associated with the pool.
    pub poolEventSourceUrl: ByteArray,     /// @notice Event source URL for verification.
    pub createdTimeStamp: u64,             /// @notice Pool creation timestamp.
    pub poolStartTime: u64,                /// @notice Pool start time.
    pub poolLockTime: u64,                 /// @notice Pool lock time.
    pub poolEndTime: u64,                  /// @notice Pool end time.
    pub option1: felt252,                  /// @notice First option for the pool.
    pub option2: felt252,                  /// @notice Second option for the pool.
    pub minBetAmount: u256,                /// @notice Minimum bet amount.
    pub maxBetAmount: u256,                /// @notice Maximum bet amount.
    pub creatorFee: u8,                    /// @notice Fee percentage for the creator.
    pub status: Status,                    /// @notice Current status of the pool.
    pub isPrivate: bool,                   /// @notice Whether the pool is private.
    pub category: Category,                /// @notice Pool category.
    pub totalBetAmountStrk: u256,          /// @notice Total bet amount in STRK.
    pub totalBetCount: u8,                 /// @notice Total number of bets.
    pub totalStakeOption1: u256,           /// @notice Total stake for option 1.
    pub totalStakeOption2: u256,           /// @notice Total stake for option 2.
    pub totalSharesOption1: u256,          /// @notice Total shares for option 1.
    pub totalSharesOption2: u256,          /// @notice Total shares for option 2.
    pub initial_share_price: u16,          /// @notice Initial share price.
    pub exists: bool,                      /// @notice Whether the pool exists.
}