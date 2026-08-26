//! Boundary & Edge Case Tests: `cancel_pool`
//!
//! Advanced boundary and edge case coverage for the `cancel_pool` entrypoint
//! of the PrediFi prediction market contract (issue #1443).
//!
//! # Scenarios covered
//!
//! 1. **Cancellation with active predictions** — a privileged caller
//!    (admin/operator) can cancel a pool that already holds bets.
//! 2. **Cancellation by a non-creator, non-privileged caller** — rejected
//!    with `Unauthorized`, both before and after the pool's stake exceeds its
//!    initial liquidity.
//! 3. **Cancellation of an already-resolved pool** — rejected with
//!    `InvalidPoolState`.
//! 4. **Cancellation of an already-cancelled pool** — rejected with
//!    `InvalidPoolState`.
//! 5. **State consistency after cancellation** — every participant's stake
//!    becomes refundable via `claim_winnings` (the cancel-path refund flow),
//!    each refund equals the original stake exactly, and the contract
//!    balance drains to zero once all refunds are claimed.
//! 6. **Overdue-pool failsafe** — once a pool is past
//!    `end_time + CANCELATION_DELAY`, any address (not just the creator or a
//!    privileged role) may cancel it.

#![cfg(test)]

use crate::{MarketState, PoolConfig, PredifiContract, PredifiContractClient, PredifiError};
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

// ─── Test environment helpers ─────────────────────────────────────────────────

struct Ctx<'a> {
    client: PredifiContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    token_address: Address,
    admin: Address,
    operator: Address,
}

/// Deploys the dummy access-control contract, the predifi contract, and a
/// whitelisted Stellar asset token. Ledger timestamp starts at 1_000.
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
    // fee_bps = 0, resolution_delay = 0, min_pool_duration = 3600, max_pred_per_user = 0
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

fn base_config(env: &Env) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "cancel_pool boundary pool"),
        metadata_url: String::from_str(env, "ipfs://boundary-cancel-pool"),
        min_stake: 1i128,
        max_stake: 0i128,
        min_total_stake: 1i128,
        max_total_stake: 0i128,
        initial_liquidity: 0i128,
        required_resolutions: 1u32,
        private: false,
        whitelist_key: None,
        outcome_descriptions: vec![
            env,
            String::from_str(env, "No"),
            String::from_str(env, "Yes"),
        ],
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. CANCELLATION WITH ACTIVE PREDICTIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_privileged_operator_can_cancel_pool_with_active_predictions() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &500);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &500, &0, &None, &None);

    let reason = String::from_str(&env, "resolved off-chain, refunding");
    ctx.client.cancel_pool(&ctx.operator, &pool_id, &reason);

    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. CANCELLATION BY A NON-CREATOR, NON-PRIVILEGED CALLER
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_cancel_by_random_non_creator_non_privileged_caller_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let random_caller = Address::generate(&env);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &base_config(&env),
    );

    let reason = String::from_str(&env, "trying to cancel someone else's pool");
    let result = ctx.client.try_cancel_pool(&random_caller, &pool_id, &reason);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::Unauthorized);
}

#[test]
fn test_creator_cannot_cancel_once_bets_exceed_initial_liquidity() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let bettor = Address::generate(&env);
    ctx.token_admin.mint(&bettor, &200);

    let end_time = 10_000u64;
    // initial_liquidity = 0, so any bet at all pushes total_stake above it.
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &base_config(&env),
    );

    ctx.client
        .place_prediction(&bettor, &pool_id, &200, &1, &None, &None);

    let reason = String::from_str(&env, "creator wants out after bets placed");
    let result = ctx.client.try_cancel_pool(&creator, &pool_id, &reason);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::Unauthorized);
}

#[test]
fn test_creator_can_cancel_when_no_bets_beyond_initial_liquidity() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Finance"),
        &base_config(&env),
    );

    // No predictions placed — total_stake (0) <= initial_liquidity (0).
    let reason = String::from_str(&env, "creator cancels their own empty pool");
    ctx.client.cancel_pool(&creator, &pool_id, &reason);

    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. CANCELLATION OF AN ALREADY-RESOLVED POOL
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_cancel_already_resolved_pool_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &base_config(&env),
    );

    env.ledger().with_mut(|li| li.timestamp = end_time);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Resolved);

    let reason = String::from_str(&env, "trying to cancel a resolved pool");
    let result = ctx.client.try_cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::InvalidPoolState);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. CANCELLATION OF AN ALREADY-CANCELLED POOL
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_cancel_already_cancelled_pool_is_rejected() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Tech"),
        &base_config(&env),
    );

    let reason1 = String::from_str(&env, "first cancellation");
    ctx.client.cancel_pool(&ctx.operator, &pool_id, &reason1);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);

    let reason2 = String::from_str(&env, "second cancellation attempt");
    let result = ctx
        .client
        .try_cancel_pool(&ctx.operator, &pool_id, &reason2);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::InvalidPoolState);
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. STATE CONSISTENCY AFTER CANCELLATION — ALL BALANCES REFUNDABLE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_all_participants_can_refund_their_exact_stake_after_cancellation() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &base_config(&env),
    );

    let stakes = [100i128, 250, 400, 50];
    let mut bettors: alloc::vec::Vec<(Address, i128)> = alloc::vec::Vec::new();
    for (i, &amount) in stakes.iter().enumerate() {
        let user = Address::generate(&env);
        ctx.token_admin.mint(&user, &amount);
        let outcome = (i % 2) as u32;
        ctx.client
            .place_prediction(&user, &pool_id, &amount, &outcome, &None, &None);
        bettors.push((user, amount));
    }

    let total_stake: i128 = stakes.iter().sum();
    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, total_stake);
    assert_eq!(ctx.token.balance(&ctx.client.address), total_stake);

    let reason = String::from_str(&env, "operator cancels pool with mixed stakes");
    ctx.client.cancel_pool(&ctx.operator, &pool_id, &reason);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);

    // Every bettor gets back exactly their original stake via claim_winnings
    // (the cancel-state branch pays 100% principal, no fee).
    for (user, amount) in &bettors {
        let refund = ctx.client.claim_winnings(user, &pool_id);
        assert_eq!(
            refund, *amount,
            "refund must equal the original stake exactly"
        );
        assert_eq!(ctx.token.balance(user), *amount);
    }

    // All funds have been returned; contract balance must be exactly zero.
    assert_eq!(
        ctx.token.balance(&ctx.client.address),
        0,
        "contract must hold zero residual balance once every refund is claimed"
    );

    // Double-claiming a refund must be rejected exactly like a double-claim
    // on a resolved pool (INV-3 applies uniformly to both pool outcomes).
    let (first_user, _) = &bettors[0];
    let result = ctx.client.try_claim_winnings(first_user, &pool_id);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::AlreadyClaimed);
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. OVERDUE-POOL FAILSAFE CANCELLATION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_overdue_pool_can_be_cancelled_by_any_address() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let random_caller = Address::generate(&env);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Politics"),
        &base_config(&env),
    );

    // CANCELATION_DELAY = 604_800 seconds (7 days) after end_time.
    env.ledger()
        .with_mut(|li| li.timestamp = end_time + 604_800 + 1);

    let reason = String::from_str(&env, "pool overdue, unlocking via failsafe");
    ctx.client.cancel_pool(&random_caller, &pool_id, &reason);

    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);
}

#[test]
fn test_pool_not_yet_overdue_still_rejects_random_caller() {
    let env = Env::default();
    let ctx = setup(&env);

    let creator = Address::generate(&env);
    let random_caller = Address::generate(&env);

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Politics"),
        &base_config(&env),
    );

    // Exactly at the overdue boundary (not past it): end_time + CANCELATION_DELAY.
    // The check is `current_time > overdue_threshold`, so equality is NOT overdue.
    env.ledger()
        .with_mut(|li| li.timestamp = end_time + 604_800);

    let reason = String::from_str(&env, "not yet overdue");
    let result = ctx
        .client
        .try_cancel_pool(&random_caller, &pool_id, &reason);
    assert_eq!(result.unwrap_err().unwrap(), PredifiError::Unauthorized);
}

extern crate alloc;
