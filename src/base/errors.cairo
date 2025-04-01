pub mod Errors {
    pub const REQUIRED_PAYMENT: u128 = 1000;
    pub const INVALID_POOL_OPTION: felt252 = 'Invalid Pool Option';
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
}
