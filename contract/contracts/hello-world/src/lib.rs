#![no_std]
use soroban_sdk::{contract, contractimpl, vec, Env, String, Vec};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolStatus {
    Active,
    Resolved,
    Closed,
    Disputed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pool {
    pub status: PoolStatus,
    pub end_time: u64,
}

#[contractimpl]
impl Pool {
    pub fn is_pool_active(&self) -> bool {
        self.status == PoolStatus::Active
    }

    pub fn is_pool_resolved(&self) -> bool {
        self.status == PoolStatus::Resolved
    }

    pub fn can_accept_predictions(&self, env: &Env) -> bool {
        if !self.is_pool_active() {
            return false;
        }
        env.ledger().timestamp() < self.end_time
    }

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
    pub fn hello(env: Env, to: String) -> Vec<String> {
        vec![&env, String::from_str(&env, "Hello"), to]
    }
}

mod test;
mod test_pool;

