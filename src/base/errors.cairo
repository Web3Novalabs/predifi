pub mod Errors {
    pub const REQUIRED_PAYMENT: u128 = 1000;
    pub const INVALID_POOL_OPTION: felt252 = 'Invalid Pool Option';
    pub const INVALID_POOL_TYPE: felt252 = 'Invalid Pool Type';
    pub const INACTIVE_POOL: felt252 = 'Pool is inactive';
    pub const AMOUNT_BELOW_MINIMUM: felt252 = 'Amount is below minimum';
    pub const AMOUNT_ABOVE_MAXIMUM: felt252 = 'Amount is above maximum';
    pub const INVALID_POOL_DETAILS: felt252 = 'Invalid Pool Details';
    pub const INVALID_VOTE_DETAILS: felt252 = 'Invalid Vote Details';
    pub const LOCKED_PREDICTION_POOL: felt252 = 'PREDICTION POOL HAS BEEN LOCKED';
    pub const PAYMENT_FAILED: felt252 = 'TRANSFER FAILED';
    pub const TOTAL_STAKE_MUST_BE_ONE_STRK: felt252 = 'Total stake should be 1 STRK';
    pub const TOTAL_SHARE_MUST_BE_ONE_STRK: felt252 = 'Total shares should be 1 STRK';
    pub const USER_SHARE_MUST_BE_ONE_STRK: felt252 = 'User shares should be 1 STRK';
    pub const POOL_SUSPENDED: felt252 = 'Pool is suspended';
    pub const DISPUTE_ALREADY_RAISED: felt252 = 'User already raised dispute';
    pub const POOL_NOT_SUSPENDED: felt252 = 'Pool is not suspended';
    pub const POOL_NOT_LOCKED: felt252 = 'Pool is not locked';
    pub const POOL_NOT_CLOSED: felt252 = 'Pool is not closed';
    pub const POOL_NOT_SETTLED: felt252 = 'Pool is not settled';
    pub const POOL_NOT_RESOLVED: felt252 = 'Pool is not resolved';

    // Validation Errors
    pub const VALIDATOR_NOT_AUTHORIZED: felt252 = 'Validator not authorized';
    pub const VALIDATOR_ALREADY_VALIDATED: felt252 = 'Validator already validated';
    pub const POOL_NOT_READY_FOR_VALIDATION: felt252 = 'Pool not ready for validation';
}
