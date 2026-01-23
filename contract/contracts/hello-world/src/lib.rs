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
        matches!(
            (self.status, new_status),
            (PoolStatus::Active, PoolStatus::Resolved)
                | (PoolStatus::Active, PoolStatus::Closed)
                | (PoolStatus::Resolved, PoolStatus::Disputed)
        )
    }
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn hello(env: Env, to: String) -> Vec<String> {
        vec![&env, String::from_str(&env, "Hello"), to]
    }
}

mod test;
mod test_pool;
