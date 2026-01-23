#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, vec, Env, String, Vec};

/// Represents the current status of a prediction pool.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolStatus {
    /// The pool is open for predictions.
    Active,
    /// The event has occurred and the outcome is determined.
    Resolved,
    /// The pool is closed for new predictions but not yet resolved.
    Closed,
    /// The outcome is being disputed.
    Disputed,
}

/// Specific errors for the PrediFi contract.
#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    InvalidEndTime = 1,
    PoolExists = 2,
    PoolNotFound = 3,
    DeadlinePassed = 4,
    PoolNotActive = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Pool(u64), // Pool ID -> Pool struct
}

/// A prediction pool structure containing status and timing information.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pool {
    pub status: PoolStatus,
    /// The timestamp (in seconds) when the pool stops accepting predictions.
    pub end_time: u64,
}

impl Pool {
    /// Checks if the pool is currently active.
    ///
    /// # Returns
    /// * `true` if the pool status is `Active`.
    pub fn is_pool_active(&self) -> bool {
        self.status == PoolStatus::Active
    }

    /// Checks if the pool has been resolved.
    ///
    /// # Returns
    /// * `true` if the pool status is `Resolved`.
    pub fn is_pool_resolved(&self) -> bool {
        self.status == PoolStatus::Resolved
    }

    /// Determines if the pool can accept new predictions.
    ///
    /// A pool can accept predictions if it is `Active` and the current
    /// ledger timestamp is before the pool's `end_time`.
    ///
    /// # Arguments
    /// * `env` - The current contract environment.
    pub fn can_accept_predictions(&self, env: &Env) -> bool {
        if !self.is_pool_active() {
            return false;
        }
        env.ledger().timestamp() < self.end_time
    }

    /// Validates if a transition to a new status is allowed.
    ///
    /// # Arguments
    /// * `new_status` - The target status to transition to.
    ///
    /// # Returns
    /// * `true` if the transition is valid according to the state machine rules.
    pub fn validate_state_transition(&self, new_status: PoolStatus) -> bool {
        match (self.status, new_status) {
            (PoolStatus::Active, PoolStatus::Resolved) => true,
            (PoolStatus::Active, PoolStatus::Closed) => true,
            (PoolStatus::Resolved, PoolStatus::Disputed) => true,
            _ => false,
        }
    }
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Creates a new prediction pool with a specified deadline.
    ///
    /// # Arguments
    /// * `pool_id` - A unique identifier for the pool.
    /// * `end_time` - The timestamp (in seconds) when the pool stops accepting predictions.
    ///
    /// # Returns
    /// * `Ok(())` on success, or an `Error` on failure.
    pub fn create_pool(env: Env, pool_id: u64, end_time: u64) -> Result<(), Error> {
        let key = DataKey::Pool(pool_id);
        
        if env.storage().persistent().has(&key) {
            return Err(Error::PoolExists);
        }

        if end_time <= env.ledger().timestamp() {
            return Err(Error::InvalidEndTime);
        }

        let pool = Pool {
            status: PoolStatus::Active,
            end_time,
        };

        env.storage().persistent().set(&key, &pool);
        Ok(())
    }

    /// Submits a prediction to a specific pool.
    ///
    /// # Arguments
    /// * `pool_id` - The identifier of the pool to predict on.
    ///
    /// # Returns
    /// * `Ok(())` if the prediction is valid, or an `Error` on failure.
    pub fn submit_prediction(env: Env, pool_id: u64) -> Result<(), Error> {
        let key = DataKey::Pool(pool_id);
        
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::PoolNotFound)?;

        if !pool.is_pool_active() {
            return Err(Error::PoolNotActive);
        }

        if !pool.can_accept_predictions(&env) {
            return Err(Error::DeadlinePassed);
        }

        // Logic for recording the prediction would go here
        Ok(())
    }

    /// Retrieves pool information.
    pub fn get_pool(env: Env, pool_id: u64) -> Option<Pool> {
        let key = DataKey::Pool(pool_id);
        env.storage().persistent().get(&key)
    }

    pub fn hello(env: Env, to: String) -> Vec<String> {
        vec![&env, String::from_str(&env, "Hello"), to]
    }
}

mod test;
#[cfg(test)]
mod test_pool;

