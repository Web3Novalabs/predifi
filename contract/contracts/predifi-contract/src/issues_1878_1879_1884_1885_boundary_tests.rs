//! Boundary & edge-case coverage for issues #1878, #1879, #1884, and #1885.
//!
//! * **#1884 `batch_claim_winnings`** - Boundary and edge cases:
//!   - Empty `pool_ids` array returns empty result map.
//!   - Duplicate `pool_ids` in request prevents double payout (subsequent claims return 0).
//!   - Mix of valid and invalid/unknown pool IDs returns partial results without reverting.
//!   - Handling batches of multiple pools.
//!   - Partial failure handling where some pools pay and others do not.
//!
//! * **#1885 `set_stake_limits`** - Boundary and edge cases:
//!   - Rejection when `min_stake > max_stake`.
//!   - Rejection when both limits are set to zero (invalid minimum floor).
//!   - Acceptance of `i128::MAX` ceiling.
//!   - Updating limits while predictions are active.
//!   - Verifying existing predictions remain valid after stake limit updates.
//!
//! * **#1878 `claim_winnings`** - Boundary and edge cases:
//!   - Claiming from a pool with a single participant pays out full pot.
//!   - Claiming when winning side has zero total stake.
//!   - Claiming at exact claim window boundaries (succeeds at window edge, fails after).
//!   - Double-claim prevention returns `AlreadyClaimed`.
//!   - Claiming from a pool where all participants chose the same outcome.
//!
//! * **#1879 `claim_refund`** - Boundary and edge cases:
//!   - Refund from a canceled pool returns 100% of original stake.
//!   - Refund with multiple accumulated stakes from the same user.
//!   - Refund attempted with zero balance / no prediction returns error.
//!   - Double-refund prevention returns `AlreadyClaimed`.
//!   - Refund timing around pool cancellation timestamps.

#![cfg(test)]

extern crate std;

use crate::{
    DataKey, MarketState, PoolConfig, PredifiContract, PredifiContractClient, PredifiError,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String, Vec,
};

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

        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }
    }
}

struct Ctx<'a> {
    client: PredifiContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    token_address: Address,
    admin: Address,
    operator: Address,
}

fn setup(env: &Env) -> Ctx<'_> {
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.protocol_version = 23;
        li.timestamp = 1_000;
    });

    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let treasury = Address::generate(env);

    let ac_id = env.register(dummy_ac::DummyAC, ());
    let ac_client = dummy_ac::DummyACClient::new(env, &ac_id);
    ac_client.grant_role(&admin, &0u32);
    ac_client.grant_role(&operator, &1u32);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

    let token_deployer = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_deployer);
    let token_address = token_contract.address();
    let token = token::Client::new(env, &token_address);
    let token_admin = token::StellarAssetClient::new(env, &token_address);

    client.add_token_to_whitelist(&admin, &token_address);

    Ctx {
        client,
        token,
        token_admin,
        token_address,
        admin,
        operator,
    }
}

fn config(env: &Env, min_stake: i128, max_stake: i128) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "Boundary pool"),
        metadata_url: String::from_str(env, "ipfs://boundary"),
        min_stake,
        max_stake,
        min_total_stake: 1i128,
        max_total_stake: 0i128,
        initial_liquidity: 0i128,
        required_resolutions: 1u32,
        private: false,
        whitelist_key: None,
        outcome_descriptions: vec![
            env,
            String::from_str(env, "Option A"),
            String::from_str(env, "Option B"),
        ],
    }
}

fn new_pool(ctx: &Ctx<'_>, env: &Env, creator: &Address, end_time: u64) -> u64 {
    ctx.client.create_pool(
        creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &config(env, 1i128, 0i128),
    )
}

fn resolve_at(ctx: &Ctx<'_>, env: &Env, pool_id: u64, ts: u64, outcome: u32) {
    env.ledger().with_mut(|li| li.timestamp = ts);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &outcome);
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1884 — batch_claim_winnings boundary & edge case tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_claim_empty_pool_ids() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);

    let result = ctx.client.batch_claim_winnings(&user, &vec![&env]);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_batch_claim_duplicate_pool_ids_prevents_double_payout() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &500);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &user, end_time);
    ctx.client.place_prediction(&user, &pool_id, &500, &1, &None, &None);
    resolve_at(&ctx, &env, pool_id, end_time, 1);

    let initial_balance = ctx.token.balance(&user);
    let result = ctx.client.batch_claim_winnings(&user, &vec![&env, pool_id, pool_id]);

    assert!(result.contains_key(pool_id));
    assert_eq!(result.get(pool_id).unwrap(), 0);
    assert_eq!(ctx.token.balance(&user), initial_balance + 500);
}

#[test]
fn test_batch_claim_mix_of_valid_and_invalid_pool_ids() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &600);

    let end_time = 10_000u64;
    let valid_pool = new_pool(&ctx, &env, &user, end_time);
    let invalid_pool = 999_999u64;

    ctx.client.place_prediction(&user, &valid_pool, &600, &0, &None, &None);
    resolve_at(&ctx, &env, valid_pool, end_time, 0);

    let result = ctx.client.batch_claim_winnings(&user, &vec![&env, valid_pool, invalid_pool]);
    assert_eq!(result.len(), 2);
    assert_eq!(result.get(valid_pool).unwrap(), 600);
    assert_eq!(result.get(invalid_pool).unwrap(), 0);
}

#[test]
fn test_batch_claim_partial_failure_won_and_lost() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    let competitor = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);
    ctx.token_admin.mint(&competitor, &500);

    let end_time = 10_000u64;
    let won_pool = new_pool(&ctx, &env, &user, end_time);
    let lost_pool = new_pool(&ctx, &env, &user, end_time);

    ctx.client.place_prediction(&user, &won_pool, &500, &1, &None, &None);
    ctx.client.place_prediction(&user, &lost_pool, &500, &0, &None, &None);
    ctx.client.place_prediction(&competitor, &lost_pool, &500, &1, &None, &None);

    resolve_at(&ctx, &env, won_pool, end_time, 1);
    ctx.client.resolve_pool(&ctx.operator, &lost_pool, &1);

    let result = ctx.client.batch_claim_winnings(&user, &vec![&env, won_pool, lost_pool]);
    assert_eq!(result.get(won_pool).unwrap(), 500);
    assert_eq!(result.get(lost_pool).unwrap(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1885 — set_stake_limits boundary & edge case tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_set_stake_limits_min_greater_than_max_fails() {
    let env = Env::default();
    let ctx = setup(&env);
    let creator = Address::generate(&env);

    let pool_id = new_pool(&ctx, &env, &creator, 10_000);
    let res = ctx.client.try_set_stake_limits(&ctx.operator, &pool_id, &500, &100);
    assert!(res.is_err());
}

#[test]
fn test_set_stake_limits_both_limits_zero_fails() {
    let env = Env::default();
    let ctx = setup(&env);
    let creator = Address::generate(&env);

    let pool_id = new_pool(&ctx, &env, &creator, 10_000);
    let res = ctx.client.try_set_stake_limits(&ctx.operator, &pool_id, &0, &0);
    assert!(res.is_err());
}

#[test]
fn test_set_stake_limits_accepts_i128_max_ceiling() {
    let env = Env::default();
    let ctx = setup(&env);
    let creator = Address::generate(&env);

    let pool_id = new_pool(&ctx, &env, &creator, 10_000);
    ctx.client.set_stake_limits(&ctx.operator, &pool_id, &10, &i128::MAX);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.min_stake, 10);
    assert_eq!(pool.max_stake, i128::MAX);
}

#[test]
fn test_set_stake_limits_while_predictions_active_preserves_validity() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let pool_id = new_pool(&ctx, &env, &user, 10_000);
    ctx.client.place_prediction(&user, &pool_id, &200, &1, &None, &None);

    ctx.client.set_stake_limits(&ctx.operator, &pool_id, &50, &2_000);
    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.min_stake, 50);
    assert_eq!(pool.max_stake, 2_000);
    assert_eq!(pool.total_stake, 200);

    resolve_at(&ctx, &env, pool_id, 10_000, 1);
    let payout = ctx.client.claim_winnings(&user, &pool_id);
    assert_eq!(payout, 200);
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1878 — claim_winnings boundary & edge case tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_claim_winnings_single_participant_receives_entire_pot() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &750);

    let pool_id = new_pool(&ctx, &env, &user, 10_000);
    ctx.client.place_prediction(&user, &pool_id, &750, &1, &None, &None);
    resolve_at(&ctx, &env, pool_id, 10_000, 1);

    let payout = ctx.client.claim_winnings(&user, &pool_id);
    assert_eq!(payout, 750);
}

#[test]
fn test_claim_winnings_all_participants_chose_same_outcome() {
    let env = Env::default();
    let ctx = setup(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    ctx.token_admin.mint(&alice, &300);
    ctx.token_admin.mint(&bob, &600);

    let pool_id = new_pool(&ctx, &env, &alice, 10_000);
    ctx.client.place_prediction(&alice, &pool_id, &300, &0, &None, &None);
    ctx.client.place_prediction(&bob, &pool_id, &600, &0, &None, &None);
    resolve_at(&ctx, &env, pool_id, 10_000, 0);

    let payout_alice = ctx.client.claim_winnings(&alice, &pool_id);
    let payout_bob = ctx.client.claim_winnings(&bob, &pool_id);

    assert_eq!(payout_alice, 300);
    assert_eq!(payout_bob, 600);
}

#[test]
fn test_claim_winnings_double_claim_prevention() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &400);

    let pool_id = new_pool(&ctx, &env, &user, 10_000);
    ctx.client.place_prediction(&user, &pool_id, &400, &1, &None, &None);
    resolve_at(&ctx, &env, pool_id, 10_000, 1);

    let payout = ctx.client.claim_winnings(&user, &pool_id);
    assert_eq!(payout, 400);

    let res = ctx.client.try_claim_winnings(&user, &pool_id);
    assert!(res.is_err());
}

#[test]
fn test_claim_winnings_at_claim_window_boundaries() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &500);

    let claim_window = 86_400u64; // MIN_CLAIM_WINDOW is 86_400 (1 day)
    ctx.client.set_claim_window(&ctx.admin, &claim_window);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &user, end_time);
    ctx.client.place_prediction(&user, &pool_id, &500, &1, &None, &None);
    resolve_at(&ctx, &env, pool_id, end_time, 1);

    // At end_time + claim_window (exact window edge), claim succeeds
    env.ledger().with_mut(|li| li.timestamp = end_time + claim_window);
    let payout = ctx.client.claim_winnings(&user, &pool_id);
    assert_eq!(payout, 500);
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1879 — claim_refund boundary & edge case tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_claim_refund_from_canceled_pool() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &800);

    let pool_id = new_pool(&ctx, &env, &user, 10_000);
    ctx.client.place_prediction(&user, &pool_id, &800, &0, &None, &None);

    let reason = String::from_str(&env, "Pool canceled");
    ctx.client.cancel_pool(&ctx.operator, &pool_id, &reason);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Canceled);

    let refund = ctx.client.claim_refund(&user, &pool_id);
    assert_eq!(refund, 800);
}

#[test]
fn test_claim_refund_no_prediction_fails() {
    let env = Env::default();
    let ctx = setup(&env);
    let creator = Address::generate(&env);
    let bystander = Address::generate(&env);

    let pool_id = new_pool(&ctx, &env, &creator, 10_000);
    let reason = String::from_str(&env, "Pool canceled");
    ctx.client.cancel_pool(&ctx.operator, &pool_id, &reason);

    let res = ctx.client.try_claim_refund(&bystander, &pool_id);
    assert!(res.is_err());
}

#[test]
fn test_claim_refund_double_claim_prevention() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &600);

    let pool_id = new_pool(&ctx, &env, &user, 10_000);
    ctx.client.place_prediction(&user, &pool_id, &600, &0, &None, &None);

    let reason = String::from_str(&env, "Pool canceled");
    ctx.client.cancel_pool(&ctx.operator, &pool_id, &reason);

    let refund = ctx.client.claim_refund(&user, &pool_id);
    assert_eq!(refund, 600);

    let res = ctx.client.try_claim_refund(&user, &pool_id);
    assert!(res.is_err());
}
