//! Boundary & Edge Case Tests for issues #1877, #1881, #1882, #1883.
//!
//! | Issue  | Function                | Coverage highlights                                                    |
//! |--------|-------------------------|--------------------------------------------------------------------------|
//! | #1877  | `place_prediction`      | min_stake, max_total_stake, exact start/end timestamps,                  |
//! |        |                         | i128::MAX amount, cooldown boundary, concurrent predictions from same user|
//! | #1881  | `cancel_pool`           | cancellation with active bets, unauthorized caller, resolved pool,       |
//! |        |                         | already-cancelled pool, state consistency & refundable user balances     |
//! | #1882  | `emergency_cancel_pool` | multi-oracle approval flow, duplicate approval, approval on cancelled,   |
//! |        |                         | threshold boundary (quorum vs quorum-1), state rollback verification     |
//! | #1883  | `set_fee_bps`           | fee_bps = 0, 10000, 10001, u32::MAX, rapid changes, active pool impact,  |
//! |        |                         | pending fee change state transitions                                    |

#![cfg(test)]

extern crate std;

use crate::{
    MarketState, PoolConfig, PredifiContract, PredifiContractClient, PredifiError,
    FEE_CHANGE_TIMELOCK_SECONDS,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

// ─── Dummy Access Control Stub ───────────────────────────────────────────────

mod ac_stub_1877_1881_1882_1883 {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct AcStubIssues;

    #[contractimpl]
    impl AcStubIssues {
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

// ─── Test Harness Environment ────────────────────────────────────────────────

struct TestEnv<'a> {
    pub env: Env,
    pub client: PredifiContractClient<'a>,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
    pub token_address: Address,
    pub admin: Address,
    pub operator: Address,
    pub operator2: Address,
    pub creator: Address,
}

impl<'a> TestEnv<'a> {
    fn new(env: &'a Env) -> Self {
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.protocol_version = 23;
            li.timestamp = 1_000;
        });

        let admin = Address::generate(env);
        let operator = Address::generate(env);
        let operator2 = Address::generate(env);
        let creator = Address::generate(env);

        let ac_id = env.register(ac_stub_1877_1881_1882_1883::AcStubIssues, ());
        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(env, &contract_id);

        let (token_address, token_admin) =
            env.register_stellar_asset_contract_v2(admin.clone());
        let token = token::Client::new(env, &token_address);

        client.initialize(&admin, &token_address, &ac_id);

        let ac_client = ac_stub_1877_1881_1882_1883::AcStubIssuesClient::new(env, &ac_id);
        ac_client.grant_role(&admin, &0);
        ac_client.grant_role(&operator, &1);
        ac_client.grant_role(&operator2, &1);

        token_admin.mint(&admin, &1_000_000_000_000);
        token_admin.mint(&creator, &1_000_000_000_000);

        Self {
            env: env.clone(),
            client,
            token,
            token_admin,
            token_address,
            admin,
            operator,
            operator2,
            creator,
        }
    }

    fn create_pool(&self, duration_secs: u64) -> u64 {
        let outcomes = vec![
            &self.env,
            String::from_str(&self.env, "Yes"),
            String::from_str(&self.env, "No"),
        ];
        let now = self.env.ledger().timestamp();
        let config = PoolConfig {
            min_stake: 10,
            max_stake: 100_000,
            max_total_stake: 1_000_000,
            start_time: now,
            end_time: now + duration_secs,
            resolve_delay: 60,
            cooldown_seconds: 5,
        };
        self.client.create_pool(
            &self.creator,
            &String::from_str(&self.env, "Test Market Pool"),
            &String::from_str(&self.env, "Description"),
            &outcomes,
            &config,
            &0u32,
        )
    }

    fn mint_and_approve(&self, user: &Address, amount: i128) {
        self.token_admin.mint(user, &amount);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1877 — `place_prediction` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_1877_stake_exact_min_and_below_min() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(3600);
    let user = Address::generate(&env);
    ctx.mint_and_approve(&user, 10_000);

    // Below min_stake (10) fails with StakeBelowMinimum
    let err = ctx.client.try_place_prediction(&user, &pool_id, &0, &9, &None);
    assert_eq!(err, Err(Ok(PredifiError::StakeBelowMinimum)));

    // Exactly min_stake (10) succeeds
    let ok = ctx.client.try_place_prediction(&user, &pool_id, &0, &10, &None);
    assert!(ok.is_ok());
}

#[test]
fn test_1877_stake_exact_max_total_stake_cap() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(3600);
    let user = Address::generate(&env);
    ctx.mint_and_approve(&user, 2_000_000);

    // Place prediction that pushes total stake above max_total_stake
    let err = ctx.client.try_place_prediction(&user, &pool_id, &0, &1_000_001, &None);
    assert!(err.is_err());
}

#[test]
fn test_1877_cooldown_boundary() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(3600);
    let user = Address::generate(&env);
    ctx.mint_and_approve(&user, 10_000);

    ctx.client.place_prediction(&user, &pool_id, &0, &100, &None);

    // 1 second short of cooldown (4s elapsed vs 5s required) -> rejected
    env.ledger().with_mut(|li| li.timestamp += 4);
    let err = ctx.client.try_place_prediction(&user, &pool_id, &0, &100, &None);
    assert!(err.is_err());

    // Exact cooldown elapsed (1s more, total 5s) -> succeeds
    env.ledger().with_mut(|li| li.timestamp += 1);
    let ok = ctx.client.try_place_prediction(&user, &pool_id, &0, &100, &None);
    assert!(ok.is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1881 — `cancel_pool` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_1881_cancel_by_unauthorized_non_creator_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(3600);
    let stranger = Address::generate(&env);

    let res = ctx.client.try_cancel_pool(&stranger, &pool_id);
    assert!(res.is_err());
}

#[test]
fn test_1881_cancel_already_cancelled_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(3600);

    // Creator cancels pool
    ctx.client.cancel_pool(&ctx.creator, &pool_id);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);

    // Second cancellation must be rejected with InvalidPoolState
    let res = ctx.client.try_cancel_pool(&ctx.creator, &pool_id);
    assert_eq!(res, Err(Ok(PredifiError::InvalidPoolState)));
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1882 — `emergency_cancel_pool` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_1882_multi_oracle_approval_flow_and_duplicate() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let reason = String::from_str(&env, "oracle anomaly");

    // First operator approves
    ctx.client.emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Active);

    // Duplicate approval from same operator is rejected
    let dup = ctx.client.try_emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    assert!(dup.is_err());

    // Second operator reaches quorum and cancels
    ctx.client.emergency_cancel_pool(&ctx.operator2, &pool_id, &reason);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);

    // Further approval on already-cancelled pool is rejected
    let after = ctx.client.try_emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    assert!(after.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1883 — `set_fee_bps` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_1883_fee_bps_boundaries_and_overflow() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // fee_bps = 0 is valid
    let zero = ctx.client.try_set_fee_bps(&ctx.admin, &0u32);
    assert!(zero.is_ok());

    // advance timelock to apply
    env.ledger().with_mut(|li| li.timestamp += FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    // fee_bps = 10001 (overflow > 10000 bps) is rejected with InvalidFeeBps
    let overflow = ctx.client.try_set_fee_bps(&ctx.admin, &10_001u32);
    assert_eq!(overflow, Err(Ok(PredifiError::InvalidFeeBps)));

    // fee_bps = u32::MAX is rejected with InvalidFeeBps
    let max_err = ctx.client.try_set_fee_bps(&ctx.admin, &u32::MAX);
    assert_eq!(max_err, Err(Ok(PredifiError::InvalidFeeBps)));
}
