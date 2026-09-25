//! Boundary & edge-case coverage for issues #1446, #1447, #1448 and #1449.
//!
//! Each of these four entry points already had some coverage before this
//! module; what follows deliberately targets the scenarios that were *not*
//! reachable from the existing suites:
//!
//! * **#1446 `batch_claim_winnings`** — partial-failure handling. The existing
//!   tests cover empty / duplicate / unknown ids, but never a batch where one
//!   pool genuinely pays and another genuinely does not.
//! * **#1447 `set_stake_limits`** — limit changes *while predictions are
//!   active*, and whether a prediction placed under the old limits survives.
//! * **#1448 `add_token_to_whitelist`** — list-level de-duplication, bulk
//!   admission, decimals-independence, and pool creation with a freshly
//!   admitted token.
//! * **#1449 `remove_token_from_whitelist`** — delisting a token that backs a
//!   live pool, emptying the whitelist, and back-to-back removals.
//!
//! The harness mirrors `claim_winnings_boundary_tests.rs` (self-contained
//! access-control stub + `Ctx`) so these tests do not depend on the shared
//! `test::setup` fixture and cannot be perturbed by changes to it.

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

// ─── Access-control stub ─────────────────────────────────────────────────────

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

// ─── A token reporting non-standard decimals ─────────────────────────────────
//
// Used only by the #1448 decimals test. `create_pool` never calls into the
// token contract (transfers happen in `place_prediction`), so a stub that
// answers metadata is enough to prove the whitelist is decimals-agnostic.

mod odd_decimals_token {
    use soroban_sdk::{contract, contractimpl, Address, Env, String};

    #[contract]
    pub struct OddDecimalsToken;

    #[contractimpl]
    impl OddDecimalsToken {
        /// 18 decimals — a common ERC-20 value, and not the 7 that Stellar
        /// assets use.
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

// ─── Test context ────────────────────────────────────────────────────────────

struct Ctx<'a> {
    client: PredifiContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    token_address: Address,
    admin: Address,
    operator: Address,
}

/// Registers the access-control stub, the predifi contract and one whitelisted
/// Stellar asset. `fee_bps = 0` and `resolution_delay = 0` keep payout
/// arithmetic exact and let pools resolve the moment `end_time` is reached.
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

/// Two-outcome pool: no fee, no ceiling, no total-stake cap. Leaving
/// `max_stake` and `max_total_stake` at 0 keeps checks 4–6 of
/// `validate_stake_limits` out of the way so #1447 tests isolate one rule at a
/// time.
fn config(env: &Env, min_stake: i128, max_stake: i128) -> PoolConfig {
    PoolConfig {
        start_time: 0,
        description: String::from_str(env, "Wave boundary pool"),
        metadata_url: String::from_str(env, "ipfs://wave-boundary"),
        min_stake,
        max_stake,
        // create_pool rejects a zero floor outright: "min_total_stake must be
        // greater than zero".
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

/// Reads the `DataKey::TokenWhitelist` enumeration list directly. There is no
/// public getter for it — `is_token_allowed` only reports the per-token
/// `TokenWl` flag — so list-level regressions (a duplicate entry, an entry
/// left behind by a removal) are invisible from the client surface.
fn whitelist_list(env: &Env, ctx: &Ctx<'_>) -> Vec<Address> {
    env.as_contract(&ctx.client.address, || {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<Address>>(&DataKey::TokenWhitelist)
            .unwrap_or_else(|| Vec::new(env))
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// #1446 — batch_claim_winnings: partial failure
// ═══════════════════════════════════════════════════════════════════════════

/// The scenario the existing suite never builds: one pool the caller genuinely
/// wins and one they genuinely lose, claimed in a single call.
///
/// Both must be reported, the winner must actually be paid, and the loss must
/// not abort the batch.
#[test]
fn batch_claim_pays_the_won_pool_and_reports_zero_for_the_lost_one() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    let rival = Address::generate(&env);
    ctx.token_admin.mint(&user, &800);
    ctx.token_admin.mint(&rival, &700);

    let end_time = 10_000u64;
    let won = new_pool(&ctx, &env, &user, end_time);
    let lost = new_pool(&ctx, &env, &user, end_time);

    // `won`: sole staker on the outcome that resolves true.
    ctx.client
        .place_prediction(&user, &won, &500, &1, &None, &None);
    // `lost`: user backs outcome 0, rival backs the outcome that wins.
    ctx.client
        .place_prediction(&user, &lost, &300, &0, &None, &None);
    ctx.client
        .place_prediction(&rival, &lost, &700, &1, &None, &None);

    resolve_at(&ctx, &env, won, end_time, 1);
    ctx.client.resolve_pool(&ctx.operator, &lost, &1);

    let before = ctx.token.balance(&user);
    let result = ctx
        .client
        .batch_claim_winnings(&user, &vec![&env, won, lost]);

    assert_eq!(result.len(), 2, "both pools must be reported");
    assert_eq!(
        result.get(won).unwrap(),
        500,
        "sole winner takes the whole pot at fee_bps = 0"
    );
    assert_eq!(
        result.get(lost).unwrap(),
        0,
        "a losing pool reports 0 rather than aborting the batch"
    );
    assert_eq!(
        ctx.token.balance(&user) - before,
        500,
        "only the winning pool may move funds"
    );
}

/// A pool that is still `Active` cannot be claimed, but that failure is
/// swallowed by `unwrap_or(0)` and must not strand a resolved pool sitting
/// beside it in the same batch.
#[test]
fn batch_claim_unresolved_pool_does_not_strand_the_resolved_one() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &900);

    let ready = new_pool(&ctx, &env, &user, 10_000u64);
    let still_open = new_pool(&ctx, &env, &user, 90_000u64);

    ctx.client
        .place_prediction(&user, &ready, &400, &1, &None, &None);
    ctx.client
        .place_prediction(&user, &still_open, &400, &1, &None, &None);

    // Advancing to `ready.end_time` leaves `still_open` short of its own.
    resolve_at(&ctx, &env, ready, 10_000u64, 1);
    assert_eq!(ctx.client.get_pool(&still_open).state, MarketState::Active);

    let before = ctx.token.balance(&user);
    let result = ctx
        .client
        .batch_claim_winnings(&user, &vec![&env, still_open, ready]);

    assert_eq!(result.get(ready).unwrap(), 400);
    assert_eq!(
        result.get(still_open).unwrap(),
        0,
        "PoolNotResolved is reported as 0, not raised"
    );
    assert_eq!(ctx.token.balance(&user) - before, 400);

    // The unresolved pool is untouched, so it can still be claimed later.
    resolve_at(&ctx, &env, still_open, 90_000u64, 1);
    assert_eq!(ctx.client.claim_winnings(&user, &still_open), 400);
}

/// Re-running the same batch must not pay twice. The second run reports 0
/// because `claim_winnings_internal` returns `AlreadyClaimed`.
#[test]
fn batch_claim_run_twice_pays_once() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &600);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &user, end_time);
    ctx.client
        .place_prediction(&user, &pool_id, &600, &1, &None, &None);
    resolve_at(&ctx, &env, pool_id, end_time, 1);

    let before = ctx.token.balance(&user);
    let first = ctx.client.batch_claim_winnings(&user, &vec![&env, pool_id]);
    let second = ctx.client.batch_claim_winnings(&user, &vec![&env, pool_id]);

    assert_eq!(first.get(pool_id).unwrap(), 600);
    assert_eq!(second.get(pool_id).unwrap(), 0, "second batch pays nothing");
    assert_eq!(
        ctx.token.balance(&user) - before,
        600,
        "double-claim protection must hold across separate batches"
    );
}

/// **Documents a reporting defect.**
///
/// A duplicated id is claimed once — the money is right — but the result map
/// is wrong. `batch_claim_winnings` calls `results.set(pool_id, amount)` on
/// every iteration, so the second pass (which fails with `AlreadyClaimed` and
/// is flattened to 0 by `unwrap_or`) **overwrites** the real payout recorded by
/// the first pass. The caller is told 0 was claimed while 600 actually moved.
///
/// The pre-existing test for duplicates only asserts `result.len() == 1`, which
/// this defect satisfies. Asserted here as observed behaviour so the bug is
/// pinned rather than silently carried; if the reporting is fixed, this test
/// fails loudly and should be updated to expect 600.
#[test]
fn batch_claim_duplicate_ids_pay_once_but_the_map_reports_zero() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &600);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &user, end_time);
    ctx.client
        .place_prediction(&user, &pool_id, &600, &1, &None, &None);
    resolve_at(&ctx, &env, pool_id, end_time, 1);

    let before = ctx.token.balance(&user);
    let result = ctx
        .client
        .batch_claim_winnings(&user, &vec![&env, pool_id, pool_id, pool_id]);

    assert_eq!(result.len(), 1, "a Map cannot hold a duplicated key");
    assert_eq!(
        ctx.token.balance(&user) - before,
        600,
        "funds are correct: the pool pays exactly once"
    );
    assert_eq!(
        result.get(pool_id).unwrap(),
        0,
        "known defect: the later AlreadyClaimed pass overwrites the real payout"
    );
}

/// A batch mixing many real pools with unknown ids: every id is accounted for
/// and only the winners move funds. Exercises the array-size dimension of the
/// issue against genuine pools rather than unknown ids alone.
#[test]
fn batch_claim_large_mixed_batch_reports_every_id() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let end_time = 10_000u64;
    let mut ids = vec![&env];
    for _ in 0..10u32 {
        let pool_id = new_pool(&ctx, &env, &user, end_time);
        ctx.client
            .place_prediction(&user, &pool_id, &100, &1, &None, &None);
        ids.push_back(pool_id);
    }
    // Unknown ids interleaved with the real ones.
    ids.push_back(50_000u64);
    ids.push_back(50_001u64);

    env.ledger().with_mut(|li| li.timestamp = end_time);
    for pool_id in ids.iter().take(10) {
        ctx.client.resolve_pool(&ctx.operator, &pool_id, &1);
    }

    let before = ctx.token.balance(&user);
    let result = ctx.client.batch_claim_winnings(&user, &ids);

    assert_eq!(result.len(), 12, "every requested id is reported");
    assert_eq!(result.get(50_000u64).unwrap(), 0);
    assert_eq!(result.get(50_001u64).unwrap(), 0);
    assert_eq!(
        ctx.token.balance(&user) - before,
        1_000,
        "all ten winning pools pay out"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// #1447 — set_stake_limits with predictions in flight
// ═══════════════════════════════════════════════════════════════════════════

/// Limits may be tightened while a pool is live and already carries stake.
#[test]
fn set_stake_limits_can_change_while_predictions_are_active() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let pool_id = new_pool(&ctx, &env, &user, 90_000u64);
    ctx.client
        .place_prediction(&user, &pool_id, &1_000, &1, &None, &None);

    ctx.client
        .set_stake_limits(&ctx.operator, &pool_id, &100, &0);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.min_stake, 100);
    assert_eq!(pool.state, MarketState::Active);
    assert_eq!(pool.total_stake, 1_000, "the change must not alter stake");
}

/// Check 3 of `validate_stake_limits`: a new minimum above the stake already
/// gathered is refused, because it would retroactively invalidate bets that
/// were legal when they were placed.
#[test]
fn set_stake_limits_rejects_a_minimum_above_the_accumulated_stake() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let pool_id = new_pool(&ctx, &env, &user, 90_000u64);
    ctx.client
        .place_prediction(&user, &pool_id, &1_000, &1, &None, &None);

    let res = ctx
        .client
        .try_set_stake_limits(&ctx.operator, &pool_id, &5_000, &0);
    assert_eq!(res, Err(Ok(PredifiError::StakeAboveMaximum)));

    assert_eq!(
        ctx.client.get_pool(&pool_id).min_stake,
        1,
        "a rejected update must leave the stored limits untouched"
    );
}

/// A prediction placed under the old limits stays valid and pays in full after
/// the minimum is raised well above it.
#[test]
fn a_prediction_placed_before_a_limit_raise_still_pays_in_full() {
    let env = Env::default();
    let ctx = setup(&env);

    let early = Address::generate(&env);
    ctx.token_admin.mint(&early, &1_000);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &early, end_time);
    ctx.client
        .place_prediction(&early, &pool_id, &1_000, &1, &None, &None);

    // Raise the floor to 500 — five hundred times the stake's original floor.
    ctx.client
        .set_stake_limits(&ctx.operator, &pool_id, &500, &0);

    resolve_at(&ctx, &env, pool_id, end_time, 1);

    assert_eq!(
        ctx.client.claim_winnings(&early, &pool_id),
        1_000,
        "the earlier prediction is honoured at its full value"
    );
}

/// The raised floor governs *new* predictions only.
#[test]
fn a_raised_minimum_applies_only_to_later_predictions() {
    let env = Env::default();
    let ctx = setup(&env);

    let early = Address::generate(&env);
    let late = Address::generate(&env);
    ctx.token_admin.mint(&early, &1_000);
    ctx.token_admin.mint(&late, &1_000);

    let pool_id = new_pool(&ctx, &env, &early, 90_000u64);
    ctx.client
        .place_prediction(&early, &pool_id, &1_000, &1, &None, &None);

    ctx.client
        .set_stake_limits(&ctx.operator, &pool_id, &500, &0);

    // Below the new floor — refused.
    assert!(
        ctx.client
            .try_place_prediction(&late, &pool_id, &100, &1, &None, &None)
            .is_err(),
        "a stake under the new minimum must be rejected"
    );

    // At the new floor — accepted, and the pool total reflects both bets.
    ctx.client
        .place_prediction(&late, &pool_id, &500, &1, &None, &None);
    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, 1_500);
}

/// Check 6: when a ceiling is set it must be at least ten times the floor, so
/// operators cannot squeeze the participation range down to nothing.
#[test]
fn set_stake_limits_requires_a_ceiling_of_at_least_ten_times_the_floor() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let pool_id = new_pool(&ctx, &env, &user, 90_000u64);
    ctx.client
        .place_prediction(&user, &pool_id, &1_000, &1, &None, &None);

    // 500 is only 5x the floor of 100.
    let too_narrow = ctx
        .client
        .try_set_stake_limits(&ctx.operator, &pool_id, &100, &500);
    assert_eq!(too_narrow, Err(Ok(PredifiError::InvalidAmount)));

    // Exactly 10x is allowed.
    ctx.client
        .set_stake_limits(&ctx.operator, &pool_id, &100, &1_000);
    assert_eq!(ctx.client.get_pool(&pool_id).max_stake, 1_000);
}

/// Once a pool has resolved its limits are frozen.
#[test]
fn set_stake_limits_rejects_a_resolved_pool() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &user, end_time);
    ctx.client
        .place_prediction(&user, &pool_id, &1_000, &1, &None, &None);
    resolve_at(&ctx, &env, pool_id, end_time, 1);

    let res = ctx
        .client
        .try_set_stake_limits(&ctx.operator, &pool_id, &10, &0);
    assert_eq!(res, Err(Ok(PredifiError::InvalidPoolState)));
}

// ═══════════════════════════════════════════════════════════════════════════
// #1448 — add_token_to_whitelist
// ═══════════════════════════════════════════════════════════════════════════

/// Re-adding a token must not append a second entry to the enumeration list.
/// `is_token_allowed` cannot see this — it only reads the per-token flag — so
/// the list is inspected directly.
#[test]
fn adding_a_duplicate_token_does_not_grow_the_whitelist_list() {
    let env = Env::default();
    let ctx = setup(&env);

    let token = Address::generate(&env);
    ctx.client.add_token_to_whitelist(&ctx.admin, &token);
    let after_first = whitelist_list(&env, &ctx).len();

    ctx.client.add_token_to_whitelist(&ctx.admin, &token);
    ctx.client.add_token_to_whitelist(&ctx.admin, &token);

    assert!(ctx.client.is_token_allowed(&token));
    assert_eq!(
        whitelist_list(&env, &ctx).len(),
        after_first,
        "repeat admissions must be idempotent at the list level too"
    );
}

/// There is no cap on the whitelist; admitting many tokens keeps every one of
/// them enumerable. Documents the absence of a limit so that introducing one
/// later is a deliberate change.
#[test]
fn many_tokens_can_be_whitelisted() {
    let env = Env::default();
    let ctx = setup(&env);

    let mut added = vec![&env];
    for _ in 0..64u32 {
        let token = Address::generate(&env);
        ctx.client.add_token_to_whitelist(&ctx.admin, &token);
        added.push_back(token);
    }

    for token in added.iter() {
        assert!(ctx.client.is_token_allowed(&token));
    }
    // 64 new tokens plus the one `setup` admitted.
    assert_eq!(whitelist_list(&env, &ctx).len(), 65);
}

/// **Documents missing validation.** `add_token_to_whitelist` never probes the
/// address, so a plain account address — or an address with no contract behind
/// it at all — is admitted exactly like a real token. The mistake only
/// surfaces later, when `place_prediction` tries to transfer and the pool is
/// already live.
#[test]
fn a_non_contract_address_is_whitelisted_without_complaint() {
    let env = Env::default();
    let ctx = setup(&env);

    let not_a_token = Address::generate(&env);
    ctx.client.add_token_to_whitelist(&ctx.admin, &not_a_token);

    assert!(
        ctx.client.is_token_allowed(&not_a_token),
        "no contract-existence check is performed"
    );

    // A pool can even be created against it, because create_pool only checks
    // the whitelist flag and never calls the token.
    let creator = Address::generate(&env);
    let pool_id = ctx.client.create_pool(
        &creator,
        &90_000u64,
        &not_a_token,
        &2u32,
        &symbol_short!("Sports"),
        &config(&env, 1i128, 0i128),
    );
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Active);
}

/// The whitelist stores an address and nothing else, so a token reporting 18
/// decimals is admitted on the same terms as a 7-decimal Stellar asset.
#[test]
fn token_decimals_do_not_affect_whitelisting() {
    let env = Env::default();
    let ctx = setup(&env);

    let odd = env.register(odd_decimals_token::OddDecimalsToken, ());
    let odd_client = odd_decimals_token::OddDecimalsTokenClient::new(&env, &odd);
    assert_eq!(odd_client.decimals(), 18, "precondition: non-standard");

    ctx.client.add_token_to_whitelist(&ctx.admin, &odd);
    assert!(ctx.client.is_token_allowed(&odd));

    let creator = Address::generate(&env);
    let pool_id = ctx.client.create_pool(
        &creator,
        &90_000u64,
        &odd,
        &2u32,
        &symbol_short!("Sports"),
        &config(&env, 1i128, 0i128),
    );
    assert_eq!(ctx.client.get_pool(&pool_id).token, odd);
}

/// A token admitted after deployment immediately backs new pools.
#[test]
fn a_newly_whitelisted_token_can_back_a_pool() {
    let env = Env::default();
    let ctx = setup(&env);

    let deployer = Address::generate(&env);
    let second = env.register_stellar_asset_contract_v2(deployer);
    let second_address = second.address();
    let creator = Address::generate(&env);

    // Before admission the pool is refused …
    let refused = ctx.client.try_create_pool(
        &creator,
        &90_000u64,
        &second_address,
        &2u32,
        &symbol_short!("Sports"),
        &config(&env, 1i128, 0i128),
    );
    assert_eq!(refused, Err(Ok(PredifiError::TokenNotWhitelisted)));

    // … and accepted immediately afterwards.
    ctx.client
        .add_token_to_whitelist(&ctx.admin, &second_address);
    let pool_id = ctx.client.create_pool(
        &creator,
        &90_000u64,
        &second_address,
        &2u32,
        &symbol_short!("Sports"),
        &config(&env, 1i128, 0i128),
    );

    // And it is usable end to end, not merely creatable.
    let staker = Address::generate(&env);
    token::StellarAssetClient::new(&env, &second_address).mint(&staker, &400);
    ctx.client
        .place_prediction(&staker, &pool_id, &400, &1, &None, &None);
    assert_eq!(ctx.client.get_pool(&pool_id).total_stake, 400);
}

// ═══════════════════════════════════════════════════════════════════════════
// #1449 — remove_token_from_whitelist
// ═══════════════════════════════════════════════════════════════════════════

/// Delisting a token that backs a live pool succeeds and leaves the pool's own
/// record alone — the pool is not cancelled, resolved, or otherwise disturbed.
#[test]
fn delisting_a_token_leaves_an_existing_pool_untouched() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &500);

    let pool_id = new_pool(&ctx, &env, &user, 90_000u64);
    ctx.client
        .place_prediction(&user, &pool_id, &500, &1, &None, &None);

    ctx.client
        .remove_token_from_whitelist(&ctx.admin, &ctx.token_address);

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Active, "the pool is not cancelled");
    assert_eq!(pool.total_stake, 500, "staked funds are still recorded");
    assert!(!ctx.client.is_token_allowed(&ctx.token_address));
}

/// New pools against a delisted token are refused.
#[test]
fn delisting_blocks_new_pools_for_that_token() {
    let env = Env::default();
    let ctx = setup(&env);

    ctx.client
        .remove_token_from_whitelist(&ctx.admin, &ctx.token_address);

    let creator = Address::generate(&env);
    let res = ctx.client.try_create_pool(
        &creator,
        &90_000u64,
        &ctx.token_address,
        &2u32,
        &symbol_short!("Sports"),
        &config(&env, 1i128, 0i128),
    );
    assert_eq!(res, Err(Ok(PredifiError::TokenNotWhitelisted)));
}

/// `place_prediction` re-checks the whitelist on every call, so delisting also
/// freezes *new* stake into pools that already exist.
#[test]
fn delisting_blocks_further_predictions_into_an_existing_pool() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000);

    let pool_id = new_pool(&ctx, &env, &user, 90_000u64);
    ctx.client
        .place_prediction(&user, &pool_id, &500, &1, &None, &None);

    ctx.client
        .remove_token_from_whitelist(&ctx.admin, &ctx.token_address);

    let res = ctx
        .client
        .try_place_prediction(&user, &pool_id, &500, &1, &None, &None);
    assert_eq!(res, Err(Ok(PredifiError::TokenNotWhitelisted)));
    assert_eq!(
        ctx.client.get_pool(&pool_id).total_stake,
        500,
        "the blocked stake must not be recorded"
    );
}

/// The important half of "existing pools still function": claiming does **not**
/// consult the whitelist, so delisting cannot strand funds already staked.
#[test]
fn delisting_does_not_strand_funds_winners_can_still_claim() {
    let env = Env::default();
    let ctx = setup(&env);

    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &600);

    let end_time = 10_000u64;
    let pool_id = new_pool(&ctx, &env, &user, end_time);
    ctx.client
        .place_prediction(&user, &pool_id, &600, &1, &None, &None);

    ctx.client
        .remove_token_from_whitelist(&ctx.admin, &ctx.token_address);
    resolve_at(&ctx, &env, pool_id, end_time, 1);

    let before = ctx.token.balance(&user);
    assert_eq!(
        ctx.client.claim_winnings(&user, &pool_id),
        600,
        "a delisted token must not block payouts"
    );
    assert_eq!(ctx.token.balance(&user) - before, 600);
    assert_eq!(ctx.token.balance(&ctx.client.address), 0);
}

/// Removing the only whitelisted token empties the enumeration list rather
/// than leaving a stale entry behind.
#[test]
fn removing_the_last_token_empties_the_whitelist() {
    let env = Env::default();
    let ctx = setup(&env);

    assert_eq!(whitelist_list(&env, &ctx).len(), 1, "precondition");

    ctx.client
        .remove_token_from_whitelist(&ctx.admin, &ctx.token_address);

    assert_eq!(whitelist_list(&env, &ctx).len(), 0);
    assert!(!ctx.client.is_token_allowed(&ctx.token_address));
}

/// Removing a token that was never admitted is a no-op, not an error, and must
/// not disturb the tokens that *are* whitelisted.
#[test]
fn removing_an_unlisted_token_is_a_noop() {
    let env = Env::default();
    let ctx = setup(&env);

    let stranger = Address::generate(&env);
    assert!(!ctx.client.is_token_allowed(&stranger));

    ctx.client
        .remove_token_from_whitelist(&ctx.admin, &stranger);

    assert!(!ctx.client.is_token_allowed(&stranger));
    assert_eq!(
        whitelist_list(&env, &ctx).len(),
        1,
        "the existing entry survives an unrelated removal"
    );
    assert!(ctx.client.is_token_allowed(&ctx.token_address));
}

/// Soroban executes one transaction at a time, so "concurrent" removals are
/// modelled as back-to-back calls inside a single ledger. Each removal must
/// take out exactly its own token.
#[test]
fn back_to_back_removals_take_out_exactly_their_own_tokens() {
    let env = Env::default();
    let ctx = setup(&env);

    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let keep = Address::generate(&env);
    for token in [&a, &b, &keep] {
        ctx.client.add_token_to_whitelist(&ctx.admin, token);
    }
    assert_eq!(whitelist_list(&env, &ctx).len(), 4);

    // Same ledger timestamp for all three calls.
    ctx.client.remove_token_from_whitelist(&ctx.admin, &a);
    ctx.client.remove_token_from_whitelist(&ctx.admin, &b);
    ctx.client.remove_token_from_whitelist(&ctx.admin, &a); // repeat: no-op

    assert!(!ctx.client.is_token_allowed(&a));
    assert!(!ctx.client.is_token_allowed(&b));
    assert!(ctx.client.is_token_allowed(&keep));
    assert!(ctx.client.is_token_allowed(&ctx.token_address));
    assert_eq!(
        whitelist_list(&env, &ctx).len(),
        2,
        "only the two removed tokens leave the list"
    );
}

/// A delisted token can be re-admitted and immediately backs pools again.
#[test]
fn a_removed_token_can_be_readded_and_used_again() {
    let env = Env::default();
    let ctx = setup(&env);

    ctx.client
        .remove_token_from_whitelist(&ctx.admin, &ctx.token_address);
    ctx.client
        .add_token_to_whitelist(&ctx.admin, &ctx.token_address);

    assert!(ctx.client.is_token_allowed(&ctx.token_address));
    assert_eq!(
        whitelist_list(&env, &ctx).len(),
        1,
        "re-adding must not leave a duplicate entry"
    );

    let creator = Address::generate(&env);
    let pool_id = new_pool(&ctx, &env, &creator, 90_000u64);
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Active);
}
