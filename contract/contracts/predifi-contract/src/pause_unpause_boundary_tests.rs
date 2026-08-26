//! Boundary & Edge Case Tests for Issue #1528 — `pause` / `unpause`
//!
//! Coverage:
//! - Pausing an already-paused contract must fail
//! - Unpausing an already-unpaused contract must fail
//! - All state-mutating operations are blocked while paused
//! - Non-admin cannot pause or unpause
//! - State consistency is preserved across pause / unpause cycles

#![cfg(test)]

extern crate std;

use crate::{MarketState, PoolConfig, PredifiContract, PredifiContractClient, PredifiError};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

// ─── Shared dummy access-control stub ────────────────────────────────────────

mod ac_stub_1528 {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct AcStub1528;

    #[contractimpl]
    impl AcStub1528 {
        pub fn grant_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user.clone(), role);
            let already: bool = env.storage().instance().get(&key).unwrap_or(false);
            env.storage().instance().set(&key, &true);
            if role == 1 && !already {
                let ck = Symbol::new(&env, "op_count");
                let c: u32 = env.storage().instance().get(&ck).unwrap_or(0);
                env.storage().instance().set(&ck, &(c + 1));
            }
        }

        pub fn revoke_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user, role);
            let had: bool = env.storage().instance().get(&key).unwrap_or(false);
            env.storage().instance().set(&key, &false);
            if role == 1 && had {
                let ck = Symbol::new(&env, "op_count");
                let c: u32 = env.storage().instance().get(&ck).unwrap_or(0);
                if c > 0 {
                    env.storage().instance().set(&ck, &(c - 1));
                }
            }
        }

        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }

        pub fn get_operator_count(env: Env) -> u32 {
            env.storage()
                .instance()
                .get(&Symbol::new(&env, "op_count"))
                .unwrap_or(0)
        }
    }
}

// ─── Test harness ─────────────────────────────────────────────────────────────

struct PauseTestEnv<'a> {
    pub env: Env,
    pub client: PredifiContractClient<'a>,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
    pub token_address: Address,
    pub admin: Address,
    pub operator: Address,
    pub creator: Address,
    pub treasury: Address,
}

impl<'a> PauseTestEnv<'a> {
    fn new(env: &'a Env) -> Self {
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.protocol_version = 23;
            li.timestamp = 1_000;
        });

        let admin = Address::generate(env);
        let operator = Address::generate(env);
        let creator = Address::generate(env);
        let treasury = Address::generate(env);

        let ac_id = env.register(ac_stub_1528::AcStub1528, ());
        let ac = ac_stub_1528::AcStub1528Client::new(env, &ac_id);
        ac.grant_role(&admin, &0u32);
        ac.grant_role(&operator, &1u32);

        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(env, &contract_id);
        client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

        let token_admin_addr = Address::generate(env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr);
        let token_address = token_contract.address();
        let token = token::Client::new(env, &token_address);
        let token_admin = token::StellarAssetClient::new(env, &token_address);

        client.add_token_to_whitelist(&admin, &token_address);

        Self {
            env: env.clone(),
            client,
            token,
            token_admin,
            token_address,
            admin,
            operator,
            creator,
            treasury,
        }
    }

    fn create_pool(&self, end_time_offset: u64) -> u64 {
        let now = self.env.ledger().timestamp();
        self.client.create_pool(
            &self.creator,
            &(now + end_time_offset),
            &self.token_address,
            &2u32,
            &symbol_short!("Tech"),
            &PoolConfig {
                start_time: 0,
                description: String::from_str(&self.env, "Pause boundary test pool"),
                metadata_url: String::from_str(&self.env, "ipfs://pause-test"),
                min_stake: 1i128,
                max_stake: 0i128,
                max_total_stake: 0i128,
                min_total_stake: 0i128,
                initial_liquidity: 0i128,
                required_resolutions: 1u32,
                private: false,
                whitelist_key: None,
                outcome_descriptions: vec![
                    &self.env,
                    String::from_str(&self.env, "No"),
                    String::from_str(&self.env, "Yes"),
                ],
            },
        )
    }

    fn advance_time(&self, seconds: u64) {
        let current = self.env.ledger().timestamp();
        self.env
            .ledger()
            .with_mut(|li| li.timestamp = current + seconds);
    }

    fn stake(&self, user: &Address, pool_id: u64, amount: i128, outcome: u32) {
        self.token_admin.mint(user, &amount);
        self.client
            .place_prediction(user, &pool_id, &amount, &outcome, &None, &None);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// #1528 — Pause / Unpause Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Pausing an already-paused contract must fail. The contract uses a hard
/// `panic!` for this guard; `try_pause` wraps the call and returns `Err`.
#[test]
fn test_1528_pause_already_paused_contract_fails() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    // Contract starts unpaused.
    assert!(!ctx.client.is_contract_paused());

    // First pause must succeed.
    ctx.client.pause(&ctx.admin);
    assert!(ctx.client.is_contract_paused());

    // Second pause while already paused must fail.
    assert!(
        ctx.client.try_pause(&ctx.admin).is_err(),
        "pausing an already-paused contract must fail"
    );
    // Pause state must remain true.
    assert!(ctx.client.is_contract_paused());
}

/// Unpausing an already-unpaused contract must fail.
#[test]
fn test_1528_unpause_already_unpaused_contract_fails() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    // Contract starts unpaused — unpausing must fail immediately.
    assert!(!ctx.client.is_contract_paused());

    assert!(
        ctx.client.try_unpause(&ctx.admin).is_err(),
        "unpausing an already-unpaused contract must fail"
    );
    assert!(!ctx.client.is_contract_paused());
}

/// While the contract is paused, `place_prediction` must return `ContractPaused`.
#[test]
fn test_1528_place_prediction_blocked_while_paused() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000i128);

    ctx.client.pause(&ctx.admin);
    assert!(ctx.client.is_contract_paused());

    let result = ctx.client.try_place_prediction(
        &user,
        &pool_id,
        &1_000i128,
        &0u32,
        &None,
        &None,
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::ContractPaused)),
        "place_prediction must return ContractPaused when contract is paused"
    );
}

/// While the contract is paused, `create_pool` must return `ContractPaused`.
#[test]
fn test_1528_create_pool_blocked_while_paused() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    ctx.client.pause(&ctx.admin);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &(ctx.env.ledger().timestamp() + 7_200),
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Blocked pool"),
            metadata_url: String::from_str(&env, "ipfs://blocked"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "No"),
                String::from_str(&env, "Yes"),
            ],
        },
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::ContractPaused)),
        "create_pool must return ContractPaused when contract is paused"
    );
}

/// While the contract is paused, `resolve_pool` must return `ContractPaused`.
#[test]
fn test_1528_resolve_pool_blocked_while_paused() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    let pool_id = ctx.create_pool(2_000);
    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 500, 0);
    ctx.advance_time(2_001);

    ctx.client.pause(&ctx.admin);

    let result = ctx
        .client
        .try_resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(
        result,
        Err(Ok(PredifiError::ContractPaused)),
        "resolve_pool must return ContractPaused when contract is paused"
    );
}

/// While the contract is paused, `cancel_pool` must return `ContractPaused`.
#[test]
fn test_1528_cancel_pool_blocked_while_paused() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    ctx.client.pause(&ctx.admin);

    let result = ctx.client.try_cancel_pool(
        &ctx.operator,
        &pool_id,
        &String::from_str(&env, "test cancel"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::ContractPaused)),
        "cancel_pool must return ContractPaused when contract is paused"
    );
}

/// While the contract is paused, `claim_winnings` must return `ContractPaused`.
#[test]
fn test_1528_claim_winnings_blocked_while_paused() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    let pool_id = ctx.create_pool(2_000);
    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 1_000, 0);
    ctx.advance_time(2_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);

    ctx.client.pause(&ctx.admin);

    let result = ctx.client.try_claim_winnings(&user, &pool_id);
    assert_eq!(
        result,
        Err(Ok(PredifiError::ContractPaused)),
        "claim_winnings must return ContractPaused when contract is paused"
    );
}

/// A non-admin address must not be able to pause the contract.
/// `try_pause` returns `Err` because the contract panics on auth failure.
#[test]
fn test_1528_non_admin_cannot_pause() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    let stranger = Address::generate(&env);

    assert!(
        ctx.client.try_pause(&stranger).is_err(),
        "non-admin must not be able to pause the contract"
    );
    assert!(
        !ctx.client.is_contract_paused(),
        "contract must remain unpaused after unauthorized pause attempt"
    );
}

/// A non-admin address must not be able to unpause the contract.
#[test]
fn test_1528_non_admin_cannot_unpause() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    // Pause by the legitimate admin first.
    ctx.client.pause(&ctx.admin);
    assert!(ctx.client.is_contract_paused());

    let stranger = Address::generate(&env);
    assert!(
        ctx.client.try_unpause(&stranger).is_err(),
        "non-admin must not be able to unpause the contract"
    );
    assert!(
        ctx.client.is_contract_paused(),
        "contract must remain paused after unauthorized unpause attempt"
    );
}

/// State created before a pause (pool data, predictions) must be intact after
/// an unpause. The pause/unpause cycle must not corrupt on-chain data.
#[test]
fn test_1528_state_consistency_across_pause_unpause_cycle() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    // Create a pool and place a prediction before pausing.
    let pool_id = ctx.create_pool(7_200);
    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 1_000, 1);

    let pool_before = ctx.client.get_pool(&pool_id);
    assert_eq!(pool_before.total_stake, 1_000);
    assert_eq!(pool_before.state, MarketState::Active);

    // Pause → verify state unchanged.
    ctx.client.pause(&ctx.admin);
    let pool_during = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool_during.total_stake, pool_before.total_stake,
        "total_stake must not change during pause"
    );
    assert_eq!(
        pool_during.state, pool_before.state,
        "pool state must not change during pause"
    );

    // Unpause → pool state must be exactly restored.
    ctx.client.unpause(&ctx.admin);
    assert!(!ctx.client.is_contract_paused());

    let pool_after = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool_after.total_stake, pool_before.total_stake,
        "total_stake must be identical after unpause"
    );
    assert_eq!(
        pool_after.state, pool_before.state,
        "pool state must be identical after unpause"
    );
}

/// Multiple pause / unpause cycles (three cycles) must all complete without
/// state corruption, and operations must work normally between cycles.
#[test]
fn test_1528_multiple_pause_unpause_cycles_preserve_state() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);

    // Cycle 1.
    ctx.client.pause(&ctx.admin);
    assert!(ctx.client.is_contract_paused());
    ctx.client.unpause(&ctx.admin);
    assert!(!ctx.client.is_contract_paused());

    // After first cycle we can place a prediction.
    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 500, 0);
    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, 500);

    // Cycle 2.
    ctx.client.pause(&ctx.admin);
    assert!(ctx.client.is_contract_paused());
    // Prediction fails while paused.
    let blocked = ctx
        .client
        .try_place_prediction(&user, &pool_id, &100i128, &1u32, &None, &None);
    assert_eq!(blocked, Err(Ok(PredifiError::ContractPaused)));
    ctx.client.unpause(&ctx.admin);
    assert!(!ctx.client.is_contract_paused());

    // Stake must be unchanged after cycle 2.
    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, 500);

    // Cycle 3.
    ctx.client.pause(&ctx.admin);
    assert!(ctx.client.is_contract_paused());
    ctx.client.unpause(&ctx.admin);
    assert!(!ctx.client.is_contract_paused());

    // Pool is still intact and operable.
    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Active);
    assert_eq!(pool.total_stake, 500);
}

/// After unpausing, operations that were blocked while paused must now succeed.
/// This confirms the pause does not leave residual locks.
#[test]
fn test_1528_operations_resume_correctly_after_unpause() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);

    ctx.client.pause(&ctx.admin);

    // Blocked while paused.
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000i128);
    assert_eq!(
        ctx.client
            .try_place_prediction(&user, &pool_id, &1_000i128, &0u32, &None, &None),
        Err(Ok(PredifiError::ContractPaused))
    );

    ctx.client.unpause(&ctx.admin);

    // Same operation must now succeed.
    ctx.client
        .place_prediction(&user, &pool_id, &1_000i128, &0u32, &None, &None);
    assert_eq!(
        ctx.client.get_pool(&pool_id).total_stake,
        1_000,
        "prediction must be recorded after unpause"
    );
}

/// `is_contract_paused` must accurately reflect the pause state at every step.
#[test]
fn test_1528_is_contract_paused_reflects_correct_state() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    assert!(!ctx.client.is_contract_paused(), "starts unpaused");

    ctx.client.pause(&ctx.admin);
    assert!(ctx.client.is_contract_paused(), "must be paused after pause()");

    ctx.client.unpause(&ctx.admin);
    assert!(
        !ctx.client.is_contract_paused(),
        "must be unpaused after unpause()"
    );
}

/// `update_pool_description` must be blocked while the contract is paused
/// and succeed after unpausing. State must be consistent throughout.
#[test]
fn test_1528_update_pool_description_blocked_during_pause_restored_after_unpause() {
    let env = Env::default();
    let ctx = PauseTestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let original_desc = ctx.client.get_pool(&pool_id).description;

    ctx.client.pause(&ctx.admin);

    let blocked = ctx.client.try_update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "attempted while paused"),
    );
    assert_eq!(
        blocked,
        Err(Ok(PredifiError::ContractPaused)),
        "update_pool_description must be blocked while paused"
    );
    assert_eq!(
        ctx.client.get_pool(&pool_id).description,
        original_desc,
        "description must be unchanged after blocked update"
    );

    ctx.client.unpause(&ctx.admin);

    ctx.client.update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "updated after unpause"),
    );
    assert_eq!(
        ctx.client.get_pool(&pool_id).description,
        String::from_str(&env, "updated after unpause"),
        "description must be updated after unpausing"
    );
}
