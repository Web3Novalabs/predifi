//! Boundary & Edge Case Tests for issues #1438, #1441, #1444, #1445.
//!
//! | Issue  | Function                | Coverage highlights                                                    |
//! |--------|-------------------------|--------------------------------------------------------------------------|
//! | #1438  | `create_pool`           | zero duration, max u64 timestamp, empty/oversized description,          |
//! |        |                         | duplicate descriptions, invalid token, fee_bps 0/10000 not captured     |
//! |        |                         | at creation, error paths leave no partial state                        |
//! | #1441  | `claim_refund`          | cancelled-pool refund, multiple predictions accumulate, zero-stake      |
//! |        |                         | refund rejection, double-refund prevention, timing around cancellation  |
//! | #1444  | `emergency_cancel_pool` | quorum boundary (N-1 vs N), duplicate approval, approval after already  |
//! |        |                         | cancelled/resolved, unauthorized approver, state rollback verification  |
//! | #1445  | `set_fee_bps`           | fee_bps 0/10000/10001/u32::MAX, rapid successive changes, fee change    |
//! |        |                         | while pools are active, pending fee change state transitions           |

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

// ─── Shared dummy access-control stub ────────────────────────────────────────

mod ac_stub_1438_1441_1444_1445 {
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

// ─── Test environment setup ───────────────────────────────────────────────────

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
        let treasury = Address::generate(env);

        let ac_id = env.register(ac_stub_1438_1441_1444_1445::AcStubIssues, ());
        let ac = ac_stub_1438_1441_1444_1445::AcStubIssuesClient::new(env, &ac_id);
        ac.grant_role(&admin, &0u32);
        ac.grant_role(&operator, &1u32);
        ac.grant_role(&operator2, &1u32);

        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(env, &contract_id);
        // fee_bps = 0, resolution_delay = 0, min_pool_duration = 3600, max_pred_per_user = 0
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
            operator2,
            creator,
        }
    }

    fn default_config(&self, description: &str) -> PoolConfig {
        PoolConfig {
            start_time: 0,
            description: String::from_str(&self.env, description),
            metadata_url: String::from_str(&self.env, "ipfs://issues-boundary"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &self.env,
                String::from_str(&self.env, "No"),
                String::from_str(&self.env, "Yes"),
            ],
        }
    }

    /// Create a minimal 2-outcome pool ending at `end_time` (relative to now).
    fn create_pool(&self, end_time_offset: u64) -> u64 {
        let now = self.env.ledger().timestamp();
        let end_time = now + end_time_offset;
        self.client.create_pool(
            &self.creator,
            &end_time,
            &self.token_address,
            &2u32,
            &symbol_short!("Tech"),
            &self.default_config("Issues boundary pool"),
        )
    }

    fn advance_time(&self, seconds: u64) {
        let current = self.env.ledger().timestamp();
        self.env
            .ledger()
            .with_mut(|li| li.timestamp = current + seconds);
    }

    /// Mint tokens to an address and place a prediction.
    fn stake(&self, user: &Address, pool_id: u64, amount: i128, outcome: u32) {
        self.token_admin.mint(user, &amount);
        self.client
            .place_prediction(user, &pool_id, &amount, &outcome, &None, &None);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1438 — `create_pool` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Zero-duration pools (`end_time == start_time == now`) must be rejected.
#[test]
fn test_1438_zero_duration_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let now = ctx.env.ledger().timestamp();

    let mut config = ctx.default_config("Zero duration");
    config.start_time = now;

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &now,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &config,
    );
    assert_eq!(result, Err(Ok(PredifiError::InvalidTimestamp)));
}

/// `end_time = u64::MAX` vastly exceeds `MAX_POOL_DURATION` from now and must
/// be rejected with `InvalidTimestamp`.
#[test]
fn test_1438_max_u64_end_time_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &u64::MAX,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &ctx.default_config("Max u64 end_time"),
    );
    assert_eq!(result, Err(Ok(PredifiError::InvalidTimestamp)));
}

/// An empty description must be rejected (contract enforces non-empty
/// descriptions via an internal assertion).
#[test]
fn test_1438_empty_description_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let now = ctx.env.ledger().timestamp();

    let mut config = ctx.default_config("");
    config.description = String::from_str(&env, "");

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &(now + 100_000),
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &config,
    );
    assert!(result.is_err(), "empty description must be rejected");
}

/// Descriptions longer than 256 bytes must be rejected.
#[test]
fn test_1438_description_over_256_bytes_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let now = ctx.env.ledger().timestamp();

    let over_limit = core::str::from_utf8(&[b'x'; 257]).unwrap();
    let mut config = ctx.default_config("placeholder");
    config.description = String::from_str(&env, over_limit);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &(now + 100_000),
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &config,
    );
    assert!(
        result.is_err(),
        "description exceeding 256 bytes must be rejected"
    );
}

/// Creating two pools with an identical description is allowed — pools are
/// identified by their numeric `pool_id`, not by description, so there is no
/// duplicate-name restriction.
#[test]
fn test_1438_duplicate_descriptions_are_allowed() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let now = ctx.env.ledger().timestamp();
    let config = ctx.default_config("Same title every time");

    let pool_id_1 = ctx.client.create_pool(
        &ctx.creator,
        &(now + 100_000),
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &config,
    );
    let pool_id_2 = ctx.client.create_pool(
        &ctx.creator,
        &(now + 100_000),
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &config,
    );

    assert_ne!(pool_id_1, pool_id_2, "duplicate descriptions get distinct pool ids");
    assert_eq!(
        ctx.client.get_pool(&pool_id_1).description,
        ctx.client.get_pool(&pool_id_2).description
    );
}

/// A non-whitelisted token address must be rejected with `TokenNotWhitelisted`.
#[test]
fn test_1438_invalid_token_address_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let now = ctx.env.ledger().timestamp();
    let unlisted_token = Address::generate(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &(now + 100_000),
        &unlisted_token,
        &2u32,
        &symbol_short!("Tech"),
        &ctx.default_config("Invalid token pool"),
    );
    assert_eq!(result, Err(Ok(PredifiError::TokenNotWhitelisted)));
}

/// `create_pool` never captures a protocol fee at creation time — `pool.fee_bps`
/// starts at 0 regardless of whether the global `Config.fee_bps` is 0 or the
/// maximum (10_000). The dynamic fee is only computed later at resolution.
/// This guards against a fee-calculation regression at the create_pool boundary.
#[test]
fn test_1438_fee_bps_boundaries_not_captured_at_creation() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Global fee at 0 bps (the default from `init`).
    let pool_zero_fee = ctx.create_pool(100_000);
    assert_eq!(ctx.client.get_pool(&pool_zero_fee).fee_bps, 0);

    // Raise the global fee to the maximum (10_000 bps / 100%).
    ctx.client.set_fee_bps(&ctx.admin, &10_000u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);
    assert_eq!(ctx.client.get_fees().treasury_fee_bps, 10_000);

    let pool_max_fee = ctx.create_pool(100_000);
    assert_eq!(
        ctx.client.get_pool(&pool_max_fee).fee_bps,
        0,
        "pool.fee_bps must remain 0 at creation even when the global fee is 10_000 bps"
    );
}

/// A failed `create_pool` call (invalid `options_count`) must not advance the
/// pool id counter or leave any partial pool state behind — the next
/// successful call must still receive the very first pool id.
#[test]
fn test_1438_error_paths_leave_no_partial_state() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let now = ctx.env.ledger().timestamp();

    // options_count = 1 is invalid (must be >= 2).
    let bad_result = ctx.client.try_create_pool(
        &ctx.creator,
        &(now + 100_000),
        &ctx.token_address,
        &1u32,
        &symbol_short!("Tech"),
        &ctx.default_config("Invalid options count"),
    );
    assert_eq!(bad_result, Err(Ok(PredifiError::InvalidData)));

    // The counter must not have moved — the first successful pool still gets id 0.
    let pool_id = ctx.create_pool(100_000);
    assert_eq!(
        pool_id, 0,
        "a failed create_pool call must not consume a pool id"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1441 — `claim_refund` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Baseline: a user who staked into a pool that is later cancelled can claim
/// a full refund of their principal.
#[test]
fn test_1441_claim_refund_from_cancelled_pool_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let user = Address::generate(&env);

    ctx.stake(&user, pool_id, 500, 0);
    ctx.client
        .cancel_pool(&ctx.operator, &pool_id, &String::from_str(&env, "cancel"));

    let balance_before = ctx.token.balance(&user);
    let refunded = ctx.client.claim_refund(&user, &pool_id);
    let balance_after = ctx.token.balance(&user);

    assert_eq!(refunded, 500);
    assert_eq!(balance_after - balance_before, 500);
}

/// A user who placed multiple predictions on the same outcome in the same
/// pool has them accumulated into a single `Prediction` record; the refund
/// must cover the full accumulated amount, not just the most recent stake.
#[test]
fn test_1441_claim_refund_with_multiple_predictions_same_pool() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let user = Address::generate(&env);

    // Two separate stakes on the same outcome accumulate into one record.
    ctx.stake(&user, pool_id, 200, 0);
    ctx.stake(&user, pool_id, 300, 0);

    ctx.client
        .cancel_pool(&ctx.operator, &pool_id, &String::from_str(&env, "cancel"));

    let refunded = ctx.client.claim_refund(&user, &pool_id);
    assert_eq!(
        refunded, 500,
        "refund must equal the sum of all accumulated predictions (200 + 300)"
    );
}

/// A user with no prediction on the pool (i.e. the effective refundable
/// balance is zero) must be rejected with `InsufficientBalance` rather than
/// silently transferring 0.
#[test]
fn test_1441_claim_refund_with_no_prediction_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let uninvolved_user = Address::generate(&env);

    ctx.client
        .cancel_pool(&ctx.operator, &pool_id, &String::from_str(&env, "cancel"));

    let result = ctx.client.try_claim_refund(&uninvolved_user, &pool_id);
    assert_eq!(result, Err(Ok(PredifiError::InsufficientBalance)));
}

/// Double-refund prevention: a second claim for the same (user, pool) must
/// fail with `AlreadyClaimed` and must not move any additional funds.
#[test]
fn test_1441_double_refund_is_prevented() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let user = Address::generate(&env);

    ctx.stake(&user, pool_id, 400, 1);
    ctx.client
        .cancel_pool(&ctx.operator, &pool_id, &String::from_str(&env, "cancel"));

    ctx.client.claim_refund(&user, &pool_id);
    let balance_after_first = ctx.token.balance(&user);

    let second_attempt = ctx.client.try_claim_refund(&user, &pool_id);
    assert_eq!(second_attempt, Err(Ok(PredifiError::AlreadyClaimed)));
    assert_eq!(
        ctx.token.balance(&user),
        balance_after_first,
        "balance must not change on a rejected duplicate refund"
    );
}

/// Claiming a refund on a pool that is still `Active` (never cancelled) must
/// fail with `InvalidPoolState`.
#[test]
fn test_1441_claim_refund_on_active_pool_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let user = Address::generate(&env);

    ctx.stake(&user, pool_id, 100, 0);

    let result = ctx.client.try_claim_refund(&user, &pool_id);
    assert_eq!(result, Err(Ok(PredifiError::InvalidPoolState)));
}

/// Claiming a refund on a `Resolved` pool (not cancelled) must fail with
/// `InvalidPoolState` — refunds only apply to cancelled pools.
#[test]
fn test_1441_claim_refund_on_resolved_pool_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(4_000);
    let user = Address::generate(&env);

    ctx.stake(&user, pool_id, 100, 0);
    ctx.advance_time(4_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Resolved);

    let result = ctx.client.try_claim_refund(&user, &pool_id);
    assert_eq!(result, Err(Ok(PredifiError::InvalidPoolState)));
}

/// A refund claimed in the very same instant the pool transitions to
/// `Canceled` must succeed — there is no minimum delay after cancellation.
#[test]
fn test_1441_claim_refund_immediately_after_cancellation_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let user = Address::generate(&env);

    ctx.stake(&user, pool_id, 250, 0);
    ctx.client
        .cancel_pool(&ctx.operator, &pool_id, &String::from_str(&env, "cancel"));

    // No time advance — claim in the same ledger timestamp as cancellation.
    let refunded = ctx.client.claim_refund(&user, &pool_id);
    assert_eq!(refunded, 250);
}

/// Unlike `claim_winnings` (which has a claim window), `claim_refund` has no
/// expiry — a refund must still succeed long after the pool was cancelled.
#[test]
fn test_1441_claim_refund_long_after_cancellation_still_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let user = Address::generate(&env);

    ctx.stake(&user, pool_id, 150, 0);
    ctx.client
        .cancel_pool(&ctx.operator, &pool_id, &String::from_str(&env, "cancel"));

    // Advance far beyond any claim-window-like duration (e.g. 2 years).
    ctx.advance_time(63_072_000);

    let refunded = ctx.client.claim_refund(&user, &pool_id);
    assert_eq!(
        refunded, 150,
        "claim_refund must not expire, unlike claim_winnings"
    );
}

/// Claiming a refund for a pool_id that was never created must fail with
/// `InvalidPoolState` (the pool lookup returns None).
#[test]
fn test_1441_claim_refund_nonexistent_pool_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let user = Address::generate(&env);

    let result = ctx.client.try_claim_refund(&user, &999u64);
    assert_eq!(result, Err(Ok(PredifiError::InvalidPoolState)));
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1444 — `emergency_cancel_pool` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

/// N-1 approvals (one short of the `EMERGENCY_CANCEL_MULTISIG_THRESHOLD` of 2)
/// must leave the pool `Active` — no partial/rolled-back cancellation state.
#[test]
fn test_1444_n_minus_1_approvals_leave_pool_active_state_rollback() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let reason = String::from_str(&env, "single approval");

    ctx.client
        .emergency_cancel_pool(&ctx.operator, &pool_id, &reason);

    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Active);
    assert_eq!(ctx.client.get_emergency_cancel_approvals(&pool_id).len(), 1);
}

/// Exactly N approvals (the quorum boundary) atomically cancels the pool and
/// clears the pending-approvers state.
#[test]
fn test_1444_exact_quorum_cancels_pool() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let reason = String::from_str(&env, "quorum reached");

    ctx.client
        .emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    ctx.client
        .emergency_cancel_pool(&ctx.operator2, &pool_id, &reason);

    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);
    assert_eq!(ctx.client.get_emergency_cancel_approvals(&pool_id).len(), 0);
}

/// A duplicate approval from the same approver must fail with
/// `EmergencyCancelAlreadyApproved` and must not advance the approver count
/// (state rollback verification for a rejected call).
#[test]
fn test_1444_duplicate_approval_same_approver_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let reason = String::from_str(&env, "dup test");

    ctx.client
        .emergency_cancel_pool(&ctx.operator, &pool_id, &reason);

    let result = ctx
        .client
        .try_emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(
        result,
        Err(Ok(PredifiError::EmergencyCancelAlreadyApproved))
    );
    assert_eq!(
        ctx.client.get_emergency_cancel_approvals(&pool_id).len(),
        1,
        "a rejected duplicate approval must not change the approver count"
    );
}

/// Approving an emergency cancel on a pool that is already `Canceled` must
/// fail with `InvalidPoolState`.
#[test]
fn test_1444_approval_after_already_cancelled_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    ctx.client
        .cancel_pool(&ctx.operator, &pool_id, &String::from_str(&env, "normal cancel"));
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);

    let result = ctx.client.try_emergency_cancel_pool(
        &ctx.operator,
        &pool_id,
        &String::from_str(&env, "too late"),
    );
    assert_eq!(result, Err(Ok(PredifiError::InvalidPoolState)));
}

/// Approving an emergency cancel on a pool that is already `Resolved` must
/// fail with `InvalidPoolState`.
#[test]
fn test_1444_approval_after_already_resolved_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(4_000);
    let user = Address::generate(&env);

    ctx.stake(&user, pool_id, 100, 0);
    ctx.advance_time(4_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Resolved);

    let result = ctx.client.try_emergency_cancel_pool(
        &ctx.operator,
        &pool_id,
        &String::from_str(&env, "too late"),
    );
    assert_eq!(result, Err(Ok(PredifiError::InvalidPoolState)));
}

/// An address without admin (0) or operator (1) role must be rejected with
/// `Unauthorized`, and the pool must remain unchanged (state rollback check).
#[test]
fn test_1444_unauthorized_approver_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);
    let stranger = Address::generate(&env);

    let result = ctx.client.try_emergency_cancel_pool(
        &stranger,
        &pool_id,
        &String::from_str(&env, "no role"),
    );
    assert_eq!(result, Err(Ok(PredifiError::Unauthorized)));
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Active);
    assert_eq!(ctx.client.get_emergency_cancel_approvals(&pool_id).len(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1445 — `set_fee_bps` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

/// The minimum valid boundary, `fee_bps = 0`, must be accepted and committed
/// after the timelock elapses.
#[test]
fn test_1445_fee_bps_zero_boundary_accepted() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    ctx.client.set_fee_bps(&ctx.admin, &0u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    assert_eq!(ctx.client.get_fees().treasury_fee_bps, 0);
}

/// The maximum valid boundary, `fee_bps = 10_000` (100%), must be accepted.
#[test]
fn test_1445_fee_bps_10000_boundary_accepted() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    ctx.client.set_fee_bps(&ctx.admin, &10_000u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    assert_eq!(ctx.client.get_fees().treasury_fee_bps, 10_000);
}

/// `fee_bps = 10_001` is one past the cap and must be rejected with
/// `InvalidFeeBps`.
#[test]
fn test_1445_fee_bps_10001_overflow_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_set_fee_bps(&ctx.admin, &10_001u32);
    assert_eq!(result, Err(Ok(PredifiError::InvalidFeeBps)));
}

/// `fee_bps = u32::MAX` must be rejected with `InvalidFeeBps`.
#[test]
fn test_1445_fee_bps_u32_max_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_set_fee_bps(&ctx.admin, &u32::MAX);
    assert_eq!(result, Err(Ok(PredifiError::InvalidFeeBps)));
}

/// Rapid successive fee changes (propose → apply → propose → apply) must
/// leave the contract in a consistent final state with no leftover pending
/// proposal.
#[test]
fn test_1445_rapid_successive_fee_changes_state_consistency() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    ctx.client.set_fee_bps(&ctx.admin, &100u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    ctx.client.set_fee_bps(&ctx.admin, &9_999u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    ctx.client.set_fee_bps(&ctx.admin, &42u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    assert_eq!(ctx.client.get_fees().treasury_fee_bps, 42);
    assert!(ctx.client.get_pending_fee_change().is_none());
}

/// Changing (and applying) the global fee while a pool is already active must
/// not retroactively affect that pool — its resolution-time fee is computed
/// dynamically at resolve time, independent of the timing of admin fee edits.
#[test]
fn test_1445_fee_change_while_pool_active_does_not_disrupt_pool() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 100, 0);

    ctx.client.set_fee_bps(&ctx.admin, &3_000u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    // The active pool must be unaffected — still Active, stake intact.
    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Active);
    assert_eq!(pool.total_stake, 100);
    assert_eq!(ctx.client.get_fees().treasury_fee_bps, 3_000);
}

/// Verify the full pending-fee-change state machine: no pending change before
/// `set_fee_bps`; a pending change appears immediately after; and it is
/// cleared again once `apply_fee_bps` commits it.
#[test]
fn test_1445_pending_fee_change_state_transitions() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    assert!(ctx.client.get_pending_fee_change().is_none());

    ctx.client.set_fee_bps(&ctx.admin, &750u32);
    let pending = ctx
        .client
        .get_pending_fee_change()
        .expect("a pending proposal must exist right after set_fee_bps");
    assert_eq!(pending.new_fee_bps, 750);
    assert_eq!(
        pending.effective_at,
        ctx.env.ledger().timestamp() + FEE_CHANGE_TIMELOCK_SECONDS
    );

    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    assert!(
        ctx.client.get_pending_fee_change().is_none(),
        "pending proposal must be cleared once applied"
    );
    assert_eq!(ctx.client.get_fees().treasury_fee_bps, 750);
}

/// A second `set_fee_bps` call while a proposal is already pending must be
/// rejected with `FeeChangePending`, and the original proposal must be
/// untouched.
#[test]
fn test_1445_second_proposal_while_pending_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    ctx.client.set_fee_bps(&ctx.admin, &250u32);
    let result = ctx.client.try_set_fee_bps(&ctx.admin, &999u32);

    assert_eq!(result, Err(Ok(PredifiError::FeeChangePending)));
    assert_eq!(
        ctx.client
            .get_pending_fee_change()
            .expect("original proposal preserved")
            .new_fee_bps,
        250
    );
}
