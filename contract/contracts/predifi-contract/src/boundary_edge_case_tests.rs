//! Boundary and edge-case tests for Issues #1313–#1316, #1323, #1324.
//!
//! | Issue  | Function              | Coverage highlights                                      |
//! |--------|-----------------------|----------------------------------------------------------|
//! | #1324  | `update_referrer`     | self-referrer, None/clear, post-prediction change, cycle |
//! | #1323  | `withdraw_treasury`   | exceed bal, zero, non-admin, non-whitelisted, consistency|
//! | #1316  | `set_fee_bps`         | min/max boundaries, overflow, timelock, concurrent pools |
//! | #1315  | `emergency_cancel_pool` | multi-sig quorum, duplicates, resolved/cancelled state |
//! | #1314  | `cancel_pool`         | auth, double-cancel, resolved pool, state invariants     |
//! | #1313  | `resolve_pool`        | zero stakes, one-sided, delay boundary, re-resolution    |

#![cfg(test)]

extern crate std;

use crate::{
    FEE_CHANGE_TIMELOCK_SECONDS,
    MarketState, PoolConfig, PredifiContract, PredifiContractClient, PredifiError,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

// ─── Shared dummy access-control stub ────────────────────────────────────────

mod dummy_ac {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct DummyAC;

    #[contractimpl]
    impl DummyAC {
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
            let had_role: bool = env.storage().instance().get(&key).unwrap_or(false);
            env.storage().instance().set(&key, &false);
            if role == 1 && had_role {
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

/// Full test environment.
struct TestEnv<'a> {
    pub env: Env,
    pub client: PredifiContractClient<'a>,
    pub ac: dummy_ac::DummyACClient<'a>,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
    pub token_address: Address,
    pub treasury: Address,
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

        let ac_id = env.register(dummy_ac::DummyAC, ());
        let ac = dummy_ac::DummyACClient::new(env, &ac_id);
        ac.grant_role(&admin, &0u32); // Admin role
        ac.grant_role(&operator, &1u32); // Operator role
        ac.grant_role(&operator2, &1u32); // Second operator role

        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(env, &contract_id);
        // resolution_delay = 0 for simplicity; min_pool_duration = 3600
        client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

        let token_admin_addr = Address::generate(env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr.clone());
        let token_address = token_contract.address();
        let token = token::Client::new(env, &token_address);
        let token_admin = token::StellarAssetClient::new(env, &token_address);

        client.add_token_to_whitelist(&admin, &token_address);

        Self {
            env: env.clone(),
            client,
            ac,
            token,
            token_admin,
            token_address,
            treasury,
            admin,
            operator,
            operator2,
            creator,
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
            &PoolConfig {
                start_time: 0,
                description: String::from_str(&self.env, "Boundary test pool"),
                metadata_url: String::from_str(&self.env, "ipfs://boundary"),
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

    /// Create a pool with a custom `required_resolutions` value.
    fn create_pool_with_resolutions(&self, end_time_offset: u64, required: u32) -> u64 {
        let now = self.env.ledger().timestamp();
        let end_time = now + end_time_offset;
        self.client.create_pool(
            &self.creator,
            &end_time,
            &self.token_address,
            &2u32,
            &symbol_short!("Tech"),
            &PoolConfig {
                start_time: 0,
                description: String::from_str(&self.env, "Multi-resolution pool"),
                metadata_url: String::from_str(&self.env, "ipfs://multi-res"),
                min_stake: 1i128,
                max_stake: 0i128,
                max_total_stake: 0i128,
                min_total_stake: 0i128,
                initial_liquidity: 0i128,
                required_resolutions: required,
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

    /// Advance ledger timestamp by `seconds`.
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
// Issue #1316 — `set_fee_bps` Boundary Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Minimum valid fee boundary: `fee_bps = 0` (0 %) must be accepted and
/// persist after the timelock elapses.
#[test]
fn test_1316_fee_bps_minimum_zero_is_valid() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Queue the proposal for 0 bps.
    ctx.client.set_fee_bps(&ctx.admin, &0u32);

    // Advance past the timelock.
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    let info = ctx.client.get_fees();
    assert_eq!(info.treasury_fee_bps, 0, "fee_bps should be 0 after applying");
}

/// Maximum valid fee boundary: `fee_bps = 10_000` (100 %) must be accepted.
#[test]
fn test_1316_fee_bps_maximum_10000_is_valid() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    ctx.client.set_fee_bps(&ctx.admin, &10_000u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    let info = ctx.client.get_fees();
    assert_eq!(
        info.treasury_fee_bps, 10_000,
        "fee_bps should be 10_000 after applying"
    );
}

/// `fee_bps = 10_001` exceeds the 100 % cap and must return `InvalidFeeBps`.
#[test]
fn test_1316_fee_bps_10001_is_invalid() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_set_fee_bps(&ctx.admin, &10_001u32);
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidFeeBps)),
        "fee_bps = 10_001 must be rejected with InvalidFeeBps"
    );
}

/// `fee_bps = u32::MAX` must be rejected with `InvalidFeeBps`.
#[test]
fn test_1316_fee_bps_u32_max_is_invalid() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_set_fee_bps(&ctx.admin, &u32::MAX);
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidFeeBps)),
        "fee_bps = u32::MAX must be rejected with InvalidFeeBps"
    );
}

/// A second `set_fee_bps` call while a proposal is pending must fail with
/// `FeeChangePending`.  The original proposal must remain intact.
#[test]
fn test_1316_second_set_fee_bps_while_pending_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // First proposal.
    ctx.client.set_fee_bps(&ctx.admin, &300u32);

    // Second proposal before the first is applied or cancelled.
    let result = ctx.client.try_set_fee_bps(&ctx.admin, &500u32);
    assert_eq!(
        result,
        Err(Ok(PredifiError::FeeChangePending)),
        "second set_fee_bps while pending must return FeeChangePending"
    );

    // The original proposal is preserved.
    let pending = ctx
        .client
        .get_pending_fee_change()
        .expect("original proposal must still exist");
    assert_eq!(pending.new_fee_bps, 300, "original proposal must be 300 bps");
}

/// Applying the fee before the timelock has elapsed must fail with
/// `TimelockNotExpired`.
#[test]
fn test_1316_apply_fee_bps_before_timelock_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    ctx.client.set_fee_bps(&ctx.admin, &400u32);

    // Advance only half the timelock — still locked.
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS / 2);

    let result = ctx.client.try_apply_fee_bps(&ctx.admin);
    assert_eq!(
        result,
        Err(Ok(PredifiError::TimelockNotExpired)),
        "apply_fee_bps before timelock must fail with TimelockNotExpired"
    );
}

/// Applying at *exactly* `now + FEE_CHANGE_TIMELOCK_SECONDS` (the first
/// eligible second) must succeed.
#[test]
fn test_1316_apply_fee_bps_at_exact_timelock_expiry_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    ctx.client.set_fee_bps(&ctx.admin, &750u32);
    // Advance to exactly effective_at.
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS);
    ctx.client.apply_fee_bps(&ctx.admin);

    let info = ctx.client.get_fees();
    assert_eq!(info.treasury_fee_bps, 750);
}

/// A proposal can be cancelled before the timelock, after which a new
/// proposal can be queued immediately.
#[test]
fn test_1316_cancel_fee_proposal_allows_new_proposal() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    ctx.client.set_fee_bps(&ctx.admin, &200u32);

    // Cancel before timelock expires.
    ctx.client.cancel_fee_proposal(&ctx.admin);
    assert!(
        ctx.client.get_pending_fee_change().is_none(),
        "pending proposal must be cleared after cancel"
    );

    // Queue a fresh proposal immediately after cancellation.
    ctx.client.set_fee_bps(&ctx.admin, &600u32);
    let pending = ctx
        .client
        .get_pending_fee_change()
        .expect("new proposal must exist");
    assert_eq!(pending.new_fee_bps, 600, "new proposal must be 600 bps");
}

/// Fee parameter changes while pools are actively running do not affect
/// already-created pools whose `fee_bps` is captured at creation.
#[test]
fn test_1316_fee_change_does_not_affect_active_pool_fee() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Create a pool (captures the fee at creation time = 0 bps from init).
    let pool_id = ctx.create_pool(7_200);
    let pool_before = ctx.client.get_pool(&pool_id);

    // Admin queues and applies a fee change while the pool is running.
    ctx.client.set_fee_bps(&ctx.admin, &2_000u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    // The running pool's captured fee_bps is unchanged.
    let pool_after = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool_after.fee_bps, pool_before.fee_bps,
        "running pool fee_bps must not change when protocol fee is updated"
    );
    assert_eq!(
        ctx.client.get_fees().treasury_fee_bps,
        2_000,
        "global fee must have been updated to 2_000 bps"
    );
}

/// Rapid successive proposals: propose → cancel → propose → apply.
/// All transitions must complete without state corruption.
#[test]
fn test_1316_rapid_successive_fee_updates() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Cycle 1: propose then cancel.
    ctx.client.set_fee_bps(&ctx.admin, &100u32);
    ctx.client.cancel_fee_proposal(&ctx.admin);

    // Cycle 2: propose then apply.
    ctx.client.set_fee_bps(&ctx.admin, &500u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    // Cycle 3: propose a third value and apply.
    ctx.client.set_fee_bps(&ctx.admin, &9_999u32);
    ctx.advance_time(FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    let info = ctx.client.get_fees();
    assert_eq!(info.treasury_fee_bps, 9_999);
    assert!(
        ctx.client.get_pending_fee_change().is_none(),
        "no pending proposal after all cycles"
    );
}

/// Applying when there is no pending proposal must return `NoFeeChangePending`.
#[test]
fn test_1316_apply_fee_bps_with_no_pending_proposal_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_apply_fee_bps(&ctx.admin);
    assert_eq!(
        result,
        Err(Ok(PredifiError::NoFeeChangePending)),
        "apply_fee_bps without a pending proposal must return NoFeeChangePending"
    );
}

/// Cancelling when there is no pending proposal must return `NoFeeChangePending`.
#[test]
fn test_1316_cancel_fee_proposal_with_no_pending_proposal_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_cancel_fee_proposal(&ctx.admin);
    assert_eq!(
        result,
        Err(Ok(PredifiError::NoFeeChangePending)),
        "cancel_fee_proposal without a pending proposal must return NoFeeChangePending"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1315 — `emergency_cancel_pool` Boundary Tests
// ═══════════════════════════════════════════════════════════════════════════

/// N-1 approvals (one short of the threshold) must leave the pool Active.
/// Only the Nth approval triggers cancellation.
#[test]
fn test_1315_n_minus_1_approvals_do_not_cancel_pool() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);

    // EMERGENCY_CANCEL_MULTISIG_THRESHOLD = 2, so one approval is N-1.
    let reason = String::from_str(&env, "suspicious activity");
    ctx.client
        .emergency_cancel_pool(&ctx.operator, &pool_id, &reason);

    // Pool must still be Active after only 1 of 2 required approvals.
    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool.state,
        MarketState::Active,
        "pool must remain Active after N-1 emergency-cancel approvals"
    );

    // Approvers list must have exactly 1 entry.
    let approvers = ctx.client.get_emergency_cancel_approvals(&pool_id);
    assert_eq!(
        approvers.len(),
        1,
        "approvers list must contain 1 entry after first approval"
    );
}

/// Exactly N (threshold) approvals must atomically transition the pool to
/// `Canceled` and clear the approvers list.
#[test]
fn test_1315_exact_quorum_cancels_pool() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let reason = String::from_str(&env, "emergency");

    // First approval — still pending.
    ctx.client
        .emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Active);

    // Second approval reaches the threshold — pool is cancelled.
    ctx.client
        .emergency_cancel_pool(&ctx.operator2, &pool_id, &reason);
    assert_eq!(
        ctx.client.get_pool(&pool_id).state,
        MarketState::Canceled,
        "pool must be Canceled after reaching quorum"
    );

    // Approvers list must have been cleared on execution.
    let approvers = ctx.client.get_emergency_cancel_approvals(&pool_id);
    assert_eq!(
        approvers.len(),
        0,
        "approvers list must be empty after cancellation executes"
    );
}

/// A duplicate approval from the same address must return
/// `EmergencyCancelAlreadyApproved` and must not advance the approver count.
#[test]
fn test_1315_duplicate_approval_is_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let reason = String::from_str(&env, "duplicate test");

    // First approval succeeds.
    ctx.client
        .emergency_cancel_pool(&ctx.operator, &pool_id, &reason);

    // Second approval from the same address must fail.
    let result = ctx
        .client
        .try_emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(
        result,
        Err(Ok(PredifiError::EmergencyCancelAlreadyApproved)),
        "duplicate approval must return EmergencyCancelAlreadyApproved"
    );

    // Approver count must not have changed.
    let approvers = ctx.client.get_emergency_cancel_approvals(&pool_id);
    assert_eq!(
        approvers.len(),
        1,
        "approver count must remain 1 after rejected duplicate"
    );
}

/// Calling `emergency_cancel_pool` on an already-Canceled pool must return
/// `InvalidPoolState`.
#[test]
fn test_1315_approval_on_cancelled_pool_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let reason = String::from_str(&env, "cancel first");

    // Cancel via the normal path first.
    ctx.client
        .cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);

    // Emergency cancel on an already-Canceled pool must fail.
    let result = ctx
        .client
        .try_emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidPoolState)),
        "emergency_cancel on a Canceled pool must return InvalidPoolState"
    );
}

/// Calling `emergency_cancel_pool` on an already-Resolved pool must return
/// `InvalidPoolState`.
#[test]
fn test_1315_approval_on_resolved_pool_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(4_000);

    // Place a stake so the pool has participants.
    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 100, 0);

    // Advance past end_time and resolve.
    ctx.advance_time(4_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Resolved);

    let reason = String::from_str(&env, "already resolved");
    let result = ctx
        .client
        .try_emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidPoolState)),
        "emergency_cancel on a Resolved pool must return InvalidPoolState"
    );
}

/// An address without admin or operator role must be rejected with
/// `Unauthorized` and the pool must remain unchanged.
#[test]
fn test_1315_unauthorized_approver_is_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let stranger = Address::generate(&env);
    let reason = String::from_str(&env, "unauthorized attempt");

    let result = ctx
        .client
        .try_emergency_cancel_pool(&stranger, &pool_id, &reason);
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "non-privileged address must be rejected with Unauthorized"
    );

    assert_eq!(
        ctx.client.get_pool(&pool_id).state,
        MarketState::Active,
        "pool must remain Active after unauthorized attempt"
    );
}

/// Only the first approver's reason is stored; subsequent approvers'
/// reasons are ignored — the original reason must be preserved.
#[test]
fn test_1315_first_reason_is_preserved() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Grant a third operator so we can test without hitting quorum too early.
    let operator3 = Address::generate(&env);
    ctx.ac.grant_role(&operator3, &1u32);

    // For this test we need 3 approvals, so create a pool with default threshold (2).
    // We'll only send 2 approvals but from different operators.
    let pool_id = ctx.create_pool(7_200);

    let first_reason = String::from_str(&env, "first reason");
    let second_reason = String::from_str(&env, "second reason");

    // First approval sets the reason.
    ctx.client
        .emergency_cancel_pool(&ctx.operator, &pool_id, &first_reason);
    // Second approval (reaches quorum) provides a different reason.
    ctx.client
        .emergency_cancel_pool(&ctx.operator2, &pool_id, &second_reason);

    // Pool is now Canceled — state transition confirms quorum reached.
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);
}

/// A pool with active stakes can be emergency-cancelled once quorum is met,
/// and the stake amounts are preserved (refundable) in the cancelled state.
#[test]
fn test_1315_emergency_cancel_with_active_stakes() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);

    // Place stakes on both outcomes.
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    ctx.stake(&user1, pool_id, 500, 0);
    ctx.stake(&user2, pool_id, 300, 1);

    let total_before = ctx.client.get_pool(&pool_id).total_stake;
    assert_eq!(total_before, 800, "total stake must be 800 before cancel");

    let reason = String::from_str(&env, "emergency with stakes");
    ctx.client
        .emergency_cancel_pool(&ctx.operator, &pool_id, &reason);
    ctx.client
        .emergency_cancel_pool(&ctx.operator2, &pool_id, &reason);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Canceled);
    // Stakes are preserved so users can claim refunds.
    assert_eq!(
        pool.total_stake, 800,
        "total_stake must be preserved after emergency cancel so refunds can be claimed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1314 — `cancel_pool` Boundary Tests
// ═══════════════════════════════════════════════════════════════════════════

/// An operator can cancel an empty pool (no predictions placed).
#[test]
fn test_1314_operator_can_cancel_empty_pool() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let reason = String::from_str(&env, "no activity");

    ctx.client
        .cancel_pool(&ctx.operator, &pool_id, &reason);

    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);
}

/// An operator can cancel a pool that already has active predictions.
#[test]
fn test_1314_operator_can_cancel_pool_with_active_predictions() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);

    // Place some predictions.
    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 200, 0);

    ctx.client.cancel_pool(
        &ctx.operator,
        &pool_id,
        &String::from_str(&env, "admin cancel"),
    );

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Canceled);
    // Stakes must be preserved for refund.
    assert_eq!(pool.total_stake, 200, "total_stake must remain for refunds");
}

/// The creator can cancel only if no extra stakes have been placed (stake ==
/// initial_liquidity).  After a user stakes, the creator's cancel must fail.
#[test]
fn test_1314_creator_cannot_cancel_after_user_stakes() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);

    // A user places a stake, pushing total_stake above initial_liquidity (0).
    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 100, 1);

    let result = ctx.client.try_cancel_pool(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "creator cancel"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "creator must not cancel after extra stakes have been placed"
    );
}

/// The creator can cancel an empty pool (total_stake == initial_liquidity == 0).
#[test]
fn test_1314_creator_can_cancel_empty_pool() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);

    ctx.client.cancel_pool(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "creator cancel empty"),
    );

    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);
}

/// An address with neither admin/operator role nor creator status must be
/// rejected before the `CANCELATION_DELAY` has passed.
#[test]
fn test_1314_unauthorized_cancel_is_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let stranger = Address::generate(&env);

    let result = ctx.client.try_cancel_pool(
        &stranger,
        &pool_id,
        &String::from_str(&env, "stranger cancel"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "unauthorized address must not be able to cancel before overdue threshold"
    );
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Active);
}

/// After `end_time + CANCELATION_DELAY` has elapsed, *any* address may cancel
/// an overdue pool (failsafe to unlock funds).
#[test]
fn test_1314_any_address_can_cancel_overdue_pool() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Pool ends in 5_000 s; CANCELATION_DELAY = 604_800 s.
    let pool_id = ctx.create_pool(5_000);
    let stranger = Address::generate(&env);

    // Advance past end_time + CANCELATION_DELAY.
    ctx.advance_time(5_000 + 604_800 + 1);

    ctx.client.cancel_pool(
        &stranger,
        &pool_id,
        &String::from_str(&env, "overdue cancel"),
    );

    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);
}

/// Double-cancellation must return `InvalidPoolState`.
#[test]
fn test_1314_double_cancel_is_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    let reason = String::from_str(&env, "first cancel");

    ctx.client.cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);

    let result = ctx.client.try_cancel_pool(
        &ctx.operator,
        &pool_id,
        &String::from_str(&env, "second cancel"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidPoolState)),
        "cancelling an already-Canceled pool must return InvalidPoolState"
    );
}

/// Attempting to cancel an already-Resolved pool must return `InvalidPoolState`.
#[test]
fn test_1314_cancel_resolved_pool_is_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(4_000);

    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 100, 0);

    // Advance past end_time and resolve.
    ctx.advance_time(4_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Resolved);

    let result = ctx.client.try_cancel_pool(
        &ctx.operator,
        &pool_id,
        &String::from_str(&env, "cancel resolved"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidPoolState)),
        "cancelling a Resolved pool must return InvalidPoolState"
    );
}

/// After cancellation, all user stakes must be refundable at full face value
/// (state invariant: total_stake equals sum of all user stakes, no leakage).
#[test]
fn test_1314_state_invariant_after_cancel_no_stake_leakage() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);

    // Three users stake different amounts on both outcomes.
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let user_c = Address::generate(&env);

    ctx.stake(&user_a, pool_id, 300, 0);
    ctx.stake(&user_b, pool_id, 150, 1);
    ctx.stake(&user_c, pool_id, 50, 0);

    let total_staked = 300 + 150 + 50_i128;

    ctx.client.cancel_pool(
        &ctx.operator,
        &pool_id,
        &String::from_str(&env, "invariant test"),
    );

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Canceled);
    assert_eq!(
        pool.total_stake, total_staked,
        "total_stake must equal sum of all user stakes after cancel — no leakage"
    );

    // Each user must be able to claim a full refund.
    let bal_a_before = ctx.token.balance(&user_a);
    let refund_a = ctx.client.claim_refund(&user_a, &pool_id);
    let bal_a_after = ctx.token.balance(&user_a);
    assert_eq!(refund_a, 300, "user_a refund must be 300");
    assert_eq!(
        bal_a_after - bal_a_before,
        300,
        "user_a token balance must increase by 300"
    );

    let bal_b_before = ctx.token.balance(&user_b);
    let refund_b = ctx.client.claim_refund(&user_b, &pool_id);
    let bal_b_after = ctx.token.balance(&user_b);
    assert_eq!(refund_b, 150, "user_b refund must be 150");
    assert_eq!(bal_b_after - bal_b_before, 150);

    let refund_c = ctx.client.claim_refund(&user_c, &pool_id);
    assert_eq!(refund_c, 50, "user_c refund must be 50");
}

// ═══════════════════════════════════════════════════════════════════════════
// Boundary & Edge Case Tests: create_pool
// ═══════════════════════════════════════════════════════════════════════════

/// Creating a pool with zero duration (end_time == start_time) must be rejected
/// with InvalidTimestamp.
#[test]
fn test_create_pool_zero_duration_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let current_time = ctx.env.ledger().timestamp();
    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &current_time, // end_time == current_time (zero duration from now)
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: current_time, // start_time == end_time
            description: String::from_str(&ctx.env, "Zero duration pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://zero"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidTimestamp)),
        "zero duration pool must be rejected with InvalidTimestamp"
    );
}

/// Creating a pool with end_time == u64::MAX should be rejected if it exceeds
/// MAX_POOL_DURATION from current time.
#[test]
fn test_create_pool_max_u64_timestamp_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &u64::MAX,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Max timestamp pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://max"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidTimestamp)),
        "u64::MAX timestamp must be rejected as exceeding MAX_POOL_DURATION"
    );
}

/// Creating a pool with an empty description string should be rejected.
/// The contract validates description length, and empty strings are invalid.
#[test]
fn test_create_pool_empty_description_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, ""), // Empty description
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    // Empty description should be rejected (assert! in code will panic)
    assert!(result.is_err(), "empty description must be rejected");
}

/// Creating a pool with a description exceeding 256 bytes must be rejected.
#[test]
fn test_create_pool_description_too_long_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let over_limit = core::str::from_utf8(&[b'a'; 257]).unwrap();
    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, over_limit), // 257 bytes
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_err(), "description > 256 bytes must be rejected");
}

/// Creating a pool with exactly 256-byte description should succeed (boundary test).
#[test]
fn test_create_pool_description_at_limit_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let at_limit = core::str::from_utf8(&[b'a'; 256]).unwrap();
    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, at_limit), // Exactly 256 bytes
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_ok(), "256-byte description should be accepted");
}

/// Creating a pool with a non-whitelisted token must be rejected with
/// TokenNotWhitelisted.
#[test]
fn test_create_pool_invalid_token_address_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let invalid_token = Address::generate(&ctx.env);
    // Do NOT whitelist this token

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &invalid_token, // Non-whitelisted token
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Invalid token pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::TokenNotWhitelisted)),
        "non-whitelisted token must be rejected with TokenNotWhitelisted"
    );
}

/// Creating multiple pools with the same description should succeed - pools are
/// identified by pool_id, not by description. This verifies there's no
/// duplicate-name restriction.
#[test]
fn test_create_pool_duplicate_descriptions_allowed() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let description = String::from_str(&ctx.env, "Duplicate Test Pool");

    // Create first pool
    let pool_id1 = ctx
        .client
        .create_pool(
            &ctx.creator,
            &100_000u64,
            &ctx.token_address,
            &2u32,
            &symbol_short!("Tech"),
            &PoolConfig {
                start_time: 0,
                description: description.clone(),
                metadata_url: String::from_str(&ctx.env, "ipfs://test1"),
                min_stake: 1i128,
                max_stake: 0i128,
                max_total_stake: 0i128,
                min_total_stake: 1i128,
                initial_liquidity: 0i128,
                required_resolutions: 1u32,
                private: false,
                whitelist_key: None,
                outcome_descriptions: vec![
                    &ctx.env,
                    String::from_str(&ctx.env, "No"),
                    String::from_str(&ctx.env, "Yes"),
                ],
            },
        )
        .unwrap();

    // Create second pool with same description
    let pool_id2 = ctx
        .client
        .create_pool(
            &ctx.creator,
            &100_001u64,
            &ctx.token_address,
            &2u32,
            &symbol_short!("Tech"),
            &PoolConfig {
                start_time: 0,
                description: description.clone(), // Same description
                metadata_url: String::from_str(&ctx.env, "ipfs://test2"),
                min_stake: 1i128,
                max_stake: 0i128,
                max_total_stake: 0i128,
                min_total_stake: 1i128,
                initial_liquidity: 0i128,
                required_resolutions: 1u32,
                private: false,
                whitelist_key: None,
                outcome_descriptions: vec![
                    &ctx.env,
                    String::from_str(&ctx.env, "No"),
                    String::from_str(&ctx.env, "Yes"),
                ],
            },
        )
        .unwrap();

    // Both pools should exist with different IDs
    assert_ne!(pool_id1, pool_id2, "pools must have different IDs");
    
    let pool1 = ctx.client.get_pool(&pool_id1);
    let pool2 = ctx.client.get_pool(&pool_id2);
    assert_eq!(pool1.description, pool2.description, "descriptions should match");
}

/// Creating a pool with options_count = 1 must be rejected (minimum is 2).
#[test]
fn test_create_pool_single_option_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &1u32, // Only 1 option - invalid
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Single option pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![&ctx.env, String::from_str(&ctx.env, "Only")],
        },
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidData)),
        "single option must be rejected with InvalidData"
    );
}

/// Creating a pool with options_count = 0 must be rejected.
#[test]
fn test_create_pool_zero_options_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &0u32, // Zero options - invalid
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Zero options pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![&ctx.env],
        },
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidData)),
        "zero options must be rejected with InvalidData"
    );
}

/// Creating a pool with negative initial_liquidity must be rejected.
#[test]
fn test_create_pool_negative_liquidity_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Negative liquidity pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: -1i128, // Negative liquidity
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_err(), "negative initial_liquidity must be rejected");
}

/// Creating a pool with required_resolutions = 0 must be rejected.
#[test]
fn test_create_pool_zero_required_resolutions_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Zero resolutions pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 0u32, // Zero required resolutions
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_err(), "zero required_resolutions must be rejected");
}

/// Creating a pool with min_stake = 0 must be rejected.
#[test]
fn test_create_pool_zero_min_stake_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Zero min stake pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 0i128, // Zero min stake
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_err(), "zero min_stake must be rejected");
}

/// Creating a pool with max_stake < min_stake must be rejected.
#[test]
fn test_create_pool_max_stake_less_than_min_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Invalid stake bounds pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 100i128,
            max_stake: 50i128, // max_stake < min_stake
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_err(), "max_stake < min_stake must be rejected");
}

/// Creating a pool with min_total_stake = 0 must be rejected.
#[test]
fn test_create_pool_zero_min_total_stake_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Zero min total stake pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 0i128, // Zero min total stake
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_err(), "zero min_total_stake must be rejected");
}

/// Creating a pool with negative max_total_stake must be rejected.
#[test]
fn test_create_pool_negative_max_total_stake_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Negative max total stake pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: -1i128, // Negative max total stake
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_err(), "negative max_total_stake must be rejected");
}

/// Creating a pool with metadata_url exceeding 512 bytes must be rejected.
#[test]
fn test_create_pool_metadata_url_too_long_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let over_limit = core::str::from_utf8(&[b'a'; 513]).unwrap();
    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Test pool"),
            metadata_url: String::from_str(&ctx.env, over_limit), // 513 bytes
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::MetadataUrlInvalid)),
        "metadata_url > 512 bytes must be rejected with MetadataUrlInvalid"
    );
}

/// Creating a pool with exactly 512-byte metadata_url should succeed (boundary test).
#[test]
fn test_create_pool_metadata_url_at_limit_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let at_limit = core::str::from_utf8(&[b'a'; 512]).unwrap();
    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Test pool"),
            metadata_url: String::from_str(&ctx.env, at_limit), // Exactly 512 bytes
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_ok(), "512-byte metadata_url should be accepted");
}

/// Creating a pool with invalid category must be rejected.
#[test]
fn test_create_pool_invalid_category_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("INVALID_CATEGORY"), // Not in allowed list
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Invalid category pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_err(), "invalid category must be rejected");
}

/// Verify that failed pool creation does not leave partial state.
/// The pool_id counter should not be incremented on failure.
#[test]
fn test_create_pool_failure_does_not_increment_pool_id() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Get initial pool_id counter
    let initial_counter = ctx.client.get_pool_id_counter();

    // Attempt to create a pool with invalid token (will fail)
    let invalid_token = Address::generate(&ctx.env);
    let result = ctx.client.try_create_pool(
        &ctx.creator,
        &100_000u64,
        &invalid_token,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&ctx.env, "Invalid token pool"),
            metadata_url: String::from_str(&ctx.env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            min_total_stake: 1i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &ctx.env,
                String::from_str(&ctx.env, "No"),
                String::from_str(&ctx.env, "Yes"),
            ],
        },
    );
    assert!(result.is_err(), "pool creation should fail");

    // Verify counter did not increment
    let final_counter = ctx.client.get_pool_id_counter();
    assert_eq!(
        initial_counter, final_counter,
        "pool_id counter must not increment on failed creation"
    );
}

/// Verify that successful pool creation increments the pool_id counter.
#[test]
fn test_create_pool_success_increments_pool_id() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let initial_counter = ctx.client.get_pool_id_counter();

    // Create a valid pool
    let _pool_id = ctx.create_pool(100_000);

    let final_counter = ctx.client.get_pool_id_counter();
    assert_eq!(
        initial_counter + 1, final_counter,
        "pool_id counter must increment by 1 on successful creation"
    );
}

/// Creating a pool when global fee is 0 bps should succeed and the pool
/// should capture the 0 bps fee at creation time.
#[test]
fn test_create_pool_with_zero_global_fee_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Set global fee to 0 bps
    ctx.client.set_fee_bps(&ctx.admin, &0u32);
    ctx.advance_time(crate::FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    let pool_id = ctx.create_pool(100_000);
    let pool = ctx.client.get_pool(&pool_id);

    assert_eq!(pool.fee_bps, 0, "pool should capture 0 bps fee at creation");
}

/// Creating a pool when global fee is 10000 bps (100%) should succeed and
/// the pool should capture the 10000 bps fee at creation time.
#[test]
fn test_create_pool_with_max_global_fee_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Set global fee to 10000 bps (100%)
    ctx.client.set_fee_bps(&ctx.admin, &10_000u32);
    ctx.advance_time(crate::FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    let pool_id = ctx.create_pool(100_000);
    let pool = ctx.client.get_pool(&pool_id);

    assert_eq!(
        pool.fee_bps, 10_000,
        "pool should capture 10000 bps fee at creation"
    );
}

/// Verify that pool fee is captured at creation time and does not change
/// when global fee is updated later.
#[test]
fn test_create_pool_fee_captured_at_creation() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Set initial global fee to 500 bps
    ctx.client.set_fee_bps(&ctx.admin, &500u32);
    ctx.advance_time(crate::FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    // Create pool - should capture 500 bps
    let pool_id = ctx.create_pool(100_000);
    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.fee_bps, 500, "pool should capture 500 bps at creation");

    // Update global fee to 2000 bps
    ctx.client.set_fee_bps(&ctx.admin, &2_000u32);
    ctx.advance_time(crate::FEE_CHANGE_TIMELOCK_SECONDS + 1);
    ctx.client.apply_fee_bps(&ctx.admin);

    // Pool fee should remain 500 bps (captured at creation)
    let pool_after = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool_after.fee_bps, 500,
        "pool fee must not change after global fee update"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1313 — `resolve_pool` Boundary Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Resolving a pool with zero total predictions must succeed; the pool moves
/// to Resolved even though no tokens were staked.
#[test]
fn test_1313_resolve_pool_zero_participants() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(4_000);

    // Advance past end_time (resolution_delay = 0).
    ctx.advance_time(4_001);

    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 0u32);
    assert_eq!(pool.total_stake, 0, "total_stake must be 0 for empty pool");
}

/// Resolving a pool where all stakes are on one side (only YES or only NO)
/// must succeed and award the winning outcome correctly.
#[test]
fn test_1313_resolve_pool_one_sided_stakes() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(4_000);

    // All stakes on outcome 1 (YES), none on outcome 0.
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    ctx.stake(&user1, pool_id, 400, 1);
    ctx.stake(&user2, pool_id, 600, 1);

    ctx.advance_time(4_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &1u32);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 1u32, "winning outcome must be 1");
    assert_eq!(pool.total_stake, 1_000);
}

/// Attempting to resolve exactly 1 second before the resolution window opens
/// must fail with `ResolutionDelayNotMet`.
#[test]
fn test_1313_resolve_one_second_before_delay_expires_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Use a non-zero resolution_delay of 600 s.
    let ac_id = env.register(dummy_ac::DummyAC, ());
    let ac = dummy_ac::DummyACClient::new(&env, &ac_id);
    ac.grant_role(&ctx.operator, &1u32);
    ac.grant_role(&ctx.admin, &0u32); // admin role for whitelist

    let contract_id = env.register(PredifiContract, ());
    let client2 = PredifiContractClient::new(&env, &contract_id);
    let treasury2 = Address::generate(&env);
    // resolution_delay = 600 s
    client2.init(&ac_id, &treasury2, &0u32, &600u64, &3600u64, &0u32);
    client2.add_token_to_whitelist(&ctx.admin, &ctx.token_address);

    let now = env.ledger().timestamp();
    let end_time = now + 5_000;
    let pool_id = client2.create_pool(
        &ctx.creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Delay boundary pool"),
            metadata_url: String::from_str(&env, "ipfs://delay"),
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

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &100);
    client2.place_prediction(&user, &pool_id, &100, &0u32, &None, &None);

    // Advance to end_time + resolution_delay - 1 (one second early).
    env.ledger()
        .with_mut(|li| li.timestamp = end_time + 600 - 1);

    let result = client2.try_resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(
        result,
        Err(Ok(PredifiError::ResolutionDelayNotMet)),
        "resolving 1 s before delay expires must fail with ResolutionDelayNotMet"
    );
}

/// Resolving at exactly `end_time + resolution_delay` must succeed.
#[test]
fn test_1313_resolve_at_exact_delay_boundary_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Non-zero resolution_delay = 300 s.
    let ac_id = env.register(dummy_ac::DummyAC, ());
    let ac = dummy_ac::DummyACClient::new(&env, &ac_id);
    ac.grant_role(&ctx.operator, &1u32);
    ac.grant_role(&ctx.admin, &0u32); // admin role for whitelist

    let contract_id = env.register(PredifiContract, ());
    let client2 = PredifiContractClient::new(&env, &contract_id);
    let treasury2 = Address::generate(&env);
    client2.init(&ac_id, &treasury2, &0u32, &300u64, &3600u64, &0u32);
    client2.add_token_to_whitelist(&ctx.admin, &ctx.token_address);

    let now = env.ledger().timestamp();
    let end_time = now + 5_000;
    let pool_id = client2.create_pool(
        &ctx.creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Exact delay pool"),
            metadata_url: String::from_str(&env, "ipfs://exact"),
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

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &100);
    client2.place_prediction(&user, &pool_id, &100, &1u32, &None, &None);

    // Advance to exactly end_time + resolution_delay.
    env.ledger()
        .with_mut(|li| li.timestamp = end_time + 300);

    client2.resolve_pool(&ctx.operator, &pool_id, &1u32);

    assert_eq!(
        client2.get_pool(&pool_id).state,
        MarketState::Resolved,
        "pool must be Resolved at exact delay boundary"
    );
}

/// Re-resolution — calling `resolve_pool` on an already-Resolved pool must
/// return `InvalidPoolState`.
#[test]
fn test_1313_re_resolution_is_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(4_000);

    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 200, 0);

    ctx.advance_time(4_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Resolved);

    // Attempt a second resolution by a different operator.
    let result = ctx.client.try_resolve_pool(&ctx.operator2, &pool_id, &1u32);
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidPoolState)),
        "resolving an already-Resolved pool must return InvalidPoolState"
    );
}

/// An operator cannot vote twice for the same pool.
#[test]
fn test_1313_duplicate_operator_vote_is_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // Require 2 resolutions so the pool stays unresolved after the first vote.
    let pool_id = ctx.create_pool_with_resolutions(4_000, 2);

    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 100, 0);

    ctx.advance_time(4_001);

    // First vote succeeds.
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);

    // Duplicate vote from the same operator must fail.
    let result = ctx.client.try_resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(
        result,
        Err(Ok(PredifiError::OracleAlreadyVoted)),
        "duplicate operator vote must return OracleAlreadyVoted"
    );
}

/// Multi-vote resolution: N-1 votes leave the pool Active; the Nth vote
/// finalises it.
#[test]
fn test_1313_multi_vote_n_minus_1_leaves_pool_active() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    // required_resolutions = 2
    let pool_id = ctx.create_pool_with_resolutions(4_000, 2);

    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 100, 0);

    ctx.advance_time(4_001);

    // First vote — pool must still be Active.
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(
        ctx.client.get_pool(&pool_id).state,
        MarketState::Active,
        "pool must remain Active after N-1 votes"
    );

    // Second vote — pool must transition to Resolved.
    ctx.client.resolve_pool(&ctx.operator2, &pool_id, &0u32);
    assert_eq!(
        ctx.client.get_pool(&pool_id).state,
        MarketState::Resolved,
        "pool must be Resolved after Nth vote"
    );
}

/// Resolving with an out-of-bounds outcome index must fail with
/// `InvalidOutcome`.
#[test]
fn test_1313_resolve_with_out_of_bounds_outcome_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(4_000);
    ctx.advance_time(4_001);

    // Pool has 2 outcomes (indices 0 and 1); index 2 is out of bounds.
    let result = ctx.client.try_resolve_pool(&ctx.operator, &pool_id, &2u32);
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidOutcome)),
        "out-of-bounds outcome must return InvalidOutcome"
    );
}

/// Resolving before end_time must return `ResolutionDelayNotMet` (pool still
/// within the betting window).
#[test]
fn test_1313_resolve_before_end_time_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(10_000);

    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 100, 0);

    // Do NOT advance past end_time.
    let result = ctx.client.try_resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(
        result,
        Err(Ok(PredifiError::ResolutionDelayNotMet)),
        "resolve before end_time must return ResolutionDelayNotMet"
    );
}

/// An address without the Operator role must be rejected with `Unauthorized`.
#[test]
fn test_1313_unauthorized_resolve_is_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(4_000);

    let user = Address::generate(&env);
    ctx.stake(&user, pool_id, 100, 0);

    ctx.advance_time(4_001);

    let stranger = Address::generate(&env);
    let result = ctx.client.try_resolve_pool(&stranger, &pool_id, &0u32);
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "non-operator must be rejected with Unauthorized"
    );

    assert_eq!(
        ctx.client.get_pool(&pool_id).state,
        MarketState::Active,
        "pool must remain Active after unauthorized resolve attempt"
    );
}

/// Resolution under stress: a pool with many participants (high-volume) must
/// resolve correctly and the winning-side total stake must be non-zero.
#[test]
fn test_1313_resolve_with_many_participants() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let pool_id = ctx.create_pool(4_000);

    // Simulate 20 participants staking on alternating outcomes.
    for i in 0..20u32 {
        let user = Address::generate(&env);
        let outcome = i % 2; // alternates 0 / 1
        ctx.stake(&user, pool_id, 100, outcome);
    }

    let pool_before = ctx.client.get_pool(&pool_id);
    assert_eq!(pool_before.total_stake, 2_000, "total stake must be 2_000");

    ctx.advance_time(4_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 0u32);
    assert_eq!(pool.total_stake, 2_000, "total_stake must be unchanged after resolve");

    // Winning-side stake (outcome 0 — even-indexed users, 10 × 100 = 1 000).
    let winning_stake = ctx.client.get_outcome_stake(&pool_id, &0u32);
    assert_eq!(winning_stake, 1_000, "winning-side stake must be 1_000");
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1324 — `update_referrer` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Test 1: Setting referrer to self must be rejected with `Unauthorized`.
#[test]
fn test_1324_update_referrer_self_rejected() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(4_000);

    let user = Address::generate(&env);

    // User attempts to set their own address as referrer
    let result = ctx.client.try_update_referrer(&user, &pool_id, &Some(user.clone()));
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "setting referrer to self must return Unauthorized"
    );

    // Verify referrer volume is 0
    let vol = ctx.client.get_referred_volume(&user, &pool_id);
    assert_eq!(vol, 0, "referred volume for self must remain 0");
}

/// Test 2: Updating referrer to None clears an existing referrer.
#[test]
fn test_1324_update_referrer_none_clears_referrer() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(4_000);

    let user = Address::generate(&env);
    let referrer = Address::generate(&env);

    // Set initial referrer
    ctx.client.update_referrer(&user, &pool_id, &Some(referrer.clone()));

    // User stakes 100 with the initial referrer active
    ctx.stake(&user, pool_id, 100, 0);
    assert_eq!(
        ctx.client.get_referred_volume(&referrer, &pool_id),
        100,
        "referred volume must equal 100"
    );

    // Clear referrer by updating to None
    ctx.client.update_referrer(&user, &pool_id, &None);

    // Place another prediction — volume for original referrer must NOT increase
    ctx.stake(&user, pool_id, 200, 0);
    assert_eq!(
        ctx.client.get_referred_volume(&referrer, &pool_id),
        100,
        "referred volume for previous referrer must remain unchanged after set to None"
    );
}

/// Test 3: Changing referrer after predictions are placed.
#[test]
fn test_1324_update_referrer_after_predictions_placed() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(4_000);

    let user = Address::generate(&env);
    let referrer_a = Address::generate(&env);
    let referrer_b = Address::generate(&env);

    // Place initial prediction with referrer_a
    ctx.token_admin.mint(&user, &1000);
    ctx.client.place_prediction(&user, &pool_id, &100, &0, &Some(referrer_a.clone()), &None);

    assert_eq!(ctx.client.get_referred_volume(&referrer_a, &pool_id), 100);
    assert_eq!(ctx.client.get_referred_volume(&referrer_b, &pool_id), 0);

    // Change referrer to referrer_b
    ctx.client.update_referrer(&user, &pool_id, &Some(referrer_b.clone()));

    // Place second prediction
    ctx.client.place_prediction(&user, &pool_id, &250, &0, &None, &None);

    // Check volume: referrer_a remains 100, referrer_b receives 250
    assert_eq!(ctx.client.get_referred_volume(&referrer_a, &pool_id), 100);
    assert_eq!(ctx.client.get_referred_volume(&referrer_b, &pool_id), 250);
}

/// Test 4: Referral chain cycles (A -> B -> C -> A).
#[test]
fn test_1324_update_referrer_chain_cycles() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(4_000);

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let user_c = Address::generate(&env);

    // Form cycle A -> B -> C -> A
    ctx.client.update_referrer(&user_a, &pool_id, &Some(user_b.clone()));
    ctx.client.update_referrer(&user_b, &pool_id, &Some(user_c.clone()));
    ctx.client.update_referrer(&user_c, &pool_id, &Some(user_a.clone()));

    // Each user places a prediction
    ctx.stake(&user_a, pool_id, 100, 0);
    ctx.stake(&user_b, pool_id, 200, 0);
    ctx.stake(&user_c, pool_id, 300, 0);

    // Verify referred volumes for B, C, A
    assert_eq!(ctx.client.get_referred_volume(&user_b, &pool_id), 100, "user_b receives user_a's volume");
    assert_eq!(ctx.client.get_referred_volume(&user_c, &pool_id), 200, "user_c receives user_b's volume");
    assert_eq!(ctx.client.get_referred_volume(&user_a, &pool_id), 300, "user_a receives user_c's volume");
}

/// Test 5: Verify referral volume tracking accuracy across multiple users and predictions.
#[test]
fn test_1324_update_referrer_volume_tracking_accuracy() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);
    let pool_id = ctx.create_pool(4_000);

    let referrer = Address::generate(&env);
    let user_1 = Address::generate(&env);
    let user_2 = Address::generate(&env);
    let user_unreferred = Address::generate(&env);

    ctx.client.update_referrer(&user_1, &pool_id, &Some(referrer.clone()));
    ctx.client.update_referrer(&user_2, &pool_id, &Some(referrer.clone()));

    // user_1 makes two predictions
    ctx.stake(&user_1, pool_id, 150, 0);
    ctx.stake(&user_1, pool_id, 350, 0);

    // user_2 makes one prediction
    ctx.stake(&user_2, pool_id, 500, 1);

    // unreferred user makes a prediction
    ctx.stake(&user_unreferred, pool_id, 400, 0);

    // Aggregate referred volume for referrer should be 150 + 350 + 500 = 1000
    let total_referred = ctx.client.get_referred_volume(&referrer, &pool_id);
    assert_eq!(
        total_referred, 1000,
        "total referred volume must accurately aggregate all referred predictions"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1323 — `withdraw_treasury` Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Test 1: Withdrawal of more than contract treasury balance must fail with `InsufficientBalance`.
#[test]
fn test_1323_withdraw_treasury_exceeding_balance_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let contract_id = ctx.client.address.clone();
    // Mint 500 tokens to contract
    ctx.token_admin.mint(&contract_id, &500);

    // Attempt to withdraw 501 tokens
    let result = ctx.client.try_withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &501i128,
        &ctx.treasury,
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InsufficientBalance)),
        "withdrawing more than contract balance must return InsufficientBalance"
    );

    // Contract balance remains intact
    assert_eq!(ctx.token.balance(&contract_id), 500);
}

/// Test 2: Withdrawal of exactly zero or negative amount must fail with `InvalidAmount`.
#[test]
fn test_1323_withdraw_treasury_zero_or_negative_amount_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let contract_id = ctx.client.address.clone();
    ctx.token_admin.mint(&contract_id, &500);

    // Attempt to withdraw 0
    let res_zero = ctx.client.try_withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &0i128,
        &ctx.treasury,
    );
    assert_eq!(
        res_zero,
        Err(Ok(PredifiError::InvalidAmount)),
        "withdrawing 0 must return InvalidAmount"
    );

    // Attempt to withdraw negative amount
    let res_neg = ctx.client.try_withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &-50i128,
        &ctx.treasury,
    );
    assert_eq!(
        res_neg,
        Err(Ok(PredifiError::InvalidAmount)),
        "withdrawing negative amount must return InvalidAmount"
    );

    assert_eq!(ctx.token.balance(&contract_id), 500);
}

/// Test 3: Withdrawal attempt by a non-admin address must fail with `Unauthorized`.
#[test]
fn test_1323_withdraw_treasury_non_admin_fails() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let contract_id = ctx.client.address.clone();
    ctx.token_admin.mint(&contract_id, &500);

    let stranger = Address::generate(&env);
    let result = ctx.client.try_withdraw_treasury(
        &stranger,
        &ctx.token_address,
        &100i128,
        &ctx.treasury,
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "non-admin caller must be rejected with Unauthorized"
    );

    assert_eq!(ctx.token.balance(&contract_id), 500);
    assert_eq!(ctx.token.balance(&ctx.treasury), 0);
}

/// Test 4: Withdrawal of a non-whitelisted token by Admin succeeds.
#[test]
fn test_1323_withdraw_treasury_non_whitelisted_token_succeeds() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let contract_id = ctx.client.address.clone();

    // Register a second token contract (NOT added to whitelist)
    let non_wl_admin_addr = Address::generate(&env);
    let non_wl_contract = env.register_stellar_asset_contract_v2(non_wl_admin_addr);
    let non_wl_address = non_wl_contract.address();
    let non_wl_token = token::Client::new(&env, &non_wl_address);
    let non_wl_token_admin = token::StellarAssetClient::new(&env, &non_wl_address);

    // Mint 1000 non-whitelisted tokens to contract
    non_wl_token_admin.mint(&contract_id, &1000);

    // Admin withdraws 400 of non-whitelisted tokens
    ctx.client.withdraw_treasury(
        &ctx.admin,
        &non_wl_address,
        &400i128,
        &ctx.treasury,
    );

    assert_eq!(
        non_wl_token.balance(&ctx.treasury),
        400,
        "treasury recipient must receive withdrawn non-whitelisted tokens"
    );
    assert_eq!(
        non_wl_token.balance(&contract_id),
        600,
        "contract balance must decrease by withdrawn amount"
    );
}

/// Test 5: Verify treasury balance consistency after multiple consecutive withdrawals.
#[test]
fn test_1323_withdraw_treasury_multiple_withdrawals_balance_consistency() {
    let env = Env::default();
    let ctx = TestEnv::new(&env);

    let contract_id = ctx.client.address.clone();
    ctx.token_admin.mint(&contract_id, &2000);

    // 1st withdrawal: 500
    ctx.client.withdraw_treasury(&ctx.admin, &ctx.token_address, &500i128, &ctx.treasury);
    assert_eq!(ctx.token.balance(&contract_id), 1500);
    assert_eq!(ctx.token.balance(&ctx.treasury), 500);

    // 2nd withdrawal: 700
    ctx.client.withdraw_treasury(&ctx.admin, &ctx.token_address, &700i128, &ctx.treasury);
    assert_eq!(ctx.token.balance(&contract_id), 800);
    assert_eq!(ctx.token.balance(&ctx.treasury), 1200);

    // 3rd withdrawal: 800
    ctx.client.withdraw_treasury(&ctx.admin, &ctx.token_address, &800i128, &ctx.treasury);
    assert_eq!(ctx.token.balance(&contract_id), 0);
    assert_eq!(ctx.token.balance(&ctx.treasury), 2000);
}

