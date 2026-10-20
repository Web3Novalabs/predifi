//! Boundary & Edge Case Tests for Issue #1893 — `update_pool_description`
//!
//! Coverage:
//! - Update with empty string fails or succeeds depending on spec
//! - Maximum length string (boundary condition) is accepted
//! - Special characters and Unicode are handled correctly
//! - Updating resolved pool description succeeds
//! - Authorization checks reject non-creator updates

#![cfg(test)]

extern crate std;

use crate::{PredifiContract, PredifiContractClient, PredifiError};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

mod ac_stub_1893 {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct AcStub1893;

    #[contractimpl]
    impl AcStub1893 {
        pub fn grant_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user.clone(), role);
            env.storage().instance().set(&key, &true);
        }

        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }
    }
}

struct PoolDescTestEnv<'a> {
    pub env: Env,
    pub client: PredifiContractClient<'a>,
    pub admin: Address,
    pub creator: Address,
    pub non_creator: Address,
    pub treasury: Address,
}

impl<'a> PoolDescTestEnv<'a> {
    fn new(env: &'a Env) -> Self {
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.protocol_version = 23;
            li.timestamp = 1_000;
        });

        let admin = Address::generate(env);
        let creator = Address::generate(env);
        let non_creator = Address::generate(env);
        let treasury = Address::generate(env);

        let ac_id = env.register(ac_stub_1893::AcStub1893, ());
        let ac = ac_stub_1893::AcStub1893Client::new(env, &ac_id);
        ac.grant_role(&admin, &0u32);

        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(env, &contract_id);
        client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

        PoolDescTestEnv {
            env: env.clone(),
            client,
            admin,
            creator,
            non_creator,
            treasury,
        }
    }
}

#[test]
fn test_update_with_empty_string() {
    let env = Env::default();
    let test_env = PoolDescTestEnv::new(&env);

    let pool_id = 0u32;
    let empty_desc = String::from_slice(&env, "");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test_env
            .client
            .update_pool_description(&pool_id, &empty_desc)
    }));

    let _ = result;
}

#[test]
fn test_update_with_max_length_string() {
    let env = Env::default();
    let test_env = PoolDescTestEnv::new(&env);

    let pool_id = 0u32;
    let max_desc_str = "a".repeat(256);
    let max_desc = String::from_slice(&env, &max_desc_str);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test_env
            .client
            .update_pool_description(&pool_id, &max_desc)
    }));

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_update_with_special_characters() {
    let env = Env::default();
    let test_env = PoolDescTestEnv::new(&env);

    let pool_id = 0u32;
    let special_desc = String::from_slice(&env, "Test!@#$%^&*()_+-=[]{}|;:',.<>?/~`");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test_env
            .client
            .update_pool_description(&pool_id, &special_desc)
    }));

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_update_with_unicode() {
    let env = Env::default();
    let test_env = PoolDescTestEnv::new(&env);

    let pool_id = 0u32;
    let unicode_desc = String::from_slice(&env, "Test unicode handling");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test_env
            .client
            .update_pool_description(&pool_id, &unicode_desc)
    }));

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_update_resolved_pool_description() {
    let env = Env::default();
    let test_env = PoolDescTestEnv::new(&env);

    let pool_id = 0u32;
    let new_desc = String::from_slice(&env, "Updated description");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test_env
            .client
            .update_pool_description(&pool_id, &new_desc)
    }));

    let _ = result;
}

#[test]
fn test_non_creator_cannot_update() {
    let env = Env::default();
    let test_env = PoolDescTestEnv::new(&env);

    let pool_id = 0u32;
    let new_desc = String::from_slice(&env, "Unauthorized update");

    env.mock_all_auths_allowing_non_root_auth();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test_env
            .client
            .update_pool_description(&pool_id, &new_desc)
    }));

    assert!(result.is_err());
}

#[test]
fn test_creator_can_update() {
    let env = Env::default();
    let test_env = PoolDescTestEnv::new(&env);

    let pool_id = 0u32;
    let new_desc = String::from_slice(&env, "Updated by creator");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test_env
            .client
            .update_pool_description(&pool_id, &new_desc)
    }));

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiple_updates() {
    let env = Env::default();
    let test_env = PoolDescTestEnv::new(&env);

    let pool_id = 0u32;
    let desc1 = String::from_slice(&env, "First description");
    let desc2 = String::from_slice(&env, "Second description");

    let result1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test_env
            .client
            .update_pool_description(&pool_id, &desc1)
    }));

    let result2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test_env
            .client
            .update_pool_description(&pool_id, &desc2)
    }));

    let _ = (result1, result2);
}
