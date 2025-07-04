pub mod Errors {
    /// @notice The minimum required payment for certain operations.
    pub const REQUIRED_PAYMENT: u128 = 1000;

    /// @notice Error: The selected pool option is invalid.
    pub const INVALID_POOL_OPTION: felt252 = 'Invalid Pool Option';

    /// @notice Error: The pool type is invalid.
    pub const INVALID_POOL_TYPE: felt252 = 'Invalid Pool Type';

    /// @notice Error: The pool is inactive.
    pub const INACTIVE_POOL: felt252 = 'Pool is inactive';

    /// @notice Error: The amount is below the minimum allowed.
    pub const AMOUNT_BELOW_MINIMUM: felt252 = 'Amount is below minimum';

    /// @notice Error: The amount is above the maximum allowed.
    pub const AMOUNT_ABOVE_MAXIMUM: felt252 = 'Amount is above maximum';

    /// @notice Error: The pool details provided are invalid.
    pub const INVALID_POOL_DETAILS: felt252 = 'Invalid Pool Details';

    /// @notice Error: The vote details provided are invalid.
    pub const INVALID_VOTE_DETAILS: felt252 = 'Invalid Vote Details';

    /// @notice Error: The prediction pool has been locked.
    pub const LOCKED_PREDICTION_POOL: felt252 = 'PREDICTION POOL HAS BEEN LOCKED';

    /// @notice Error: Token transfer failed.
    pub const PAYMENT_FAILED: felt252 = 'TRANSFER FAILED';

    /// @notice Error: The total stake must be exactly 1 STRK.
    pub const TOTAL_STAKE_MUST_BE_ONE_STRK: felt252 = 'Total stake should be 1 STRK';

    /// @notice Error: The total shares must be exactly 1 STRK.
    pub const TOTAL_SHARE_MUST_BE_ONE_STRK: felt252 = 'Total shares should be 1 STRK';

    /// @notice Error: The user shares must be exactly 1 STRK.
    pub const USER_SHARE_MUST_BE_ONE_STRK: felt252 = 'User shares should be 1 STRK';

    /// @notice Error: The pool is suspended.
    pub const POOL_SUSPENDED: felt252 = 'Pool is suspended';

    /// @notice Error: The user has already raised a dispute.
    pub const DISPUTE_ALREADY_RAISED: felt252 = 'User already raised dispute';

    /// @notice Error: The pool is not suspended.
    pub const POOL_NOT_SUSPENDED: felt252 = 'Pool is not suspended';

    /// @notice Error: The pool is not locked.
    pub const POOL_NOT_LOCKED: felt252 = 'Pool is not locked';

    /// @notice Error: The pool is not closed.
    pub const POOL_NOT_CLOSED: felt252 = 'Pool is not closed';

    /// @notice Error: The pool is not settled.
    pub const POOL_NOT_SETTLED: felt252 = 'Pool is not settled';

    /// @notice Error: The pool is not resolved.
    pub const POOL_NOT_RESOLVED: felt252 = 'Pool is not resolved';

    /// @notice Error: The pool does not exist.
    pub const POOL_DOES_NOT_EXIST: felt252 = 'Pool does not exist';

    /// @notice Error: The requested state transition is invalid.
    pub const INVALID_STATE_TRANSITION: felt252 = 'Invalid state transition';

    /// @notice Error: The account has insufficient balance.
    pub const INSUFFICIENT_BALANCE: felt252 = 'Insufficient balance';

    /// @notice Error: The account has insufficient allowance.
    pub const INSUFFICIENT_ALLOWANCE: felt252 = 'Insufficient allowance';

    /// @notice Error: The stake amount is too low.
    pub const STAKE_AMOUNT_TOO_LOW: felt252 = 'stake amount too low';

    /// @notice Error: The user has zero stake.
    pub const ZERO_USER_STAKE: felt252 = 'Zero user stake';

    /// @notice Error: The account has insufficient STRK balance.
    pub const INSUFFICIENT_STRK_BALANCE: felt252 = 'Insufficient STRK balance';

    /// @notice Error: The count must be greater than zero.
    pub const COUNT_MUST_BE_GREATER_THAN_ZERO: felt252 = 'Count must be greater than 0';

    // Validation Errors

    /// @notice Error: The validator is not authorized.
    pub const VALIDATOR_NOT_AUTHORIZED: felt252 = 'Validator not authorized';

    /// @notice Error: The validator has already validated.
    pub const VALIDATOR_ALREADY_VALIDATED: felt252 = 'Validator already validated';

    /// @notice Error: The pool is not ready for validation.
    pub const POOL_NOT_READY_FOR_VALIDATION: felt252 = 'Pool not ready for validation';

    /// @notice Error: The lock time is invalid.
    pub const INVALID_LOCK_TIME: felt252 = 'Invalid lock time';

    /// @notice Error: The lock time is greater than the end time.
    pub const INVALID_LOCK_TIME_TO_END_TIME: felt252 = 'lock time greater than end time';

    /// @notice Error: The minimum bet cannot be zero.
    pub const ZERO_MINIMUM_BET: felt252 = 'Minimum bet cannot be zero';

    /// @notice Error: The maximum bet is invalid.
    pub const INVALID_MAXIMUM_BET: felt252 = 'Invalid Maximum Bet';

    /// @notice Error: The start time is invalid.
    pub const INVALID_START_TIME: felt252 = 'Invalid Start Time';

    /// @notice Error: The creator fee cannot exceed 5%.
    pub const CREATOR_FEE_TOO_HIGH: felt252 = 'Creator fee cannot exceed 5%';

    /// @notice Error: The caller is not authorized to perform this action.
    pub const UNAUTHORIZED_CALLER: felt252 = 'Unauthorized Caller';
}
