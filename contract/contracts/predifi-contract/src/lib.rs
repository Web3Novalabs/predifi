#![no_std]
use access_control::Role;
use predifi_errors::PrediFiError;
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, token, Address, Env};

const RESOLUTION_WINDOW: u64 = 7 * 24 * 60 * 60; // 7 days in seconds

// Event structs for contract events
#[contractevent]
pub struct SetFeeBpsEvent {
    pub new_fee_bps: u32,
}

#[contractevent]
pub struct SetTreasuryEvent {
    pub new_treasury: Address,
}

#[contractevent]
pub struct FeeCollectedEvent {
    pub pool_id: u64,
    pub fee: i128,
}

#[contractevent]
pub struct FeeDistributedEvent {
    pub pool_id: u64,
    pub fee: i128,
}
#[contractevent]
pub struct PoolCreatedEvent {
    pub pool_id: u64,
    pub creator: Address,
    pub end_time: u64,
}

#[contractevent]
pub struct PredictionPlacedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
    pub outcome: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct Pool {
    pub creator: Address, // Added for auth/tracking
    pub end_time: u64,
    pub resolved: bool,
    pub outcome: u32,
    pub token: Address,
    pub total_stake: i128,
    pub min_stake: i128,
    pub category: soroban_sdk::Symbol,
    pub options_count: u32, // To validate outcomes later
}

#[contracttype]
#[derive(Clone)]
pub struct UserPredictionDetail {
    pub pool_id: u64,
    pub amount: i128,
    pub user_outcome: u32,
    pub pool_end_time: u64,
    pub pool_resolved: bool,
    pub pool_outcome: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Pool(u64),
    Prediction(Address, u64), // User, PoolId
    PoolIdCounter,
    HasClaimed(Address, u64), // User, PoolId
    OutcomeStake(u64, u32),   // PoolId, Outcome -> Total stake for this outcome
    UserPredictionCount(Address),
    UserPredictionIndex(Address, u32), // User, Index -> PoolId
    FeeBps,                            // Fee in basis points (1/100 of a percent)
    Treasury,                          // Protocol treasury address
    CollectedFees(u64),                // PoolId -> Collected fee amount
    AccessControlAddress,              // Access control contract address
}

#[contracttype]
#[derive(Clone)]
pub struct Prediction {
    pub amount: i128,
    pub outcome: u32,
}

#[contract]
pub struct PredifiContract;

#[contractimpl]
impl PredifiContract {
    /// Get the access control contract address
    ///
    /// # Returns
    /// The address of the access control contract
    ///
    /// # Errors
    /// * `NotInitialized` - If access control contract is not set
    fn get_access_control_address(env: &Env) -> Result<Address, PrediFiError> {
        env.storage()
            .instance()
            .get(&DataKey::AccessControlAddress)
            .ok_or(PrediFiError::NotInitialized)
    }

    /// Get the access control client
    ///
    /// # Returns
    /// The AccessControlClient instance
    fn get_access_control_client(env: &Env) -> access_control::AccessControlClient<'_> {
        let access_control_addr = Self::get_access_control_address(env).unwrap();
        access_control::AccessControlClient::new(env, &access_control_addr)
    }

    /// Set the access control contract address (only callable once)
    ///
    /// # Arguments
    /// * `access_control_address` - The address of the access control contract
    ///
    /// # Errors
    /// * `AlreadyInitialized` - If access control contract is already set
    pub fn set_access_control(
        env: Env,
        access_control_address: Address,
    ) -> Result<(), PrediFiError> {
        if env.storage().instance().has(&DataKey::AccessControlAddress) {
            return Err(PrediFiError::AlreadyInitialized);
        }
        env.storage()
            .instance()
            .set(&DataKey::AccessControlAddress, &access_control_address);
        Ok(())
    }

    /// Initialize the contract.
    ///
    /// Sets up the initial pool ID counter, fee basis points, and treasury address.
    ///
    /// # Arguments
    /// * `treasury` - Address to receive protocol fees
    /// * `fee_bps` - Fee in basis points (e.g., 100 = 1%)
    pub fn init(env: Env, treasury: Address, fee_bps: u32) {
        // Only set if not already initialized
        if !env.storage().instance().has(&DataKey::PoolIdCounter) {
            env.storage().instance().set(&DataKey::PoolIdCounter, &0u64);
        }
        if !env.storage().instance().has(&DataKey::FeeBps) {
            env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
        }
        if !env.storage().instance().has(&DataKey::Treasury) {
            env.storage().instance().set(&DataKey::Treasury, &treasury);
        }
    }

    /// Set the protocol fee in basis points.
    ///
    /// # Arguments
    /// * `caller` - The address calling the function
    /// * `fee_bps` - Fee in basis points (e.g., 100 = 1%)
    ///
    /// # Errors
    /// * `InsufficientPermissions` - If caller doesn't have Admin role
    pub fn set_fee_bps(env: Env, caller: Address, fee_bps: u32) -> Result<(), PrediFiError> {
        // Check if caller has Admin role
        let access_control_client = Self::get_access_control_client(&env);
        if !access_control_client.has_role(&caller, &Role::Admin) {
            return Err(PrediFiError::InsufficientPermissions);
        }

        env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
        SetFeeBpsEvent {
            new_fee_bps: fee_bps,
        }
        .publish(&env);
        Ok(())
    }

    /// Get the current protocol fee in basis points.
    ///
    /// # Returns
    /// The fee in basis points, or 0 if not set
    pub fn get_fee_bps(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0)
    }

    /// Set the treasury address.
    ///
    /// # Arguments
    /// * `caller` - The address calling the function
    /// * `treasury` - New treasury address
    ///
    /// # Errors
    /// * `InsufficientPermissions` - If caller doesn't have Admin role
    pub fn set_treasury(env: Env, caller: Address, treasury: Address) -> Result<(), PrediFiError> {
        // Check if caller has Admin role
        let access_control_client = Self::get_access_control_client(&env);
        if !access_control_client.has_role(&caller, &Role::Admin) {
            return Err(PrediFiError::InsufficientPermissions);
        }

        env.storage().instance().set(&DataKey::Treasury, &treasury);
        SetTreasuryEvent {
            new_treasury: treasury.clone(),
        }
        .publish(&env);
        Ok(())
    }

    /// Get the treasury address.
    ///
    /// # Returns
    /// The treasury address
    pub fn get_treasury(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Treasury)
            .expect("Treasury not set")
    }

    /// Create a new prediction pool.
    pub fn create_pool(
        env: Env,
        creator: Address,
        end_time: u64,
        token: Address,
        category: soroban_sdk::Symbol,
        options_count: u32,
        min_stake: i128,
    ) -> Result<u64, PrediFiError> {
        // 1. Authorization
        creator.require_auth();

        // 2. Validate Parameters
        let current_time = env.ledger().timestamp();
        if end_time <= current_time {
            return Err(PrediFiError::EndTimeMustBeFuture);
        }

        if options_count < 2 {
            return Err(PrediFiError::InvalidOptionsCount);
        }

        // 3. Generate Unique ID
        let pool_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PoolIdCounter)
            .unwrap_or(0);

        // 4. Initialize Pool with ALL fields
        let pool = Pool {
            creator: creator.clone(), // Field added here
            end_time,
            resolved: false,
            outcome: 0,
            token,
            total_stake: 0,
            min_stake,
            category: category.clone(), // Field added here
            options_count,              // Field added here
        };

        // 5. Save State
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);
        env.storage()
            .instance()
            .set(&DataKey::PoolIdCounter, &(pool_id + 1));

        // 6. Emit Event
        PoolCreatedEvent {
            pool_id,
            creator,
            end_time,
        }
        .publish(&env);

        Ok(pool_id)
    }

    /// Resolve a prediction pool with the final outcome.
    ///
    /// # Arguments
    /// * `caller` - The address calling the function
    /// * `pool_id` - ID of the pool to resolve
    /// * `outcome` - The winning outcome number
    ///
    /// # Errors
    /// * `PoolNotFound` - If the pool doesn't exist
    /// * `PoolAlreadyResolved` - If the pool has already been resolved
    /// * `PoolNotExpired` - If the pool end time hasn't been reached
    /// * `ResolutionWindowExpired` - If the resolution window has passed
    /// * `InsufficientPermissions` - If caller doesn't have Oracle role
    pub fn resolve_pool(
        env: Env,
        caller: Address,
        pool_id: u64,
        outcome: u32,
    ) -> Result<(), PrediFiError> {
        // Check if caller has Oracle role
        let access_control_client = Self::get_access_control_client(&env);
        if !access_control_client.has_role(&caller, &Role::Oracle) {
            return Err(PrediFiError::InsufficientPermissions);
        }

        let mut pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .ok_or(PrediFiError::PoolNotFound)?;

        if pool.resolved {
            return Err(PrediFiError::PoolAlreadyResolved);
        }

        let current_time = env.ledger().timestamp();

        // Pool must have ended
        if current_time < pool.end_time {
            return Err(PrediFiError::PoolNotExpired);
        }

        // Resolution must happen within the window
        if current_time > pool.end_time + RESOLUTION_WINDOW {
            return Err(PrediFiError::ResolutionWindowExpired);
        }

        pool.resolved = true;
        pool.outcome = outcome;
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);

        // Calculate and store fee for this pool
        let fee_bps: u32 = env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0);
        if fee_bps > 0 && pool.total_stake > 0 {
            let fee = pool
                .total_stake
                .checked_mul(fee_bps as i128)
                .and_then(|v| v.checked_div(10_000))
                .ok_or(PrediFiError::ArithmeticOverflow)?;

            env.storage()
                .instance()
                .set(&DataKey::CollectedFees(pool_id), &fee);

            FeeCollectedEvent { pool_id, fee }.publish(&env);
        }

        Ok(())
    }

    /// Place a prediction on a pool.
    ///
    /// # Arguments
    /// * `caller` - The address calling the function (should match user)
    /// * `user` - Address of the user placing the prediction
    /// * `pool_id` - ID of the pool
    /// * `amount` - Amount to stake (must be positive)
    /// * `outcome` - The outcome being predicted
    ///
    /// # Errors
    /// * `PoolNotFound` - If the pool doesn't exist
    /// * `InvalidPredictionAmount` - If amount is zero or negative
    /// * `PredictionTooLate` - If pool has already ended
    /// * `PoolAlreadyResolved` - If pool is already resolved
    /// * `PredictionAlreadyExists` - If user already has a prediction on this pool
    /// * `InsufficientPermissions` - If caller doesn't have User role or doesn't match user
    pub fn place_prediction(
        env: Env,
        caller: Address,
        user: Address,
        pool_id: u64,
        amount: i128,
        outcome: u32,
    ) -> Result<(), PrediFiError> {
        // Check if caller has User role and matches the user address
        let access_control_client = Self::get_access_control_client(&env);
        if !access_control_client.has_role(&caller, &Role::User) || caller != user {
            return Err(PrediFiError::InsufficientPermissions);
        }

        user.require_auth();

        let mut pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .ok_or(PrediFiError::PoolNotFound)?;

        // Validate amount
        if amount <= 0 || amount < pool.min_stake {
            return Err(PrediFiError::InvalidPredictionAmount);
        }

        // Check if pool is resolved
        if pool.resolved {
            return Err(PrediFiError::PoolAlreadyResolved);
        }

        // Check if pool has ended
        let current_time = env.ledger().timestamp();
        if current_time >= pool.end_time {
            return Err(PrediFiError::PredictionTooLate);
        }

        // Check if user already has a prediction
        if env
            .storage()
            .instance()
            .has(&DataKey::Prediction(user.clone(), pool_id))
        {
            return Err(PrediFiError::PredictionAlreadyExists);
        }

        // Transfer tokens to contract
        let token_client = token::Client::new(&env, &pool.token);
        let contract_addr = env.current_contract_address();
        token_client.transfer(&user, &contract_addr, &amount);

        // Record prediction
        let prediction = Prediction { amount, outcome };
        env.storage()
            .instance()
            .set(&DataKey::Prediction(user.clone(), pool_id), &prediction);

        // Update total pool stake
        pool.total_stake = pool
            .total_stake
            .checked_add(amount)
            .ok_or(PrediFiError::ArithmeticOverflow)?;
        env.storage().instance().set(&DataKey::Pool(pool_id), &pool);

        // Update stake specific to this outcome
        let outcome_key = DataKey::OutcomeStake(pool_id, outcome);
        let current_outcome_stake: i128 = env.storage().instance().get(&outcome_key).unwrap_or(0);
        let new_outcome_stake = current_outcome_stake
            .checked_add(amount)
            .ok_or(PrediFiError::ArithmeticOverflow)?;
        env.storage()
            .instance()
            .set(&outcome_key, &new_outcome_stake);

        // Index user's prediction for pagination
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::UserPredictionCount(user.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::UserPredictionIndex(user.clone(), count), &pool_id);
        let new_count = count
            .checked_add(1)
            .ok_or(PrediFiError::ArithmeticOverflow)?;
        env.storage()
            .instance()
            .set(&DataKey::UserPredictionCount(user.clone()), &new_count);

        PredictionPlacedEvent {
            pool_id,
            user,
            amount,
            outcome,
        }
        .publish(&env);

        Ok(())
    }

    /// Claim winnings from a resolved pool.
    ///
    /// # Arguments
    /// * `user` - Address of the user claiming winnings
    /// * `pool_id` - ID of the pool
    ///
    /// # Returns
    /// The net amount of winnings claimed (after fees)
    ///
    /// # Errors
    /// * `PoolNotFound` - If the pool doesn't exist
    /// * `PoolNotResolved` - If the pool hasn't been resolved yet
    /// * `AlreadyClaimed` - If the user already claimed from this pool
    /// * `PredictionNotFound` - If the user has no prediction on this pool
    /// * `NotAWinner` - If the user's prediction didn't match the winning outcome
    /// * `WinningStakeZero` - If there's a critical error with winning stake calculation
    pub fn claim_winnings(env: Env, user: Address, pool_id: u64) -> Result<i128, PrediFiError> {
        user.require_auth();

        // 1. Validate pool exists and is resolved
        let pool: Pool = env
            .storage()
            .instance()
            .get(&DataKey::Pool(pool_id))
            .ok_or(PrediFiError::PoolNotFound)?;

        if !pool.resolved {
            return Err(PrediFiError::PoolNotResolved);
        }

        // 2. Prevent double claiming
        if env
            .storage()
            .instance()
            .has(&DataKey::HasClaimed(user.clone(), pool_id))
        {
            return Err(PrediFiError::AlreadyClaimed);
        }

        // 3. Get user prediction
        let prediction: Prediction = env
            .storage()
            .instance()
            .get(&DataKey::Prediction(user.clone(), pool_id))
            .ok_or(PrediFiError::PredictionNotFound)?;

        // 4. Check if user won
        if prediction.outcome != pool.outcome {
            return Err(PrediFiError::NotAWinner);
        }

        // 5. Calculate winnings: Share = (User Stake / Total Winning Stake) * Total Pool Stake
        let outcome_key = DataKey::OutcomeStake(pool_id, pool.outcome);
        let winning_outcome_stake: i128 = env.storage().instance().get(&outcome_key).unwrap_or(0);

        if winning_outcome_stake == 0 {
            return Err(PrediFiError::WinningStakeZero);
        }

        let gross_winnings = prediction
            .amount
            .checked_mul(pool.total_stake)
            .ok_or(PrediFiError::ArithmeticOverflow)?
            .checked_div(winning_outcome_stake)
            .ok_or(PrediFiError::DivisionByZero)?;

        // 6. Deduct fee proportionally from winnings
        let fee_bps: u32 = env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0);
        let mut fee_share = 0i128;

        if fee_bps > 0 && pool.total_stake > 0 {
            let total_fee: i128 = env
                .storage()
                .instance()
                .get(&DataKey::CollectedFees(pool_id))
                .unwrap_or(0);

            // User's share of fee = (user's gross winnings / total pool stake) * total_fee
            fee_share = gross_winnings
                .checked_mul(total_fee)
                .ok_or(PrediFiError::ArithmeticOverflow)?
                .checked_div(pool.total_stake)
                .ok_or(PrediFiError::DivisionByZero)?;
        }

        let net_winnings = gross_winnings
            .checked_sub(fee_share)
            .ok_or(PrediFiError::ArithmeticOverflow)?;

        // 7. Transfer net winnings to user
        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&env.current_contract_address(), &user, &net_winnings);

        // 8. Update claim status
        env.storage()
            .instance()
            .set(&DataKey::HasClaimed(user.clone(), pool_id), &true);

        // 9. On first claim, transfer accumulated fees to treasury
        if fee_bps > 0 && pool.total_stake > 0 {
            let total_fee: i128 = env
                .storage()
                .instance()
                .get(&DataKey::CollectedFees(pool_id))
                .unwrap_or(0);

            if total_fee > 0 {
                // Use a marker to track if fee has been distributed
                let marker_addr = env.current_contract_address();
                let fee_paid_key = DataKey::HasClaimed(marker_addr, pool_id);

                if !env.storage().instance().has(&fee_paid_key) {
                    let treasury: Address = env
                        .storage()
                        .instance()
                        .get(&DataKey::Treasury)
                        .expect("Treasury not set");

                    token_client.transfer(&env.current_contract_address(), &treasury, &total_fee);
                    env.storage().instance().set(&fee_paid_key, &true);

                    FeeDistributedEvent {
                        pool_id,
                        fee: total_fee,
                    }
                    .publish(&env);
                }
            }
        }

        Ok(net_winnings)
    }

    /// Get a paginated list of user's predictions.
    ///
    /// # Arguments
    /// * `user` - Address of the user
    /// * `offset` - Starting index for pagination
    /// * `limit` - Maximum number of predictions to return
    ///
    /// # Returns
    /// A vector of UserPredictionDetail structs
    ///
    /// # Errors
    /// * `InvalidLimit` - If limit is zero
    /// * `StorageKeyNotFound` - If expected storage keys are missing
    /// * `PredictionNotFound` - If a prediction cannot be retrieved
    /// * `PoolNotFound` - If a pool cannot be retrieved
    pub fn get_user_predictions(
        env: Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> Result<soroban_sdk::Vec<UserPredictionDetail>, PrediFiError> {
        if limit == 0 {
            return Err(PrediFiError::InvalidLimit);
        }

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::UserPredictionCount(user.clone()))
            .unwrap_or(0);

        let mut results = soroban_sdk::Vec::new(&env);

        if offset >= count {
            return Ok(results);
        }

        let end = core::cmp::min(
            offset
                .checked_add(limit)
                .ok_or(PrediFiError::ArithmeticOverflow)?,
            count,
        );

        for i in offset..end {
            let pool_id: u64 = env
                .storage()
                .instance()
                .get(&DataKey::UserPredictionIndex(user.clone(), i))
                .ok_or(PrediFiError::StorageKeyNotFound)?;

            let prediction: Prediction = env
                .storage()
                .instance()
                .get(&DataKey::Prediction(user.clone(), pool_id))
                .ok_or(PrediFiError::PredictionNotFound)?;

            let pool: Pool = env
                .storage()
                .instance()
                .get(&DataKey::Pool(pool_id))
                .ok_or(PrediFiError::PoolNotFound)?;

            results.push_back(UserPredictionDetail {
                pool_id,
                amount: prediction.amount,
                user_outcome: prediction.outcome,
                pool_end_time: pool.end_time,
                pool_resolved: pool.resolved,
                pool_outcome: pool.outcome,
            });
        }

        Ok(results)
    }
}

mod test;
