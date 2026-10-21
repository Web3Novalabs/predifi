//! Shared integration-test harness for the `predifi-contract` crate.
//!
//! Every integration suite in `tests/` builds its fixture through [`setup`],
//! which wires up:
//!
//! * an in-memory `DummyAccessControl` contract (role checks used by the
//!   contract under test),
//! * a Soroban Stellar Asset Contract used as the betting token, already
//!   whitelisted in the contract,
//! * a fully initialised [`PredifiContractClient`].
//!
//! Event assertions rely on the [`event_data`] / [`event_count`] helpers, which
//! implement the canonical "match on `topics[0]`, decode the `Map<Symbol, Val>`
//! payload" pattern used throughout the repo.

#![allow(dead_code)]

use predifi_contract::{PredifiContract, PredifiContractClient};
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events as _, Ledger},
    token, Address, Env, IntoVal, Map, Symbol, TryFromVal, Val,
};

/// Role index for the Admin role in the access-control contract.
pub const ROLE_ADMIN: u32 = 0;
/// Role index for the Operator role in the access-control contract.
pub const ROLE_OPERATOR: u32 = 1;
/// Role index for the Oracle role in the access-control contract.
pub const ROLE_ORACLE: u32 = 3;

/// Default ledger timestamp used by [`setup`].
pub const START_TIME: u64 = 1_000;

#[contract]
pub struct DummyAccessControl;

#[contractimpl]
impl DummyAccessControl {
    pub fn grant_role(env: Env, user: Address, role: u32) {
        let key = (Symbol::new(&env, "role"), user.clone(), role);
        let already: bool = env.storage().instance().get(&key).unwrap_or(false);
        env.storage().instance().set(&key, &true);
        if role == ROLE_OPERATOR && !already {
            let ck = Symbol::new(&env, "op_count");
            let count: u32 = env.storage().instance().get(&ck).unwrap_or(0);
            env.storage().instance().set(&ck, &(count + 1));
        }
    }

    pub fn revoke_role(env: Env, user: Address, role: u32) {
        let key = (Symbol::new(&env, "role"), user, role);
        let had: bool = env.storage().instance().get(&key).unwrap_or(false);
        env.storage().instance().set(&key, &false);
        if role == ROLE_OPERATOR && had {
            let ck = Symbol::new(&env, "op_count");
            let count: u32 = env.storage().instance().get(&ck).unwrap_or(0);
            if count > 0 {
                env.storage().instance().set(&ck, &(count - 1));
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

/// A fully-wired test fixture.
pub struct Harness<'a> {
    pub env: &'a Env,
    pub client: PredifiContractClient<'a>,
    pub ac: DummyAccessControlClient<'a>,
    pub contract_id: Address,
    pub token_address: Address,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
    pub admin: Address,
    pub operator: Address,
    pub oracle: Address,
    pub creator: Address,
    pub treasury: Address,
}

impl<'a> Harness<'a> {
    /// Mint `amount` of the betting token to `to`.
    pub fn mint(&self, to: &Address, amount: i128) {
        self.token_admin.mint(to, &amount);
    }

    /// Advance the ledger timestamp to `timestamp`.
    pub fn set_time(&self, timestamp: u64) {
        self.env.ledger().with_mut(|li| li.timestamp = timestamp);
    }
}

/// Build an [`Env`] configured for contract tests.
pub fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.protocol_version = 23;
        li.timestamp = START_TIME;
    });
    env
}

/// Register the access-control stub, the contract under test and a whitelisted
/// betting token, then return the assembled [`Harness`].
///
/// Equivalent to [`setup_with_fee`] with `fee_bps = 0`.
pub fn setup(env: &Env) -> Harness<'_> {
    setup_with_fee(env, 0)
}

/// As [`setup`], but initialises the protocol with an explicit `fee_bps`.
///
/// `fee_bps` must satisfy `0 <= fee_bps <= 10_000` (validated by `init`).
pub fn setup_with_fee(env: &Env, fee_bps: u32) -> Harness<'_> {
    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let oracle = Address::generate(env);
    let creator = Address::generate(env);
    let treasury = Address::generate(env);

    let ac_id = env.register(DummyAccessControl, ());
    let ac = DummyAccessControlClient::new(env, &ac_id);
    ac.grant_role(&admin, &ROLE_ADMIN);
    ac.grant_role(&operator, &ROLE_OPERATOR);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);

    // resolution_delay = 0 lets pools be resolved as soon as `end_time` has
    // elapsed; `min_pool_duration = 0` removes any lower bound on pool length.
    client.init(&ac_id, &treasury, &fee_bps, &0u64, &0u64, &0u32);

    let token_admin_addr = Address::generate(env);
    let token_address = env.register_stellar_asset_contract(token_admin_addr);
    let token = token::Client::new(env, &token_address);
    let token_admin = token::StellarAssetClient::new(env, &token_address);
    client.add_token_to_whitelist(&admin, &token_address);

    Harness {
        env,
        client,
        ac,
        contract_id,
        token_address,
        token,
        token_admin,
        admin,
        operator,
        oracle,
        creator,
        treasury,
    }
}

// ── Event inspection helpers ────────────────────────────────────────────────
//
// IMPORTANT: `Env::events().all()` only reflects the events published by the
// **most recent top-level contract invocation**. Any subsequent client call
// (including read-only getters) clears the buffer. Always call these helpers
// immediately after the entry point whose events you want to assert, before
// making any other contract call.

/// Decode the `Map<Symbol, Val>` payload of the **last** event published under
/// `topic`, or `None` if no such event was emitted.
///
/// `topic` is the value stored in `topics[0]`, i.e. the string passed to
/// `#[contractevent(topics = ["..."])]`.
pub fn event_data(env: &Env, topic: &str) -> Option<Map<Symbol, Val>> {
    let want = Symbol::new(env, topic);
    let mut found = None;
    for e in env.events().all().iter() {
        if let Some(topic_val) = e.1.get(0) {
            if let Ok(sym) = Symbol::try_from_val(env, &topic_val) {
                if sym == want {
                    found = Some(e.2.clone().into_val(env));
                }
            }
        }
    }
    found
}

/// Number of events published under `topic` since the ledger started.
pub fn event_count(env: &Env, topic: &str) -> u32 {
    let want = Symbol::new(env, topic);
    let mut count = 0u32;
    for e in env.events().all().iter() {
        if let Some(topic_val) = e.1.get(0) {
            if let Ok(sym) = Symbol::try_from_val(env, &topic_val) {
                if sym == want {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Read a named field out of an event payload, panicking with the field name
/// when the payload is missing it.
pub fn field(env: &Env, data: &Map<Symbol, Val>, name: &str) -> Val {
    data.get(Symbol::new(env, name))
        .unwrap_or_else(|| panic!("event payload missing field `{}`", name))
}
