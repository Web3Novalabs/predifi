//! Boundary & edge-case coverage for issues #1880, #1886, #1887, and #1888.
//!
//! * **#1886 `add_token_to_whitelist`** - Boundary & edge cases:
//!   - Adding duplicate token is idempotent and does not duplicate list entries.
//!   - Adding multiple tokens works without capacity limits.
//!   - Adding non-contract / arbitrary address registers in whitelist without panic.
//!   - Adding token with non-standard decimals (e.g. 18 decimals).
//!   - Removing and re-adding the same token.
//!   - Verifying pool creation with newly whitelisted token.
//!
//! * **#1880 `resolve_pool`** - Boundary & edge cases:
//!   - Resolution of a pool with no predictions placed.
//!   - Resolution with stakes on only one outcome.
//!   - Exact `resolution_delay` boundary enforcement (fails at T - 1, succeeds at T).
//!   - Prevention of re-resolution (second resolve attempt fails with `InvalidPoolState`).
//!   - Resolution of a pool with multiple participants.
//!
//! * **#1887 `remove_token_from_whitelist`** - Boundary & edge cases:
//!   - Removing a token with active pools preserves pool validity and existing predictions.
//!   - Removing last whitelisted token leaves whitelist empty.
//!   - Removing non-existent token is a safe no-op.
//!   - Sequential removal of multiple tokens.
//!
//! * **#1888 `add_oracle` / `remove_oracle`** - Boundary & edge cases:
//!   - Adding duplicate oracle is idempotent.
//!   - Removing an oracle updates the whitelist correctly.
//!   - Removing the last oracle.
//!   - Adding multiple oracles.
//!   - Non-admin attempts to add or remove oracles are rejected.

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

mod odd_decimals_token {
    use soroban_sdk::{contract, contractimpl, Address, Env, String};

    #[contract]
    pub struct OddDecimalsToken;

    #[contractimpl]
    impl OddDecimalsToken {
        pub fn decimals(_env: Env) -> u32 {
            18
        }
        pub fn name(env: Env) -> String {
            String::from_str(&env, "EighteenDecimals")
        }
        pub fn symbol(env: Env) -> String {
            String::from_str(&env, "E18")
        }
        pub fn balance(_env: Env, _id: Address) -> i128 {
            0
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
    // resolution_delay = 3600 seconds, min_pool_duration = 3600 seconds
    client.init(&ac_id, &treasury, &0u32, &3600u64, &3600u64, &0u32);

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

fn token_whitelist_list(env: &Env, ctx: &Ctx<'_>) -> Vec<Address> {
    env.as_contract(&ctx.client.address, || {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<Address>>(&DataKey::TokenWhitelist)
            .unwrap_or_else(|| Vec::new(env))
    })
}

fn oracle_whitelist_list(env: &Env, ctx: &Ctx<'_>) -> Vec<Address> {
    env.as_contract(&ctx.client.address, || {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<Address>>(&DataKey::OracleWhitelist)
            .unwrap_or_else(|| Vec::new(env))
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1886 — add_token_to_whitelist boundary & edge case tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_add_duplicate_token_is_idempotent() {
    let env = Env::default();
    let ctx = setup(&env);
    let token = Address::generate(&env);

    ctx.client.add_token_to_whitelist(&ctx.admin, &token);
    let count_before = token_whitelist_list(&env, &ctx).len();

    // Adding duplicate token repeatedly
    ctx.client.add_token_to_whitelist(&ctx.admin, &token);
    ctx.client.add_token_to_whitelist(&ctx.admin, &token);

    assert!(ctx.client.is_token_allowed(&token));
    assert_eq!(token_whitelist_list(&env, &ctx).len(), count_before);
}

#[test]
fn test_add_maximum_number_of_tokens() {
    let env = Env::default();
    let ctx = setup(&env);

    let mut tokens = vec![&env];
    for _ in 0..30u32 {
        let t = Address::generate(&env);
        ctx.client.add_token_to_whitelist(&ctx.admin, &t);
        tokens.push_back(t);
    }

    for t in tokens.iter() {
        assert!(ctx.client.is_token_allowed(&t));
    }
}

#[test]
fn test_add_non_contract_address_succeeds() {
    let env = Env::default();
    let ctx = setup(&env);
    let random_addr = Address::generate(&env);

    ctx.client.add_token_to_whitelist(&ctx.admin, &random_addr);
    assert!(ctx.client.is_token_allowed(&random_addr));
}

#[test]
fn test_add_token_with_non_standard_decimals() {
    let env = Env::default();
    let ctx = setup(&env);

    let odd_token = env.register(odd_decimals_token::OddDecimalsToken, ());
    ctx.client.add_token_to_whitelist(&ctx.admin, &odd_token);
    assert!(ctx.client.is_token_allowed(&odd_token));
}

#[test]
fn test_remove_and_re_add_same_token() {
    let env = Env::default();
    let ctx = setup(&env);
    let token = Address::generate(&env);

    ctx.client.add_token_to_whitelist(&ctx.admin, &token);
    assert!(ctx.client.is_token_allowed(&token));

    ctx.client.remove_token_from_whitelist(&ctx.admin, &token);
    assert!(!ctx.client.is_token_allowed(&token));

    ctx.client.add_token_to_whitelist(&ctx.admin, &token);
    assert!(ctx.client.is_token_allowed(&token));
}

#[test]
fn test_pool_creation_with_newly_whitelisted_token() {
    let env = Env::default();
    let ctx = setup(&env);
    let creator = Address::generate(&env);

    let deployer = Address::generate(&env);
    let new_token = env.register_stellar_asset_contract_v2(deployer).address();

    ctx.client.add_token_to_whitelist(&ctx.admin, &new_token);
    assert!(ctx.client.is_token_allowed(&new_token));

    let end_time = 10_000u64;
    let pool_id = ctx.client.create_pool(
        &creator,
        &end_time,
        &new_token,
        &2u32,
        &symbol_short!("Sports"),
        &config(&env, 1i128, 0i128),
    );

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.token, new_token);
    assert_eq!(pool.state, MarketState::Active);
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1880 — resolve_pool boundary & edge case tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_resolve_pool_with_no_predictions() {
    let env = Env::default();
    let ctx = setup(&env);
    let creator = Address::generate(&env);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &creator, end_time);

    // Fast-forward past end_time + resolution_delay (10_000 + 3600 = 13_600)
    env.ledger().with_mut(|li| li.timestamp = 14_000);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 0);
}

#[test]
fn test_resolve_pool_with_stakes_on_only_one_side() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &500);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &user, end_time);

    // Only outcome 1 receives stakes
    ctx.client.place_prediction(&user, &pool_id, &500, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 14_000);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &1);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 1);
}

#[test]
fn test_resolve_pool_at_exact_resolution_delay_boundary() {
    let env = Env::default();
    let ctx = setup(&env);
    let creator = Address::generate(&env);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &creator, end_time);
    let resolution_delay = 3_600u64;

    // At end_time + resolution_delay - 1 (13_599), resolution must fail
    env.ledger().with_mut(|li| li.timestamp = end_time + resolution_delay - 1);
    let early_res = ctx.client.try_resolve_pool(&ctx.operator, &pool_id, &0);
    assert!(early_res.is_err());

    // At exact boundary (13_600), resolution succeeds
    env.ledger().with_mut(|li| li.timestamp = end_time + resolution_delay);
    let res = ctx.client.try_resolve_pool(&ctx.operator, &pool_id, &0);
    assert!(res.is_ok());

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
}

#[test]
fn test_resolve_pool_re_resolution_prevention() {
    let env = Env::default();
    let ctx = setup(&env);
    let creator = Address::generate(&env);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &creator, end_time);

    env.ledger().with_mut(|li| li.timestamp = 14_000);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0);

    // Second resolution attempt must fail
    let re_res = ctx.client.try_resolve_pool(&ctx.operator, &pool_id, &1);
    assert!(re_res.is_err());
}

#[test]
fn test_resolve_pool_with_multiple_participants() {
    let env = Env::default();
    let ctx = setup(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    ctx.token_admin.mint(&alice, &400);
    ctx.token_admin.mint(&bob, &600);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &alice, end_time);

    ctx.client.place_prediction(&alice, &pool_id, &400, &0, &None, &None);
    ctx.client.place_prediction(&bob, &pool_id, &600, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 14_000);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1887 — remove_token_from_whitelist boundary & edge case tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_remove_token_with_active_pools_preserves_pool_functionality() {
    let env = Env::default();
    let ctx = setup(&env);
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &500);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &user, end_time);
    ctx.client.place_prediction(&user, &pool_id, &500, &1, &None, &None);

    // Remove token from whitelist
    ctx.client.remove_token_from_whitelist(&ctx.admin, &ctx.token_address);
    assert!(!ctx.client.is_token_allowed(&ctx.token_address));

    // Existing pool is still active and functions properly
    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Active);

    // Resolving and claiming from existing pool still succeeds
    env.ledger().with_mut(|li| li.timestamp = 14_000);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &1);
    let payout = ctx.client.claim_winnings(&user, &pool_id);
    assert_eq!(payout, 500);

    // But creating a NEW pool with the delisted token is rejected
    let new_pool_res = ctx.client.try_create_pool(
        &user,
        &20_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &config(&env, 1i128, 0i128),
    );
    assert!(new_pool_res.is_err());
}

#[test]
fn test_remove_last_whitelisted_token() {
    let env = Env::default();
    let ctx = setup(&env);

    ctx.client.remove_token_from_whitelist(&ctx.admin, &ctx.token_address);
    assert!(!ctx.client.is_token_allowed(&ctx.token_address));
    assert_eq!(token_whitelist_list(&env, &ctx).len(), 0);
}

#[test]
fn test_remove_non_existent_token_is_safe_noop() {
    let env = Env::default();
    let ctx = setup(&env);
    let random_token = Address::generate(&env);

    let count_before = token_whitelist_list(&env, &ctx).len();
    ctx.client.remove_token_from_whitelist(&ctx.admin, &random_token);
    assert_eq!(token_whitelist_list(&env, &ctx).len(), count_before);
}

#[test]
fn test_sequential_removal_of_multiple_tokens() {
    let env = Env::default();
    let ctx = setup(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    ctx.client.add_token_to_whitelist(&ctx.admin, &a);
    ctx.client.add_token_to_whitelist(&ctx.admin, &b);

    assert!(ctx.client.is_token_allowed(&a));
    assert!(ctx.client.is_token_allowed(&b));

    ctx.client.remove_token_from_whitelist(&ctx.admin, &a);
    assert!(!ctx.client.is_token_allowed(&a));
    assert!(ctx.client.is_token_allowed(&b));

    ctx.client.remove_token_from_whitelist(&ctx.admin, &b);
    assert!(!ctx.client.is_token_allowed(&b));
}

// ═══════════════════════════════════════════════════════════════════════════
// Issue #1888 — add_oracle / remove_oracle boundary & edge case tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_add_duplicate_oracle_is_idempotent() {
    let env = Env::default();
    let ctx = setup(&env);
    let oracle = Address::generate(&env);

    ctx.client.add_oracle(&ctx.admin, &oracle);
    let count_before = oracle_whitelist_list(&env, &ctx).len();

    // Re-adding oracle
    ctx.client.add_oracle(&ctx.admin, &oracle);
    assert_eq!(oracle_whitelist_list(&env, &ctx).len(), count_before);
}

#[test]
fn test_remove_oracle_updates_whitelist() {
    let env = Env::default();
    let ctx = setup(&env);
    let oracle_a = Address::generate(&env);
    let oracle_b = Address::generate(&env);

    ctx.client.add_oracle(&ctx.admin, &oracle_a);
    ctx.client.add_oracle(&ctx.admin, &oracle_b);

    assert_eq!(oracle_whitelist_list(&env, &ctx).len(), 2);

    ctx.client.remove_oracle(&ctx.admin, &oracle_a);
    let list = oracle_whitelist_list(&env, &ctx);
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), oracle_b);
}

#[test]
fn test_remove_last_oracle() {
    let env = Env::default();
    let ctx = setup(&env);
    let oracle = Address::generate(&env);

    ctx.client.add_oracle(&ctx.admin, &oracle);
    assert_eq!(oracle_whitelist_list(&env, &ctx).len(), 1);

    ctx.client.remove_oracle(&ctx.admin, &oracle);
    assert_eq!(oracle_whitelist_list(&env, &ctx).len(), 0);
}

#[test]
fn test_add_many_oracles() {
    let env = Env::default();
    let ctx = setup(&env);

    for _ in 0..10u32 {
        let o = Address::generate(&env);
        ctx.client.add_oracle(&ctx.admin, &o);
    }

    assert_eq!(oracle_whitelist_list(&env, &ctx).len(), 10);
}

#[test]
fn test_oracle_operations_unauthorized_non_admin() {
    let env = Env::default();
    let ctx = setup(&env);
    let non_admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    let add_res = ctx.client.try_add_oracle(&non_admin, &oracle);
    assert!(add_res.is_err());

    ctx.client.add_oracle(&ctx.admin, &oracle);
    let remove_res = ctx.client.try_remove_oracle(&non_admin, &oracle);
    assert!(remove_res.is_err());
}
