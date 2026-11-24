//! Boundary & Edge Case Tests for Issue #1523 — `withdraw_treasury`
//!
//! Coverage:
//! - Withdrawal of more than treasury balance fails with `InsufficientBalance`
//! - Withdrawal of exactly zero fails with `InvalidAmount`
//! - Withdrawal by a non-admin fails with `Unauthorized`
//! - Withdrawal of a non-whitelisted token (admin can withdraw any token)
//! - Treasury balance consistency after multiple withdrawals

#![cfg(test)]

extern crate std;

use crate::{PredifiContract, PredifiContractClient, PredifiError};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

// ─── Shared dummy access-control stub ────────────────────────────────────────

mod ac_stub_1523 {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct AcStub1523;

    #[contractimpl]
    impl AcStub1523 {
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

struct TreasuryTestEnv<'a> {
    pub env: Env,
    pub client: PredifiContractClient<'a>,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
    pub token_address: Address,
    pub admin: Address,
    pub operator: Address,
    pub treasury: Address,
}

impl<'a> TreasuryTestEnv<'a> {
    fn new(env: &'a Env) -> Self {
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.protocol_version = 23;
            li.timestamp = 1_000;
        });

        let admin = Address::generate(env);
        let operator = Address::generate(env);
        let treasury = Address::generate(env);

        let ac_id = env.register(ac_stub_1523::AcStub1523, ());
        let ac = ac_stub_1523::AcStub1523Client::new(env, &ac_id);
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
            treasury,
        }
    }

    /// Mint `amount` tokens directly to the predifi contract (simulating accumulated fees).
    fn fund_contract(&self, amount: i128) {
        let contract_id = self.client.address.clone();
        self.token_admin.mint(&contract_id, &amount);
    }

    fn contract_balance(&self) -> i128 {
        self.token.balance(&self.client.address)
    }

    fn treasury_balance(&self) -> i128 {
        self.token.balance(&self.treasury)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// #1523 — withdraw_treasury Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Attempting to withdraw more than the contract holds must fail with
/// `InsufficientBalance`. The contract balance must remain unchanged.
#[test]
fn test_1523_withdraw_more_than_balance_fails_with_insufficient_balance() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    ctx.fund_contract(1_000);

    let result = ctx.client.try_withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &1_001i128, // one more than available
        &ctx.treasury,
    );

    assert_eq!(
        result,
        Err(Ok(PredifiError::InsufficientBalance)),
        "withdrawing more than the contract balance must return InsufficientBalance"
    );

    // Balances must be completely unchanged.
    assert_eq!(
        ctx.contract_balance(),
        1_000,
        "contract balance must remain 1_000 after failed over-withdrawal"
    );
    assert_eq!(
        ctx.treasury_balance(),
        0,
        "treasury balance must remain 0 after failed withdrawal"
    );
}

/// Attempting to withdraw a very large amount from an empty contract must also
/// fail with `InsufficientBalance` (special-case: zero balance edge).
#[test]
fn test_1523_withdraw_from_empty_contract_fails() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    // Contract has zero balance — nothing was minted.
    assert_eq!(ctx.contract_balance(), 0);

    let result = ctx.client.try_withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &1i128,
        &ctx.treasury,
    );

    assert_eq!(
        result,
        Err(Ok(PredifiError::InsufficientBalance)),
        "any withdrawal from an empty contract must fail with InsufficientBalance"
    );
}

/// Attempting to withdraw exactly the contract balance must succeed and leave
/// the contract with zero balance.
#[test]
fn test_1523_withdraw_exact_balance_succeeds() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    let exact = 750i128;
    ctx.fund_contract(exact);

    ctx.client
        .withdraw_treasury(&ctx.admin, &ctx.token_address, &exact, &ctx.treasury);

    assert_eq!(ctx.contract_balance(), 0, "contract must be drained to zero");
    assert_eq!(
        ctx.treasury_balance(),
        exact,
        "treasury must receive the full amount"
    );
}

/// Withdrawal of exactly zero must fail with `InvalidAmount`.
#[test]
fn test_1523_withdraw_exactly_zero_fails_with_invalid_amount() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    ctx.fund_contract(500);

    let result = ctx.client.try_withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &0i128,
        &ctx.treasury,
    );

    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidAmount)),
        "withdrawing 0 must return InvalidAmount"
    );

    // Balances unchanged.
    assert_eq!(ctx.contract_balance(), 500);
    assert_eq!(ctx.treasury_balance(), 0);
}

/// Withdrawal of a negative amount must fail with `InvalidAmount`.
#[test]
fn test_1523_withdraw_negative_amount_fails_with_invalid_amount() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    ctx.fund_contract(500);

    let result = ctx.client.try_withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &-100i128,
        &ctx.treasury,
    );

    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidAmount)),
        "withdrawing a negative amount must return InvalidAmount"
    );

    assert_eq!(ctx.contract_balance(), 500);
    assert_eq!(ctx.treasury_balance(), 0);
}

/// A non-admin caller must be rejected with `Unauthorized`.
/// The contract balance and treasury balance must not change.
#[test]
fn test_1523_non_admin_withdrawal_is_rejected() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    ctx.fund_contract(1_000);

    let stranger = Address::generate(&env);
    let result = ctx.client.try_withdraw_treasury(
        &stranger,
        &ctx.token_address,
        &500i128,
        &ctx.treasury,
    );

    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "non-admin must be rejected with Unauthorized"
    );

    assert_eq!(ctx.contract_balance(), 1_000);
    assert_eq!(ctx.treasury_balance(), 0);
}

/// The operator role (role 1) is NOT the admin role (role 0).
/// An operator attempting withdrawal must be rejected with `Unauthorized`.
#[test]
fn test_1523_operator_role_cannot_withdraw_treasury() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    ctx.fund_contract(1_000);

    let result = ctx.client.try_withdraw_treasury(
        &ctx.operator,
        &ctx.token_address,
        &500i128,
        &ctx.treasury,
    );

    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "operator (role 1) must not be authorized to withdraw treasury"
    );

    assert_eq!(ctx.contract_balance(), 1_000);
}

/// Admin CAN withdraw a non-whitelisted token. The whitelist applies to
/// prediction staking, not to treasury recovery — admins must be able to
/// rescue any token accidentally sent to the contract.
#[test]
fn test_1523_admin_can_withdraw_non_whitelisted_token() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    // Register a second token that is NOT added to the prediction whitelist.
    let non_wl_admin = Address::generate(&env);
    let non_wl_contract = env.register_stellar_asset_contract_v2(non_wl_admin);
    let non_wl_address = non_wl_contract.address();
    let non_wl_token = token::Client::new(&env, &non_wl_address);
    let non_wl_token_admin = token::StellarAssetClient::new(&env, &non_wl_address);

    // Fund the predifi contract with non-whitelisted tokens.
    let contract_id = ctx.client.address.clone();
    non_wl_token_admin.mint(&contract_id, &2_000i128);
    assert!(!ctx.client.is_token_allowed(&non_wl_address));

    // Admin withdrawal must succeed.
    ctx.client.withdraw_treasury(
        &ctx.admin,
        &non_wl_address,
        &2_000i128,
        &ctx.treasury,
    );

    assert_eq!(
        non_wl_token.balance(&ctx.treasury),
        2_000,
        "admin must receive non-whitelisted tokens on withdrawal"
    );
    assert_eq!(
        non_wl_token.balance(&contract_id),
        0,
        "contract balance must be zero after full non-whitelisted withdrawal"
    );
}

/// Treasury balance consistency after multiple sequential withdrawals.
/// Each withdrawal must reduce contract balance and increase treasury balance
/// by exactly the withdrawn amount, with no rounding or off-by-one errors.
#[test]
fn test_1523_treasury_balance_consistency_after_multiple_withdrawals() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    let initial = 3_000i128;
    ctx.fund_contract(initial);

    // 1st withdrawal: 500
    ctx.client
        .withdraw_treasury(&ctx.admin, &ctx.token_address, &500i128, &ctx.treasury);
    assert_eq!(ctx.contract_balance(), 2_500, "after 1st withdrawal: contract");
    assert_eq!(ctx.treasury_balance(), 500, "after 1st withdrawal: treasury");

    // 2nd withdrawal: 1_000
    ctx.client
        .withdraw_treasury(&ctx.admin, &ctx.token_address, &1_000i128, &ctx.treasury);
    assert_eq!(ctx.contract_balance(), 1_500, "after 2nd withdrawal: contract");
    assert_eq!(ctx.treasury_balance(), 1_500, "after 2nd withdrawal: treasury");

    // 3rd withdrawal: 1_499 (one less than remaining)
    ctx.client
        .withdraw_treasury(&ctx.admin, &ctx.token_address, &1_499i128, &ctx.treasury);
    assert_eq!(ctx.contract_balance(), 1, "after 3rd withdrawal: contract");
    assert_eq!(ctx.treasury_balance(), 2_999, "after 3rd withdrawal: treasury");

    // 4th withdrawal: exactly the last 1 token
    ctx.client
        .withdraw_treasury(&ctx.admin, &ctx.token_address, &1i128, &ctx.treasury);
    assert_eq!(ctx.contract_balance(), 0, "contract must be empty after 4th");
    assert_eq!(ctx.treasury_balance(), initial, "treasury must equal initial amount");

    // 5th withdrawal attempt on empty contract must fail.
    let result = ctx.client.try_withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &1i128,
        &ctx.treasury,
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InsufficientBalance)),
        "5th withdrawal on empty contract must fail with InsufficientBalance"
    );
    // Balances unchanged.
    assert_eq!(ctx.contract_balance(), 0);
    assert_eq!(ctx.treasury_balance(), initial);
}

/// The sum of all amounts withdrawn must never exceed the initial contract
/// balance — total conservation of value.
#[test]
fn test_1523_total_withdrawn_never_exceeds_initial_balance() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    let initial = 5_000i128;
    ctx.fund_contract(initial);

    let withdrawals: &[i128] = &[100, 200, 400, 800, 1_600, 900];
    let total: i128 = withdrawals.iter().sum();
    assert!(total <= initial, "test precondition: total withdrawals must not exceed initial");

    for &amount in withdrawals {
        ctx.client
            .withdraw_treasury(&ctx.admin, &ctx.token_address, &amount, &ctx.treasury);
    }

    assert_eq!(ctx.treasury_balance(), total);
    assert_eq!(ctx.contract_balance(), initial - total);
    assert_eq!(
        ctx.contract_balance() + ctx.treasury_balance(),
        initial,
        "value conservation: contract + treasury must equal initial balance"
    );
}

/// Withdrawals to a custom recipient (not the default treasury) must correctly
/// route funds to the specified recipient.
#[test]
fn test_1523_withdrawal_to_custom_recipient_is_routed_correctly() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    ctx.fund_contract(1_000);

    let recipient = Address::generate(&env);
    ctx.client.withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &600i128,
        &recipient,
    );

    assert_eq!(
        ctx.token.balance(&recipient),
        600,
        "custom recipient must receive the withdrawn amount"
    );
    assert_eq!(
        ctx.treasury_balance(),
        0,
        "default treasury must not receive funds when a custom recipient is used"
    );
    assert_eq!(ctx.contract_balance(), 400);
}

/// Withdrawal of i128::MAX that is far beyond any real balance must fail with
/// `InsufficientBalance` — the comparison must not overflow.
#[test]
fn test_1523_withdraw_i128_max_against_small_balance_fails() {
    let env = Env::default();
    let ctx = TreasuryTestEnv::new(&env);

    ctx.fund_contract(100);

    let result = ctx.client.try_withdraw_treasury(
        &ctx.admin,
        &ctx.token_address,
        &i128::MAX,
        &ctx.treasury,
    );

    assert_eq!(
        result,
        Err(Ok(PredifiError::InsufficientBalance)),
        "i128::MAX withdrawal against a small balance must return InsufficientBalance"
    );

    assert_eq!(ctx.contract_balance(), 100);
    assert_eq!(ctx.treasury_balance(), 0);
}
