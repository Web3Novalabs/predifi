//! Boundary & Edge Case Tests for Issue #1526 — `update_pool_description`
//!
//! Coverage:
//! - Update with empty string must be rejected
//! - Update at exactly 256 bytes (maximum length boundary) must succeed
//! - Update with 257+ bytes must be rejected
//! - Update with special characters and unicode must work within byte limits
//! - Updating a resolved pool's description must be rejected
//! - Non-creator, non-admin update must be rejected (authorization check)

#![cfg(test)]

extern crate std;

use crate::{MarketState, PoolConfig, PredifiContract, PredifiContractClient, PredifiError};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

// ─── Shared dummy access-control stub ────────────────────────────────────────

mod ac_stub_1526 {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct AcStub1526;

    #[contractimpl]
    impl AcStub1526 {
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

struct DescTestEnv<'a> {
    pub env: Env,
    pub client: PredifiContractClient<'a>,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
    pub token_address: Address,
    pub admin: Address,
    pub operator: Address,
    pub creator: Address,
}

impl<'a> DescTestEnv<'a> {
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

        let ac_id = env.register(ac_stub_1526::AcStub1526, ());
        let ac = ac_stub_1526::AcStub1526Client::new(env, &ac_id);
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
                description: String::from_str(&self.env, "Initial description"),
                metadata_url: String::from_str(&self.env, "ipfs://initial"),
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
}

// ═══════════════════════════════════════════════════════════════════════════
// #1526 — update_pool_description Boundary & Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Updating with an empty string must be rejected with `InvalidAmount`.
/// An empty description provides no value and the pool creation contract
/// already rejects empty descriptions.
#[test]
fn test_1526_empty_description_is_rejected() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    let result = ctx.client.try_update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, ""),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidAmount)),
        "empty description must be rejected with InvalidAmount"
    );

    // The description must remain unchanged.
    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool.description,
        String::from_str(&env, "Initial description"),
        "description must be unchanged after empty-string rejection"
    );
}

/// Exactly 256 bytes is the maximum accepted length. A description at this
/// boundary must be accepted and persisted.
#[test]
fn test_1526_description_exactly_256_bytes_is_accepted() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    let at_limit = core::str::from_utf8(&[b'a'; 256]).unwrap();
    ctx.client
        .update_pool_description(&ctx.creator, &pool_id, &String::from_str(&env, at_limit));

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool.description,
        String::from_str(&env, at_limit),
        "a 256-byte description must be persisted"
    );
}

/// A description of exactly 257 bytes must be rejected with `InvalidAmount`.
/// This is one byte over the 256-byte ceiling.
#[test]
fn test_1526_description_257_bytes_is_rejected() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    let over_limit = core::str::from_utf8(&[b'a'; 257]).unwrap();
    let result = ctx.client.try_update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, over_limit),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidAmount)),
        "a 257-byte description must be rejected"
    );
}

/// A description well over the limit (e.g. 1024 bytes) must also be rejected.
#[test]
fn test_1526_description_very_long_string_is_rejected() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    let way_over = core::str::from_utf8(&[b'x'; 1024]).unwrap();
    let result = ctx.client.try_update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, way_over),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidAmount)),
        "a 1024-byte description must be rejected"
    );
}

/// Basic ASCII special characters must be accepted within the byte limit.
#[test]
fn test_1526_special_characters_ascii_are_accepted() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    let special = "Will BTC/USD exceed $100,000? [Yes/No] — 99% confidence! @oracle #crypto";
    ctx.client
        .update_pool_description(&ctx.creator, &pool_id, &String::from_str(&env, special));

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool.description,
        String::from_str(&env, special),
        "ASCII special characters must be preserved"
    );
}

/// Unicode characters (multi-byte) are counted by byte length, not character
/// count. A short unicode string well within 256 bytes must be accepted.
#[test]
fn test_1526_unicode_within_byte_limit_is_accepted() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    // This string uses multi-byte characters but stays under 256 bytes total.
    let unicode_desc = "Prédiction marché 日本語 🎯 — résultat attendu";
    assert!(
        unicode_desc.len() < 256,
        "test precondition: unicode string must be < 256 bytes"
    );

    ctx.client.update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, unicode_desc),
    );

    let pool = ctx.client.get_pool(&pool_id);
    assert_eq!(
        pool.description,
        String::from_str(&env, unicode_desc),
        "unicode description within byte limit must be persisted"
    );
}

/// 100 four-byte emoji exceed 256 bytes even though the character count (100)
/// is well within range. The byte-based limit must reject this input.
#[test]
fn test_1526_unicode_exceeding_byte_limit_is_rejected() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    // 100 × 🎯 (4 bytes each) = 400 bytes > 256.
    let mut many_emoji = std::string::String::new();
    for _ in 0..100 {
        many_emoji.push('🎯');
    }
    assert!(
        many_emoji.len() > 256,
        "test precondition: emoji string must exceed 256 bytes"
    );

    let result = ctx.client.try_update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, many_emoji.as_str()),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidAmount)),
        "byte-length exceeding 256 must be rejected even for unicode"
    );
}

/// Updating a resolved pool's description must fail with `InvalidPoolState`.
/// Once a pool is resolved its terms are locked in history.
#[test]
fn test_1526_updating_resolved_pool_description_is_rejected() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);

    let pool_id = ctx.create_pool(2_000);

    // Stake so the pool is resolvable, then resolve it.
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000i128);
    ctx.client
        .place_prediction(&user, &pool_id, &1_000i128, &0u32, &None, &None);

    ctx.advance_time(2_001);
    ctx.client.resolve_pool(&ctx.operator, &pool_id, &0u32);
    assert_eq!(
        ctx.client.get_pool(&pool_id).state,
        MarketState::Resolved,
        "pool must be Resolved before testing description update"
    );

    let result = ctx.client.try_update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "Post-resolution edit attempt"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidPoolState)),
        "updating a resolved pool's description must return InvalidPoolState"
    );
}

/// Updating a cancelled pool's description must fail with `InvalidPoolState`.
#[test]
fn test_1526_updating_cancelled_pool_description_is_rejected() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);

    let pool_id = ctx.create_pool(7_200);
    ctx.client.cancel_pool(
        &ctx.operator,
        &pool_id,
        &String::from_str(&env, "test cancel"),
    );
    assert_eq!(ctx.client.get_pool(&pool_id).state, MarketState::Canceled);

    let result = ctx.client.try_update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "Trying to edit after cancel"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidPoolState)),
        "updating a cancelled pool's description must return InvalidPoolState"
    );
}

/// A non-creator, non-admin caller must be rejected with `Unauthorized`.
#[test]
fn test_1526_non_creator_non_admin_update_is_rejected() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    let stranger = Address::generate(&env);
    let result = ctx.client.try_update_pool_description(
        &stranger,
        &pool_id,
        &String::from_str(&env, "Unauthorized edit"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "non-creator non-admin must be rejected with Unauthorized"
    );

    // Description must remain unchanged.
    assert_eq!(
        ctx.client.get_pool(&pool_id).description,
        String::from_str(&env, "Initial description"),
        "description must not change after unauthorized update attempt"
    );
}

/// The operator role (role 1) is not the creator and not an admin (role 0).
/// An operator calling `update_pool_description` must be rejected.
#[test]
fn test_1526_operator_non_admin_cannot_update_description() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    // operator has role 1 but not role 0 (admin) and is not the creator.
    let result = ctx.client.try_update_pool_description(
        &ctx.operator,
        &pool_id,
        &String::from_str(&env, "Operator sneaky edit"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::Unauthorized)),
        "operator without admin role must be rejected with Unauthorized"
    );
}

/// An admin (role 0) who is not the creator must be allowed to update the
/// description — admins hold override authority for protocol governance.
#[test]
fn test_1526_admin_non_creator_can_update_description() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    let new_desc = "Admin governance update";
    ctx.client.update_pool_description(
        &ctx.admin,
        &pool_id,
        &String::from_str(&env, new_desc),
    );

    assert_eq!(
        ctx.client.get_pool(&pool_id).description,
        String::from_str(&env, new_desc),
        "admin must be able to update description even as non-creator"
    );
}

/// Once any participant has staked into the pool, the description must be
/// locked — the pool's terms must not change mid-flight for existing stakers.
#[test]
fn test_1526_description_locked_after_participant_joins() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    // Participant joins.
    let user = Address::generate(&env);
    ctx.token_admin.mint(&user, &1_000i128);
    ctx.client
        .place_prediction(&user, &pool_id, &1_000i128, &0u32, &None, &None);

    assert_eq!(ctx.client.get_pool(&pool_id).participants_count, 1);

    // Creator tries to update after participant has joined.
    let result = ctx.client.try_update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "Post-join edit"),
    );
    assert_eq!(
        result,
        Err(Ok(PredifiError::InvalidPoolState)),
        "description must be locked once a participant has joined"
    );
}

/// The creator can update before any participant joins, and the change must
/// persist. This is the happy path.
#[test]
fn test_1526_creator_can_update_before_any_participant_joins() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    let new_desc = "Updated description before participants";
    ctx.client.update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, new_desc),
    );

    assert_eq!(
        ctx.client.get_pool(&pool_id).description,
        String::from_str(&env, new_desc),
        "creator update before participants must persist"
    );
}

/// Multiple consecutive updates by the creator must each overwrite the previous
/// and the final description must reflect the last call.
#[test]
fn test_1526_multiple_consecutive_updates_last_one_wins() {
    let env = Env::default();
    let ctx = DescTestEnv::new(&env);
    let pool_id = ctx.create_pool(7_200);

    ctx.client.update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "First update"),
    );
    ctx.client.update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "Second update"),
    );
    ctx.client.update_pool_description(
        &ctx.creator,
        &pool_id,
        &String::from_str(&env, "Third update"),
    );

    assert_eq!(
        ctx.client.get_pool(&pool_id).description,
        String::from_str(&env, "Third update"),
        "the final description must be the one from the last update"
    );
}
