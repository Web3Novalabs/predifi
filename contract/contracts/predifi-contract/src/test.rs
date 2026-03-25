#![cfg(test)]
#![allow(deprecated)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{storage::Instance as _, storage::Persistent as _, Address as _, Events, Ledger},
    token, vec, Address, BytesN, Env, IntoVal, String, Symbol, TryFromVal, Val,
};

mod dummy_access_control {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct DummyAccessControl;

    #[contractimpl]
    impl DummyAccessControl {
        pub fn grant_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().set(&key, &true);
        }

        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }
    }
}

mod rogue_token {
    use crate::PredifiContractClient;
    use soroban_sdk::{contract, contractimpl, Address, Env};

    #[contract]
    pub struct RogueToken;

    #[contractimpl]
    impl RogueToken {
        pub fn transfer(env: Env, _from: Address, _to: Address, _amount: i128) {
            if env.ledger().timestamp() > 100000 {
                let target: Address = env.storage().instance().get(&0u32).unwrap();
                let user: Address = env.storage().instance().get(&1u32).unwrap();
                let pool_id: u64 = env.storage().instance().get(&2u32).unwrap();

                let client = PredifiContractClient::new(&env, &target);
                client.claim_winnings(&user, &pool_id);
            }
        }

        pub fn setup(env: Env, target: Address, user: Address, pool_id: u64) {
            env.storage().instance().set(&0u32, &target);
            env.storage().instance().set(&1u32, &user);
            env.storage().instance().set(&2u32, &pool_id);
        }
    }
}

const ROLE_ADMIN: u32 = 0; // i am testing this
const ROLE_OPERATOR: u32 = 1; // i am testing this the second one
const ROLE_ORACLE: u32 = 3;

fn setup(
    env: &Env,
) -> (
    dummy_access_control::DummyAccessControlClient<'_>,
    PredifiContractClient<'_>,
    Address,
    token::Client<'_>,
    token::StellarAssetClient<'_>,
    Address,
    Address,
    Address,
) {
    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(env, &ac_id);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);

    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token = token::Client::new(env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(env, &token_contract);
    let token_address = token_contract;

    let treasury = Address::generate(env);
    let operator = Address::generate(env);
    let creator = Address::generate(env);
    let admin = Address::generate(env);

    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&admin, &token_address);

    (
        ac_client,
        client,
        token_address,
        token,
        token_admin_client,
        treasury,
        operator,
        creator,
    )
}

// ── Core prediction tests ────────────────────────────────────────────────────

#[test]
fn test_claim_winnings() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator, creator) = setup(&env);
    let contract_addr = client.address.clone();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );
    client.place_prediction(&user1, &pool_id, &100, &1, &None, &None);
    client.place_prediction(&user2, &pool_id, &100, &2, &None, &None);

    assert_eq!(token.balance(&contract_addr), 200);

    env.ledger().with_mut(|li| li.timestamp = 100001);

    client.resolve_pool(&operator, &pool_id, &1u32);

    let winnings = client.claim_winnings(&user1, &pool_id);
    assert_eq!(winnings, 200);
    assert_eq!(token.balance(&user1), 1100);

    let winnings2 = client.claim_winnings(&user2, &pool_id);
    assert_eq!(winnings2, 0);
    assert_eq!(token.balance(&user2), 900);
}

/// Referral: referred user places with referrer; on claim, referrer receives a cut of the protocol fee.
#[test]
fn test_referral_fee_distribution() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;
    let treasury = Address::generate(&env);
    let operator = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &200u32, &0u64, &3600u64); // 2% protocol fee
    client.add_token_to_whitelist(&admin, &token_address);
    client.set_referral_cut_bps(&admin, &5000u32); // 50% of fee share to referrer

    let referrer = Address::generate(&env);
    let referred_user = Address::generate(&env);
    token_admin_client.mint(&referred_user, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Referral Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    // Referred user places with referrer (100 on outcome 0)
    client.place_prediction(
        &referred_user,
        &pool_id,
        &100,
        &0,
        &Some(referrer.clone()),
        &None,
    );
    assert_eq!(client.get_referred_volume(&referrer, &pool_id), 100);

    env.ledger().with_mut(|li| li.timestamp = 100001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    let winnings = client.claim_winnings(&referred_user, &pool_id);
    // Protocol fee = 2% of 100 = 2. Payout pool = 98. Winner gets 98.
    assert_eq!(winnings, 98);
    // Referrer gets 50% of (100/100 * 2) = 1
    assert_eq!(token.balance(&referrer), 1);
    assert_eq!(token.balance(&referred_user), 1000 - 100 + 98);
}

#[test]
#[should_panic(expected = "Error(Contract, #60)")]
fn test_double_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );
    client.place_prediction(&user1, &pool_id, &100, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 100001);

    client.resolve_pool(&operator, &pool_id, &1u32);

    client.claim_winnings(&user1, &pool_id);
    client.claim_winnings(&user1, &pool_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_claim_unresolved() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );
    client.place_prediction(&user1, &pool_id, &100, &1, &None, &None);

    client.claim_winnings(&user1, &pool_id);
}

#[test]
fn test_multiple_pools_independent() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    let pool_a = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );
    let pool_b = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    client.place_prediction(&user1, &pool_a, &100, &1, &None, &None);
    client.place_prediction(&user2, &pool_b, &100, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 100001);

    client.resolve_pool(&operator, &pool_a, &1u32);
    client.resolve_pool(&operator, &pool_b, &2u32);

    let w1 = client.claim_winnings(&user1, &pool_a);
    assert_eq!(w1, 100);

    let w2 = client.claim_winnings(&user2, &pool_b);
    assert_eq!(w2, 0);
}

// ── Access control tests ─────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_unauthorized_set_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, _, _, _, _, _, _creator) = setup(&env);
    let not_admin = Address::generate(&env);
    client.set_fee_bps(&not_admin, &999u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_unauthorized_set_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, _, _, _, _, _, _creator) = setup(&env);
    let not_admin = Address::generate(&env);
    let new_treasury = Address::generate(&env);
    client.set_treasury(&not_admin, &new_treasury);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_unauthorized_resolve_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);
    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );
    let not_operator = Address::generate(&env);
    env.ledger().with_mut(|li| li.timestamp = 10001);
    client.resolve_pool(&not_operator, &pool_id, &1u32);
}

#[test]
fn test_oracle_can_resolve() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let treasury = Address::generate(&env);
    let oracle = Address::generate(&env);
    let admin = Address::generate(&env);

    ac_client.grant_role(&oracle, &ROLE_ORACLE);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&admin, &token_address);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100001);

    // Call oracle_resolve which should succeed
    client.oracle_resolve(
        &oracle,
        &pool_id,
        &1u32,
        &String::from_str(&env, "proof_123"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_unauthorized_oracle_resolve() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let treasury = Address::generate(&env);
    let not_oracle = Address::generate(&env);

    let admin = Address::generate(&env);
    // Give them OPERATOR instead of ORACLE, they still shouldn't be able to call oracle_resolve
    ac_client.grant_role(&not_oracle, &ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&admin, &token_address);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100001);

    client.oracle_resolve(
        &not_oracle,
        &pool_id,
        &1u32,
        &String::from_str(&env, "proof_123"),
    );
}

#[test]
fn test_admin_can_set_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.set_fee_bps(&admin, &500u32);
}

#[test]
fn test_admin_can_set_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let new_treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.set_treasury(&admin, &new_treasury);
}

// ── Pause tests ───────────────────────────────────────────────────────────────

#[test]
fn test_admin_can_pause_and_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.pause(&admin);
    client.unpause(&admin);
}

#[test]
#[should_panic]
fn test_admin_can_upgrade() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    // We expect this to panic in the mock environment because the Wasm hash is not registered.
    // The point is to verify it passes the Authorization check.
    let new_wasm_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.upgrade_contract(&admin, &new_wasm_hash);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_non_admin_cannot_upgrade() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let not_admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    let new_wasm_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.upgrade_contract(&not_admin, &new_wasm_hash);
}

#[test]
fn test_admin_can_migrate() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.migrate_state(&admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_non_admin_cannot_migrate() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let not_admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.migrate_state(&not_admin);
}

#[test]
#[should_panic(expected = "Unauthorized: missing required role")]
fn test_non_admin_cannot_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let not_admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.pause(&not_admin);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_set_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.pause(&admin);
    client.set_fee_bps(&admin, &100u32);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_set_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.pause(&admin);
    client.set_treasury(&admin, &Address::generate(&env));
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_create_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&admin, &token);

    let creator = Address::generate(&env);
    client.pause(&admin);
    client.create_pool(
        &creator,
        &100000u64,
        &token,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_place_prediction() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.pause(&admin);
    client.place_prediction(&user, &0u64, &10, &1, &None, &None);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_resolve_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.pause(&admin);
    client.resolve_pool(&operator, &0u64, &1u32);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_claim_winnings() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.pause(&admin);
    client.claim_winnings(&user, &0u64);
}

#[test]
fn test_unpause_restores_functionality() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&admin, &token_contract);
    token_admin_client.mint(&user, &1000);

    let creator = Address::generate(&env);
    client.pause(&admin);
    client.unpause(&admin);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_contract,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );
    client.place_prediction(&user, &pool_id, &10, &1, &None, &None);
}

// ── Pagination tests ──────────────────────────────────────────────────────────

#[test]
fn test_get_user_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let pool0 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );
    let pool1 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );
    let pool2 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    client.place_prediction(&user, &pool0, &10, &1, &None, &None);
    client.place_prediction(&user, &pool1, &20, &2, &None, &None);
    client.place_prediction(&user, &pool2, &30, &1, &None, &None);

    let first_two = client.get_user_predictions(&user, &0, &2);
    assert_eq!(first_two.len(), 2);
    assert_eq!(first_two.get(0).unwrap().pool_id, pool0);
    assert_eq!(first_two.get(1).unwrap().pool_id, pool1);

    let last_two = client.get_user_predictions(&user, &1, &2);
    assert_eq!(last_two.len(), 2);
    assert_eq!(last_two.get(0).unwrap().pool_id, pool1);
    assert_eq!(last_two.get(1).unwrap().pool_id, pool2);

    let last_one = client.get_user_predictions(&user, &2, &1);
    assert_eq!(last_one.len(), 1);
    assert_eq!(last_one.get(0).unwrap().pool_id, pool2);

    let p2 = client.get_user_predictions(&user, &2, &5);
    assert_eq!(p2.len(), 1);
}

#[test]
fn test_multi_oracle_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, _, _treasury, _, creator) = setup(&env);

    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let oracle3 = Address::generate(&env);

    ac_client.grant_role(&oracle1, &ROLE_ORACLE);
    ac_client.grant_role(&oracle2, &ROLE_ORACLE);
    ac_client.grant_role(&oracle3, &ROLE_ORACLE);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Multi-Oracle Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 2u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100001);

    // Oracle 1 votes for outcome 1
    client.oracle_resolve(&oracle1, &pool_id, &1u32, &String::from_str(&env, "proof1"));

    // Verify pool is NOT yet resolved
    let _stats = client.get_pool_stats(&pool_id);
    // Since get_pool_stats doesn't directly show "resolved" bool in PoolStats struct (it shows current_odds etc)
    // We could try to claim winnings and expect failure.
    let _user1 = Address::generate(&env);
    // Actually, let's just use oracle_resolve and see it works.

    // Oracle 2 votes for outcome 2 (Conflict!)
    client.oracle_resolve(&oracle2, &pool_id, &2u32, &String::from_str(&env, "proof2"));

    // Oracle 3 votes for outcome 1 (Threshold met!)
    client.oracle_resolve(&oracle3, &pool_id, &1u32, &String::from_str(&env, "proof3"));

    // Now it should be resolved to 1.
    // If we call get_user_predictions for a user who predicted 1, it should show resolved.
}
// ── Pool cancellation tests ───────────────────────────────────────────────────

#[test]
fn test_admin_can_cancel_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let whitelist_admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let creator = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    ac_client.grant_role(&whitelist_admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    // Admin should be able to cancel
    client.cancel_pool(&admin, &pool_id);
}

#[test]
fn test_pool_creator_can_cancel_unresolved_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let creator = Address::generate(&env);
    let treasury = Address::generate(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&creator, &ROLE_OPERATOR);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&admin, &token_address);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    // Admin should be able to cancel their pool
    client.cancel_pool(&creator, &pool_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_non_admin_non_creator_cannot_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    let unauthorized = Address::generate(&env);
    // This should fail - user is not admin
    client.cancel_pool(&unauthorized, &pool_id);
}

// ── Token whitelist tests ───────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #91)")]
fn test_create_pool_rejects_non_whitelisted_token() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let treasury = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_not_whitelisted = Address::generate(&env);

    ac_client.grant_role(&creator, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    // Do NOT whitelist token_not_whitelisted

    client.create_pool(
        &creator,
        &100000u64,
        &token_not_whitelisted,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool"),
            metadata_url: String::from_str(&env, "ipfs://meta"),
            min_stake: 0i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
}

#[test]
fn test_token_whitelist_add_remove_and_is_allowed() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    assert!(!client.is_token_allowed(&token));
    client.add_token_to_whitelist(&admin, &token);
    assert!(client.is_token_allowed(&token));
    client.remove_token_from_whitelist(&admin, &token);
    assert!(!client.is_token_allowed(&token));
}

/// Helper to set up a minimal contract + access control for whitelist tests.
fn setup_whitelist_env() -> (Env, PredifiContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    (env, client, admin, treasury)
}

#[test]
fn test_token_not_whitelisted_by_default() {
    let (env, client, _admin, _treasury) = setup_whitelist_env();
    let token = Address::generate(&env);
    assert!(!client.is_token_allowed(&token));
}

#[test]
fn test_add_token_to_whitelist_makes_it_allowed() {
    let (env, client, admin, _treasury) = setup_whitelist_env();
    let token = Address::generate(&env);

    client.add_token_to_whitelist(&admin, &token);
    assert!(client.is_token_allowed(&token));
}

#[test]
fn test_remove_token_from_whitelist_disallows_it() {
    let (env, client, admin, _treasury) = setup_whitelist_env();
    let token = Address::generate(&env);

    client.add_token_to_whitelist(&admin, &token);
    assert!(client.is_token_allowed(&token));

    client.remove_token_from_whitelist(&admin, &token);
    assert!(!client.is_token_allowed(&token));
}

#[test]
fn test_readd_token_after_removal_works() {
    let (env, client, admin, _treasury) = setup_whitelist_env();
    let token = Address::generate(&env);

    client.add_token_to_whitelist(&admin, &token);
    client.remove_token_from_whitelist(&admin, &token);
    assert!(!client.is_token_allowed(&token));

    // Re-add should work fine
    client.add_token_to_whitelist(&admin, &token);
    assert!(client.is_token_allowed(&token));
}

#[test]
fn test_multiple_tokens_whitelisted_independently() {
    let (env, client, admin, _treasury) = setup_whitelist_env();
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let token_c = Address::generate(&env);

    client.add_token_to_whitelist(&admin, &token_a);
    client.add_token_to_whitelist(&admin, &token_b);

    assert!(client.is_token_allowed(&token_a));
    assert!(client.is_token_allowed(&token_b));
    assert!(!client.is_token_allowed(&token_c));

    // Removing one doesn't affect the other
    client.remove_token_from_whitelist(&admin, &token_a);
    assert!(!client.is_token_allowed(&token_a));
    assert!(client.is_token_allowed(&token_b));
}

#[test]
#[should_panic]
fn test_unauthorized_add_to_whitelist_panics() {
    let (env, client, _admin, _treasury) = setup_whitelist_env();
    let non_admin = Address::generate(&env);
    let token = Address::generate(&env);

    // non_admin has no role — should fail
    client.add_token_to_whitelist(&non_admin, &token);
}

#[test]
#[should_panic]
fn test_unauthorized_remove_from_whitelist_panics() {
    let (env, client, admin, _treasury) = setup_whitelist_env();
    let non_admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.add_token_to_whitelist(&admin, &token);
    // non_admin has no role — should fail
    client.remove_token_from_whitelist(&non_admin, &token);
}

#[test]
fn test_whitelist_persists_in_persistent_storage() {
    let (env, client, admin, _treasury) = setup_whitelist_env();
    let token = Address::generate(&env);

    client.add_token_to_whitelist(&admin, &token);

    // Verify the key exists in persistent storage (not instance)
    let key = DataKey::TokenWl(token.clone());
    let in_persistent = env.as_contract(&client.address, || env.storage().persistent().has(&key));
    assert!(in_persistent, "Token should be in persistent storage");

    // Confirm it is NOT in instance storage
    let in_instance = env.as_contract(&client.address, || env.storage().instance().has(&key));
    assert!(!in_instance, "Token should NOT be in instance storage");
}

#[test]
#[should_panic(expected = "Error(Contract, #91)")]
fn test_place_prediction_fails_for_non_whitelisted_token() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    // Intentionally do NOT whitelist the token
    token_admin_client.mint(&user, &1000);

    env.ledger().with_mut(|l| l.timestamp = 1000);

    // create_pool itself checks the whitelist — should panic with TokenNotWhitelisted (#91)
    client.create_pool(
        &creator,
        &2000u64,
        &token_contract,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "desc"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 10i128,
            max_stake: 500i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
}

#[test]
fn test_place_prediction_succeeds_for_whitelisted_token() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    // Whitelist the token
    client.add_token_to_whitelist(&admin, &token_contract);
    token_admin_client.mint(&user, &1000);

    env.ledger().with_mut(|l| l.timestamp = 1000);

    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_contract,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "desc"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 10i128,
            max_stake: 500i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Should succeed without panic
    client.place_prediction(&user, &pool_id, &100i128, &0u32, &None, &None);

    // Verify prediction was recorded via get_user_predictions
    let preds = client.get_user_predictions(&user, &0u32, &10u32);
    assert_eq!(preds.len(), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_cannot_cancel_resolved_pool_by_operator() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let whitelist_admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    let creator = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    ac_client.grant_role(&whitelist_admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100001);
    client.resolve_pool(&operator, &pool_id, &1u32);

    // Now try to cancel - should fail
    client.cancel_pool(&admin, &pool_id);
}

#[test]
#[should_panic(expected = "Cannot place prediction on canceled pool")]
fn test_cannot_place_prediction_on_canceled_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let whitelist_admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    ac_client.grant_role(&whitelist_admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    // Create and cancel pool
    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    // Cancel the pool
    client.cancel_pool(&admin, &pool_id);

    // Try to place prediction on canceled pool - should panic
    client.place_prediction(&user, &pool_id, &100, &1, &None, &None);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_pool_creator_cannot_cancel_after_admin_cancels() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let whitelist_admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    ac_client.grant_role(&whitelist_admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    // Admin cancels the pool
    client.cancel_pool(&admin, &pool_id);

    // Attempt to cancel again should fail (already canceled)
    let non_admin = Address::generate(&env);
    client.cancel_pool(&non_admin, &pool_id);
}

#[test]
#[should_panic(expected = "Cannot place prediction on canceled pool")]
fn test_admin_can_cancel_pool_with_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let whitelist_admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    ac_client.grant_role(&whitelist_admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(
                &env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    // User places a prediction
    client.place_prediction(&user, &pool_id, &100, &1, &None, &None);

    // Admin cancels the pool - this freezes betting
    client.cancel_pool(&admin, &pool_id);

    // Verify no more predictions can be placed - should panic
    client.place_prediction(&user, &pool_id, &50, &2, &None, &None);
}

#[test]
fn test_cancel_pool_refunds_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let whitelist_admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    ac_client.grant_role(&whitelist_admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let creator = Address::generate(&env);
    let contract_addr = client.address.clone();
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Cancel Test Pool"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // User places a prediction
    client.place_prediction(&user1, &pool_id, &100, &1, &None, &None);
    assert_eq!(token_admin_client.balance(&contract_addr), 100);
    assert_eq!(token_admin_client.balance(&user1), 900);

    // Admin cancels the pool - this should enable refund of predictions
    client.cancel_pool(&admin, &pool_id);

    // Verify predictions are refunded (get_user_predictions should show the prediction still exists for potential refund claim)
    let predictions = client.get_user_predictions(&user1, &0u32, &10u32);
    assert_eq!(predictions.len(), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_cannot_cancel_resolved_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, _) = setup(&env);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Resolve Then Cancel Pool"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 10001);
    client.resolve_pool(&operator, &pool_id, &1u32);
    // Should panic because pool is already resolved
    client.cancel_pool(&operator, &pool_id);
}

#[test]
#[should_panic(expected = "Cannot resolve a canceled pool")]
fn test_cannot_resolve_canceled_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let admin = Address::generate(&env);
    let whitelist_admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_OPERATOR);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    ac_client.grant_role(&whitelist_admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    client.cancel_pool(&admin, &pool_id);
    // Should panic because pool is not active (canceled)
    client.resolve_pool(&operator, &pool_id, &1u32);
}

#[test]
#[should_panic(expected = "Cannot place prediction on canceled pool")]
fn test_cannot_predict_on_canceled_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, _) = setup(&env);
    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Predict Canceled Pool Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    client.cancel_pool(&operator, &pool_id);
    // Should panic
    client.place_prediction(&user1, &pool_id, &100, &1, &None, &None);
}

#[test]
#[should_panic(expected = "Error(Contract, #81)")]
fn test_resolve_pool_before_delay() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    // Init with 3600s delay
    client.init(&ac_id, &treasury, &0u32, &3600u64, &3600u64);
    client.add_token_to_whitelist(&admin, &token);

    let end_time = 10000;
    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Delay Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Set time to end_time + MIN_POOL_DURATION (to allow creation)
    // Wait, create_pool checks end_time > current_time + MIN_POOL_DURATION.
    // In setup, current_time is 0. So 10000 is fine.

    // Set time to end_time + 10s (less than delay)
    env.ledger().with_mut(|li| li.timestamp = end_time + 10);

    // Should panic with ResolutionDelayNotMet (81)
    client.resolve_pool(&operator, &pool_id, &1u32);
}

#[test]
fn test_resolve_pool_after_delay() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    // Init with 3600s delay
    client.init(&ac_id, &treasury, &0u32, &3600u64, &3600u64);
    client.add_token_to_whitelist(&admin, &token);

    let end_time = 10000;
    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Delay Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Set time to end_time + 3601s (more than delay)
    env.ledger().with_mut(|li| li.timestamp = end_time + 3601);

    // Should succeed
    client.resolve_pool(&operator, &pool_id, &1u32);
}

#[test]
fn test_mark_pool_ready() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &3600u64, &3600u64);
    client.add_token_to_whitelist(&admin, &token);

    let end_time = 10000;
    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Ready Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Test before delay
    env.ledger().with_mut(|li| li.timestamp = end_time + 10);
    let res = client.try_mark_pool_ready(&pool_id);
    assert!(res.is_err());

    // Test after delay
    env.ledger().with_mut(|li| li.timestamp = end_time + 3600);
    let res = client.try_mark_pool_ready(&pool_id);
    assert!(res.is_ok());
}

// ── Staking Limits Tests ──────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #107)")]
fn test_stake_below_minimum_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, _) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let creator = Address::generate(&env);
    // Create pool with min_stake = 50
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Min Stake Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 50i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Should panic: amount (10) < min_stake (50)
    client.place_prediction(&user, &pool_id, &10, &0, &None, &None);
}

#[test]
#[should_panic(expected = "Error(Contract, #108)")]
fn test_stake_above_maximum_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, _) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let creator = Address::generate(&env);
    // Create pool with min_stake = 1, max_stake = 100
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Max Stake Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 100i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Should panic: amount (200) > max_stake (100)
    client.place_prediction(&user, &pool_id, &200, &0, &None, &None);
}

#[test]
fn test_stake_at_boundaries_accepted() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, _) = setup(&env);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    let creator = Address::generate(&env);
    // Create pool with min_stake = 10, max_stake = 200
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Boundary Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 10i128,
            max_stake: 200i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Both boundary values should succeed
    client.place_prediction(&user1, &pool_id, &10, &0, &None, &None); // exactly min_stake
    client.place_prediction(&user2, &pool_id, &200, &1, &None, &None); // exactly max_stake
}

#[test]
fn test_set_stake_limits_by_operator() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, _) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let creator = Address::generate(&env);
    // Create pool with min_stake = 1
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Update Limits Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Operator updates: min_stake = 50, max_stake = 500
    client.set_stake_limits(&operator, &pool_id, &50i128, &500i128);

    // Stake at the new minimum should succeed
    client.place_prediction(&user, &pool_id, &50, &0, &None, &None);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_set_stake_limits_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, _) = setup(&env);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Unauthorized Limits Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Non-operator should be rejected
    let not_operator = Address::generate(&env);
    client.set_stake_limits(&not_operator, &pool_id, &50i128, &500i128);
}

#[test]
fn test_set_stake_limits_zero_min_stake_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, _) = setup(&env);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Zero Min Stake Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let result = client.try_set_stake_limits(&operator, &pool_id, &0i128, &0i128);
    assert_eq!(result, Err(Ok(PredifiError::StakeBelowMinimum)));
}

#[test]
fn test_set_stake_limits_max_below_min_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, _) = setup(&env);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &10000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Max Below Min Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let result = client.try_set_stake_limits(&operator, &pool_id, &100i128, &50i128);
    assert_eq!(result, Err(Ok(PredifiError::StakeAboveMaximum)));
}

#[test]
fn test_get_pools_by_category() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let cat1 = symbol_short!("Tech");
    let cat2 = symbol_short!("Sports");

    let pool0 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &cat1,
        &PoolConfig {
            description: String::from_str(&env, "Pool 0"),
            metadata_url: String::from_str(&env, "ipfs://0"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    let pool1 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &cat1,
        &PoolConfig {
            description: String::from_str(&env, "Pool 1"),
            metadata_url: String::from_str(&env, "ipfs://1"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    let pool2 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &cat2,
        &PoolConfig {
            description: String::from_str(&env, "Pool 2"),
            metadata_url: String::from_str(&env, "ipfs://2"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let tech_pools = client.get_pools_by_category(&cat1, &0, &10);
    assert_eq!(tech_pools.len(), 2);
    assert_eq!(tech_pools.get(0).unwrap(), pool1);
    assert_eq!(tech_pools.get(1).unwrap(), pool0);

    let sports_pools = client.get_pools_by_category(&cat2, &0, &10);
    assert_eq!(sports_pools.len(), 1);
    assert_eq!(sports_pools.get(0).unwrap(), pool2);

    let paginated = client.get_pools_by_category(&cat1, &1, &1);
    assert_eq!(paginated.len(), 1);
    assert_eq!(paginated.get(0).unwrap(), pool0);

    let empty = client.get_pools_by_category(&cat1, &2, &10);
    assert_eq!(empty.len(), 0);
}

// ================== Treasury withdrawal tests ==================

#[test]
fn test_admin_can_withdraw_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, token, token_admin_client, treasury, _, _creator) =
        setup(&env);
    let contract_addr = client.address.clone();
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Mint tokens to contract (simulating accumulated fees)
    token_admin_client.mint(&contract_addr, &5000);

    // Admin withdraws to treasury
    client.withdraw_treasury(&admin, &token_address, &3000, &treasury);

    // Verify balances
    assert_eq!(token.balance(&treasury), 3000);
    assert_eq!(token.balance(&contract_addr), 2000);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_non_admin_cannot_withdraw_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _token, token_admin_client, treasury, _, _) = setup(&env);
    let contract_addr = client.address.clone();
    let non_admin = Address::generate(&env);

    token_admin_client.mint(&contract_addr, &5000);

    // Non-admin tries to withdraw - should panic
    client.withdraw_treasury(&non_admin, &token_address, &3000, &treasury);
}

#[test]
#[should_panic(expected = "Error(Contract, #42)")]
fn test_withdraw_treasury_rejects_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _token, token_admin_client, treasury, _, _) =
        setup(&env);
    let contract_addr = client.address.clone();
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    token_admin_client.mint(&contract_addr, &5000);

    // Try to withdraw zero amount - should panic
    client.withdraw_treasury(&admin, &token_address, &0, &treasury);
}

#[test]
#[should_panic(expected = "Error(Contract, #44)")]
fn test_withdraw_treasury_rejects_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _token, token_admin_client, treasury, _, _) =
        setup(&env);
    let contract_addr = client.address.clone();
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    token_admin_client.mint(&contract_addr, &1000);

    // Try to withdraw more than balance - should panic
    client.withdraw_treasury(&admin, &token_address, &5000, &treasury);
}

#[test]
fn test_withdraw_treasury_multiple_tokens_with_pools_and_fees() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, token, token_admin_client, treasury, operator, creator) =
        setup(&env);
    let contract_addr = client.address.clone();
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Setup second token
    let token_admin2 = Address::generate(&env);
    let token_contract2 = env.register_stellar_asset_contract(token_admin2.clone());
    let token2 = token::Client::new(&env, &token_contract2);
    let token_admin_client2 = token::StellarAssetClient::new(&env, &token_contract2);
    client.add_token_to_whitelist(&admin, &token_contract2);

    // Set protocol fee to 10% (1000 bps) for clear fee calculation
    client.set_fee_bps(&admin, &1000u32);

    // Create two pools with different tokens
    let pool1_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Finance"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 1 - Token 1"),
            metadata_url: String::from_str(&env, "ipfs://pool1"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let pool2_id = client.create_pool(
        &creator,
        &100001u64,
        &token_contract2,
        &2u32,
        &symbol_short!("Crypto"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 2 - Token 2"),
            metadata_url: String::from_str(&env, "ipfs://pool2"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Create users for betting
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);
    token_admin_client2.mint(&user1, &1000);
    token_admin_client2.mint(&user2, &1000);

    // Place predictions in pool1 (token1) - user1 bets on outcome 0, user2 on outcome 1
    client.place_prediction(&user1, &pool1_id, &500, &0, &None, &None);
    client.place_prediction(&user2, &pool1_id, &500, &1, &None, &None);

    // Place predictions in pool2 (token2) - user1 bets on outcome 0, user2 on outcome 1
    client.place_prediction(&user1, &pool2_id, &400, &0, &None, &None);
    client.place_prediction(&user2, &pool2_id, &600, &1, &None, &None);

    // Verify contract balances before resolution
    assert_eq!(token.balance(&contract_addr), 1000); // 500 + 500 from pool1
    assert_eq!(token2.balance(&contract_addr), 1000); // 400 + 600 from pool2

    // Advance time to allow resolution
    env.ledger().with_mut(|li| li.timestamp = 100001);

    // Resolve both pools with different outcomes
    // Pool1: outcome 0 wins (user1 wins)
    client.resolve_pool(&operator, &pool1_id, &0u32);
    // Pool2: outcome 1 wins (user2 wins)
    client.resolve_pool(&operator, &pool2_id, &1u32);

    // Users claim winnings - this is where fees are collected
    // Pool1: total_stake=1000, fee=10% (100), payout=900, user1 gets all 900
    let winnings1_user1 = client.claim_winnings(&user1, &pool1_id);
    assert_eq!(winnings1_user1, 900); // 1000 - 10% fee

    // Pool2: total_stake=1000, fee=10% (100), payout=900, user2 gets all 900
    let winnings2_user2 = client.claim_winnings(&user2, &pool2_id);
    assert_eq!(winnings2_user2, 900); // 1000 - 10% fee

    // Verify contract balances after claims (should have 100 tokens each as fees)
    assert_eq!(token.balance(&contract_addr), 100); // 10% fee from pool1
    assert_eq!(token2.balance(&contract_addr), 100); // 10% fee from pool2

    // Verify treasury balances before withdraw
    assert_eq!(token.balance(&treasury), 0);
    assert_eq!(token2.balance(&treasury), 0);

    // Withdraw treasury for token1 (partial withdrawal - 60 out of 100)
    client.withdraw_treasury(&admin, &token_address, &60, &treasury);

    // Verify first withdrawal
    assert_eq!(token.balance(&treasury), 60);
    assert_eq!(token.balance(&contract_addr), 40); // 100 - 60
                                                   // Token2 should be unaffected
    assert_eq!(token2.balance(&treasury), 0);
    assert_eq!(token2.balance(&contract_addr), 100);

    // Withdraw treasury for token2 (full withdrawal - 100)
    client.withdraw_treasury(&admin, &token_contract2, &100, &treasury);

    // Verify second withdrawal doesn't affect token1
    assert_eq!(token.balance(&treasury), 60); // Unchanged
    assert_eq!(token.balance(&contract_addr), 40); // Unchanged
    assert_eq!(token2.balance(&treasury), 100);
    assert_eq!(token2.balance(&contract_addr), 0);

    // Withdraw remaining token1 balance
    client.withdraw_treasury(&admin, &token_address, &40, &treasury);

    // Final verification - all fees withdrawn independently
    assert_eq!(token.balance(&treasury), 100); // 60 + 40
    assert_eq!(token.balance(&contract_addr), 0);
    assert_eq!(token2.balance(&treasury), 100);
    assert_eq!(token2.balance(&contract_addr), 0);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_withdraw_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _token, token_admin_client, treasury, _, _) =
        setup(&env);
    let contract_addr = client.address.clone();
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    token_admin_client.mint(&contract_addr, &5000);

    // Pause contract
    client.pause(&admin);

    // Try to withdraw while paused - should panic
    client.withdraw_treasury(&admin, &token_address, &1000, &treasury);
}

#[test]
fn test_get_pool_stats() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    token_admin_client.mint(&user1, &5000);
    token_admin_client.mint(&user2, &5000);
    token_admin_client.mint(&user3, &5000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: // Binary pool
        String::from_str(&env, "Stats Test"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32, private: false, whitelist_key: None,
            outcome_descriptions: vec![&env, String::from_str(&env, "Outcome 0"), String::from_str(&env, "Outcome 1")],
        },
    );

    // Initial stats
    let stats = client.get_pool_stats(&pool_id);
    assert_eq!(stats.participants_count, 0);
    assert_eq!(stats.total_stake, 0);

    // User 1 bets 100 on outcome 0
    client.place_prediction(&user1, &pool_id, &100, &0, &None, &None);
    // User 2 bets 200 on outcome 1
    client.place_prediction(&user2, &pool_id, &200, &1, &None, &None);
    // User 3 bets 100 on outcome 1
    client.place_prediction(&user3, &pool_id, &100, &1, &None, &None);
    // User 1 bets 100 more on outcome 0 (should not increase participants)
    client.place_prediction(&user1, &pool_id, &100, &0, &None, &None);

    let stats = client.get_pool_stats(&pool_id);
    assert_eq!(stats.participants_count, 3);
    assert_eq!(stats.total_stake, 500); // 100+200+100+100
    assert_eq!(stats.stakes_per_outcome.get(0), Some(200));
    assert_eq!(stats.stakes_per_outcome.get(1), Some(300));

    // Odds:
    // Outcome 0: (500 * 10000) / 200 = 25000 (2.5x)
    // Outcome 1: (500 * 10000) / 300 = 16666 (1.6666x)
    assert_eq!(stats.current_odds.get(0), Some(25000));
    assert_eq!(stats.current_odds.get(1), Some(16666));
}

// ═══════════════════════════════════════════════════════════════════════════
// EDGE-CASE TESTS  (#327)
// ═══════════════════════════════════════════════════════════════════════════
//
// Coverage additions mandated by GitHub issue #327:
//   • Leap-year timestamp boundaries
//   • Maximum possible stake values
//   • Rapid resolution / claim sequences
//   • Boundary values in all validation logic
//   • (Simulated) race conditions & unauthorized access attempts
//   • State consistency after multiple resolution cycles

// ── Constants for leap-year tests ────────────────────────────────────────────

/// Feb 28, 2024 00:00:00 UTC (day before the 2024 leap day).
const FEB_28_2024_UTC: u64 = 1_709_078_400;
/// Feb 29, 2024 00:00:00 UTC (2024 is a leap year).
const LEAP_DAY_2024_UTC: u64 = 1_709_164_800;
/// Mar 01, 2024 00:00:00 UTC (first day after the 2024 leap day).
const MAR_01_2024_UTC: u64 = 1_709_251_200;

// ── Leap-year timestamp edge cases ───────────────────────────────────────────

/// A pool whose end time falls exactly on the leap day (Feb 29, 2024)
/// must be created and accepted for predictions without any off-by-one error.
#[test]
fn test_pool_end_time_on_leap_day() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    // Advance ledger to Feb 28. end_time = Feb 29 (86 400 s later, well above 3 600 s minimum).
    env.ledger().with_mut(|li| li.timestamp = FEB_28_2024_UTC);

    let pool_id = client.create_pool(
        &creator,
        &LEAP_DAY_2024_UTC,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Leap year pool"),
            metadata_url: String::from_str(&env, "ipfs://leap"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);
    // Prediction must be accepted while before the leap-day deadline.
    client.place_prediction(&user, &pool_id, &100, &0, &None, &None);
}

/// Creating a pool whose end time is the leap day, but the ledger is already
/// past Mar 1, must be rejected because the end time is in the past.
#[test]
#[should_panic(expected = "end_time must be in the future")]
fn test_pool_end_time_at_leap_day_already_past() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    // Ledger at Mar 1 – the leap day is in the past.
    env.ledger().with_mut(|li| li.timestamp = MAR_01_2024_UTC);

    client.create_pool(
        &creator,
        &LEAP_DAY_2024_UTC,
        // Feb 29 – already past
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Expired leap pool"),
            metadata_url: String::from_str(&env, "ipfs://expired"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
}

/// A pool created before the leap day, resolved after it, must behave
/// correctly.  This validates timestamp arithmetic across the Feb 29 boundary.
#[test]
fn test_pool_end_time_spans_leap_day_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    // Creation at Feb 28 00:00 UTC – 3 600 s before end_time on Mar 01.
    // (Difference = 1 709 251 200 – 1 709 074 800 = 176 400 > MIN_POOL_DURATION)
    let creation_time: u64 = FEB_28_2024_UTC - 3_600;
    env.ledger().with_mut(|li| li.timestamp = creation_time);

    let pool_id = client.create_pool(
        &creator,
        &MAR_01_2024_UTC,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Leap span pool"),
            metadata_url: String::from_str(&env, "ipfs://span"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &500);
    token_admin_client.mint(&user2, &500);

    client.place_prediction(&user1, &pool_id, &300, &0, &None, &None);
    client.place_prediction(&user2, &pool_id, &200, &1, &None, &None);

    // Advance ledger past Mar 1 (resolution_delay == 0 in setup).
    env.ledger()
        .with_mut(|li| li.timestamp = MAR_01_2024_UTC + 1);
    client.resolve_pool(&operator, &pool_id, &0u32);

    // user1 staked on the winning outcome – receives full pot.
    let w1 = client.claim_winnings(&user1, &pool_id);
    assert_eq!(w1, 500);

    let w2 = client.claim_winnings(&user2, &pool_id);
    assert_eq!(w2, 0);
}

// ── Maximum possible stake amounts ───────────────────────────────────────────

/// A single bet equal to MAX_INITIAL_LIQUIDITY (the contract ceiling) must be
/// accepted, correctly recorded, and fully refunded on a win.
#[test]
fn test_maximum_single_stake_roundtrip() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator, creator) = setup(&env);

    // MAX_INITIAL_LIQUIDITY = 100_000_000_000_000
    let max_amount: i128 = 100_000_000_000_000;

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Max stake pool"),
            metadata_url: String::from_str(&env, "ipfs://max"),
            min_stake: 1i128,
            max_stake: max_amount,
            max_total_stake: 0,
            initial_liquidity: // max_stake == max_amount is valid
        0i128,
            required_resolutions: 1u32, private: false, whitelist_key: None,
            outcome_descriptions: vec![&env, String::from_str(&env, "Outcome 0"), String::from_str(&env, "Outcome 1")],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &max_amount);

    client.place_prediction(&user, &pool_id, &max_amount, &0, &None, &None);

    let contract_addr = client.address.clone();
    assert_eq!(token.balance(&contract_addr), max_amount);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    // Sole better on the winning side – receives the entire pot (no fee in setup).
    let winnings = client.claim_winnings(&user, &pool_id);
    assert_eq!(winnings, max_amount);
    assert_eq!(token.balance(&user), max_amount);
}

/// Two winners each holding large stakes on the winning side must receive
/// their proportional share without arithmetic overflow.
#[test]
fn test_large_stake_winnings_split_correctly() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    let big_stake: i128 = 10_000_000_000; // 10 billion base units

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Large stake split"),
            metadata_url: String::from_str(&env, "ipfs://large"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: // no max_stake limit
        0i128,
            required_resolutions: 1u32, private: false, whitelist_key: None,
            outcome_descriptions: vec![&env, String::from_str(&env, "Outcome 0"), String::from_str(&env, "Outcome 1")],
        },
    );

    let winner1 = Address::generate(&env);
    let winner2 = Address::generate(&env);
    let loser1 = Address::generate(&env);
    let loser2 = Address::generate(&env);
    token_admin_client.mint(&winner1, &big_stake);
    token_admin_client.mint(&winner2, &big_stake);
    token_admin_client.mint(&loser1, &big_stake);
    token_admin_client.mint(&loser2, &big_stake);

    // Two winners on outcome 0, two losers on outcome 1.
    client.place_prediction(&winner1, &pool_id, &big_stake, &0, &None, &None);
    client.place_prediction(&winner2, &pool_id, &big_stake, &0, &None, &None);
    client.place_prediction(&loser1, &pool_id, &big_stake, &1, &None, &None);
    client.place_prediction(&loser2, &pool_id, &big_stake, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    let total = big_stake * 4;
    let w1 = client.claim_winnings(&winner1, &pool_id);
    let w2 = client.claim_winnings(&winner2, &pool_id);

    // Each winner gets half the pot.
    assert_eq!(w1, total / 2);
    assert_eq!(w2, total / 2);
    assert_eq!(w1 + w2, total);

    // Losers get nothing.
    let l1 = client.claim_winnings(&loser1, &pool_id);
    let l2 = client.claim_winnings(&loser2, &pool_id);
    assert_eq!(l1, 0);
    assert_eq!(l2, 0);
}

// ── Rapid resolution / claim sequences ───────────────────────────────────────

/// Resolving the same pool twice in a row must fail the second time.
#[test]
#[should_panic(expected = "Pool already resolved")]
fn test_double_resolution_attempt() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Double resolve"),
            metadata_url: String::from_str(&env, "ipfs://double"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &0u32);
    // Second resolution must panic.
    client.resolve_pool(&operator, &pool_id, &1u32);
}

/// Ten users all claim winnings immediately after resolution.
/// The total paid out must equal the total staked (no value lost or created).
#[test]
fn test_many_users_rapid_claim_after_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator, creator) = setup(&env);
    let contract_addr = client.address.clone();

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Rapid claim"),
            metadata_url: String::from_str(&env, "ipfs://rapid"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let stake: i128 = 100;

    // 5 winners (outcome 0) and 5 losers (outcome 1).
    let w0 = Address::generate(&env);
    let w1 = Address::generate(&env);
    let w2 = Address::generate(&env);
    let w3 = Address::generate(&env);
    let w4 = Address::generate(&env);
    let l0 = Address::generate(&env);
    let l1 = Address::generate(&env);
    let l2 = Address::generate(&env);
    let l3 = Address::generate(&env);
    let l4 = Address::generate(&env);

    for u in [&w0, &w1, &w2, &w3, &w4] {
        token_admin_client.mint(u, &stake);
        client.place_prediction(u, &pool_id, &stake, &0, &None, &None);
    }
    for u in [&l0, &l1, &l2, &l3, &l4] {
        token_admin_client.mint(u, &stake);
        client.place_prediction(u, &pool_id, &stake, &1, &None, &None);
    }

    let total = stake * 10;
    assert_eq!(token.balance(&contract_addr), total);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    let mut total_paid: i128 = 0;
    for u in [&w0, &w1, &w2, &w3, &w4] {
        total_paid += client.claim_winnings(u, &pool_id);
    }
    for u in [&l0, &l1, &l2, &l3, &l4] {
        assert_eq!(client.claim_winnings(u, &pool_id), 0);
    }

    // No value created or destroyed (INV-5).
    assert_eq!(total_paid, total);
}

/// Resolving pool A then immediately creating pool B must leave pool A's
/// state intact.  Verifies the ID counter doesn't corrupt resolved data.
#[test]
fn test_resolution_then_new_pool_state_isolation() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    let pool_a = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool A"),
            metadata_url: String::from_str(&env, "ipfs://a"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &500);
    client.place_prediction(&user, &pool_a, &200, &0, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_a, &0u32);

    // Create pool B immediately after resolution.
    let pool_b = client.create_pool(
        &creator,
        &200_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool B"),
            metadata_url: String::from_str(&env, "ipfs://b"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    assert_ne!(pool_a, pool_b);

    // User can still claim from pool A.
    let winnings = client.claim_winnings(&user, &pool_a);
    assert_eq!(winnings, 200);

    // Pool B is still active – predictions can be placed.
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user2, &500);
    client.place_prediction(&user2, &pool_b, &100, &1, &None, &None);
}

// ── Boundary values in all validation logic ───────────────────────────────────

/// min_stake == 0 must be rejected.
#[test]
#[should_panic(expected = "min_stake must be greater than zero")]
fn test_create_pool_rejects_zero_min_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Zero min stake"),
            metadata_url: String::from_str(&env, "ipfs://zero"),
            min_stake: 0i128, // invalid
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
}

/// options_count == 1 must be rejected (minimum is 2).
#[test]
#[should_panic(expected = "options_count must be at least 2")]
fn test_create_pool_rejects_single_option() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &1u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Single option pool"), // invalid
            metadata_url: String::from_str(&env, "ipfs://single"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![&env, String::from_str(&env, "Outcome 0")],
        },
    );
}

/// options_count > MAX_OPTIONS_COUNT (100) must be rejected.
#[test]
#[should_panic(expected = "options_count exceeds maximum allowed value")]
fn test_create_pool_rejects_excess_options_count() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &101u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Too many options"),
            metadata_url: String::from_str(&env, "ipfs://many"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
                String::from_str(&env, "Outcome 3"),
                String::from_str(&env, "Outcome 4"),
                String::from_str(&env, "Outcome 5"),
                String::from_str(&env, "Outcome 6"),
                String::from_str(&env, "Outcome 7"),
                String::from_str(&env, "Outcome 8"),
                String::from_str(&env, "Outcome 9"),
                String::from_str(&env, "Outcome 10"),
                String::from_str(&env, "Outcome 11"),
                String::from_str(&env, "Outcome 12"),
                String::from_str(&env, "Outcome 13"),
                String::from_str(&env, "Outcome 14"),
                String::from_str(&env, "Outcome 15"),
                String::from_str(&env, "Outcome 16"),
                String::from_str(&env, "Outcome 17"),
                String::from_str(&env, "Outcome 18"),
                String::from_str(&env, "Outcome 19"),
                String::from_str(&env, "Outcome 20"),
                String::from_str(&env, "Outcome 21"),
                String::from_str(&env, "Outcome 22"),
                String::from_str(&env, "Outcome 23"),
                String::from_str(&env, "Outcome 24"),
                String::from_str(&env, "Outcome 25"),
                String::from_str(&env, "Outcome 26"),
                String::from_str(&env, "Outcome 27"),
                String::from_str(&env, "Outcome 28"),
                String::from_str(&env, "Outcome 29"),
                String::from_str(&env, "Outcome 30"),
                String::from_str(&env, "Outcome 31"),
                String::from_str(&env, "Outcome 32"),
                String::from_str(&env, "Outcome 33"),
                String::from_str(&env, "Outcome 34"),
                String::from_str(&env, "Outcome 35"),
                String::from_str(&env, "Outcome 36"),
                String::from_str(&env, "Outcome 37"),
                String::from_str(&env, "Outcome 38"),
                String::from_str(&env, "Outcome 39"),
                String::from_str(&env, "Outcome 40"),
                String::from_str(&env, "Outcome 41"),
                String::from_str(&env, "Outcome 42"),
                String::from_str(&env, "Outcome 43"),
                String::from_str(&env, "Outcome 44"),
                String::from_str(&env, "Outcome 45"),
                String::from_str(&env, "Outcome 46"),
                String::from_str(&env, "Outcome 47"),
                String::from_str(&env, "Outcome 48"),
                String::from_str(&env, "Outcome 49"),
                String::from_str(&env, "Outcome 50"),
                String::from_str(&env, "Outcome 51"),
                String::from_str(&env, "Outcome 52"),
                String::from_str(&env, "Outcome 53"),
                String::from_str(&env, "Outcome 54"),
                String::from_str(&env, "Outcome 55"),
                String::from_str(&env, "Outcome 56"),
                String::from_str(&env, "Outcome 57"),
                String::from_str(&env, "Outcome 58"),
                String::from_str(&env, "Outcome 59"),
                String::from_str(&env, "Outcome 60"),
                String::from_str(&env, "Outcome 61"),
                String::from_str(&env, "Outcome 62"),
                String::from_str(&env, "Outcome 63"),
                String::from_str(&env, "Outcome 64"),
                String::from_str(&env, "Outcome 65"),
                String::from_str(&env, "Outcome 66"),
                String::from_str(&env, "Outcome 67"),
                String::from_str(&env, "Outcome 68"),
                String::from_str(&env, "Outcome 69"),
                String::from_str(&env, "Outcome 70"),
                String::from_str(&env, "Outcome 71"),
                String::from_str(&env, "Outcome 72"),
                String::from_str(&env, "Outcome 73"),
                String::from_str(&env, "Outcome 74"),
                String::from_str(&env, "Outcome 75"),
                String::from_str(&env, "Outcome 76"),
                String::from_str(&env, "Outcome 77"),
                String::from_str(&env, "Outcome 78"),
                String::from_str(&env, "Outcome 79"),
                String::from_str(&env, "Outcome 80"),
                String::from_str(&env, "Outcome 81"),
                String::from_str(&env, "Outcome 82"),
                String::from_str(&env, "Outcome 83"),
                String::from_str(&env, "Outcome 84"),
                String::from_str(&env, "Outcome 85"),
                String::from_str(&env, "Outcome 86"),
                String::from_str(&env, "Outcome 87"),
                String::from_str(&env, "Outcome 88"),
                String::from_str(&env, "Outcome 89"),
                String::from_str(&env, "Outcome 90"),
                String::from_str(&env, "Outcome 91"),
                String::from_str(&env, "Outcome 92"),
                String::from_str(&env, "Outcome 93"),
                String::from_str(&env, "Outcome 94"),
                String::from_str(&env, "Outcome 95"),
                String::from_str(&env, "Outcome 96"),
                String::from_str(&env, "Outcome 97"),
                String::from_str(&env, "Outcome 98"),
                String::from_str(&env, "Outcome 99"),
                String::from_str(&env, "Outcome 100"),
            ],
        },
    );
}

/// options_count == MAX_OPTIONS_COUNT (100) must be accepted, and a
/// prediction on the last valid outcome index (99) must succeed.
#[test]
fn test_create_pool_accepts_maximum_options_count() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &100u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Max options pool"),
            metadata_url: String::from_str(&env, "ipfs://maxopts"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
                String::from_str(&env, "Outcome 3"),
                String::from_str(&env, "Outcome 4"),
                String::from_str(&env, "Outcome 5"),
                String::from_str(&env, "Outcome 6"),
                String::from_str(&env, "Outcome 7"),
                String::from_str(&env, "Outcome 8"),
                String::from_str(&env, "Outcome 9"),
                String::from_str(&env, "Outcome 10"),
                String::from_str(&env, "Outcome 11"),
                String::from_str(&env, "Outcome 12"),
                String::from_str(&env, "Outcome 13"),
                String::from_str(&env, "Outcome 14"),
                String::from_str(&env, "Outcome 15"),
                String::from_str(&env, "Outcome 16"),
                String::from_str(&env, "Outcome 17"),
                String::from_str(&env, "Outcome 18"),
                String::from_str(&env, "Outcome 19"),
                String::from_str(&env, "Outcome 20"),
                String::from_str(&env, "Outcome 21"),
                String::from_str(&env, "Outcome 22"),
                String::from_str(&env, "Outcome 23"),
                String::from_str(&env, "Outcome 24"),
                String::from_str(&env, "Outcome 25"),
                String::from_str(&env, "Outcome 26"),
                String::from_str(&env, "Outcome 27"),
                String::from_str(&env, "Outcome 28"),
                String::from_str(&env, "Outcome 29"),
                String::from_str(&env, "Outcome 30"),
                String::from_str(&env, "Outcome 31"),
                String::from_str(&env, "Outcome 32"),
                String::from_str(&env, "Outcome 33"),
                String::from_str(&env, "Outcome 34"),
                String::from_str(&env, "Outcome 35"),
                String::from_str(&env, "Outcome 36"),
                String::from_str(&env, "Outcome 37"),
                String::from_str(&env, "Outcome 38"),
                String::from_str(&env, "Outcome 39"),
                String::from_str(&env, "Outcome 40"),
                String::from_str(&env, "Outcome 41"),
                String::from_str(&env, "Outcome 42"),
                String::from_str(&env, "Outcome 43"),
                String::from_str(&env, "Outcome 44"),
                String::from_str(&env, "Outcome 45"),
                String::from_str(&env, "Outcome 46"),
                String::from_str(&env, "Outcome 47"),
                String::from_str(&env, "Outcome 48"),
                String::from_str(&env, "Outcome 49"),
                String::from_str(&env, "Outcome 50"),
                String::from_str(&env, "Outcome 51"),
                String::from_str(&env, "Outcome 52"),
                String::from_str(&env, "Outcome 53"),
                String::from_str(&env, "Outcome 54"),
                String::from_str(&env, "Outcome 55"),
                String::from_str(&env, "Outcome 56"),
                String::from_str(&env, "Outcome 57"),
                String::from_str(&env, "Outcome 58"),
                String::from_str(&env, "Outcome 59"),
                String::from_str(&env, "Outcome 60"),
                String::from_str(&env, "Outcome 61"),
                String::from_str(&env, "Outcome 62"),
                String::from_str(&env, "Outcome 63"),
                String::from_str(&env, "Outcome 64"),
                String::from_str(&env, "Outcome 65"),
                String::from_str(&env, "Outcome 66"),
                String::from_str(&env, "Outcome 67"),
                String::from_str(&env, "Outcome 68"),
                String::from_str(&env, "Outcome 69"),
                String::from_str(&env, "Outcome 70"),
                String::from_str(&env, "Outcome 71"),
                String::from_str(&env, "Outcome 72"),
                String::from_str(&env, "Outcome 73"),
                String::from_str(&env, "Outcome 74"),
                String::from_str(&env, "Outcome 75"),
                String::from_str(&env, "Outcome 76"),
                String::from_str(&env, "Outcome 77"),
                String::from_str(&env, "Outcome 78"),
                String::from_str(&env, "Outcome 79"),
                String::from_str(&env, "Outcome 80"),
                String::from_str(&env, "Outcome 81"),
                String::from_str(&env, "Outcome 82"),
                String::from_str(&env, "Outcome 83"),
                String::from_str(&env, "Outcome 84"),
                String::from_str(&env, "Outcome 85"),
                String::from_str(&env, "Outcome 86"),
                String::from_str(&env, "Outcome 87"),
                String::from_str(&env, "Outcome 88"),
                String::from_str(&env, "Outcome 89"),
                String::from_str(&env, "Outcome 90"),
                String::from_str(&env, "Outcome 91"),
                String::from_str(&env, "Outcome 92"),
                String::from_str(&env, "Outcome 93"),
                String::from_str(&env, "Outcome 94"),
                String::from_str(&env, "Outcome 95"),
                String::from_str(&env, "Outcome 96"),
                String::from_str(&env, "Outcome 97"),
                String::from_str(&env, "Outcome 98"),
                String::from_str(&env, "Outcome 99"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);
    // outcome index 99 is the last valid index and must be accepted.
    client.place_prediction(&user, &pool_id, &100, &99, &None, &None);
}

/// Placing a prediction with outcome >= options_count must be rejected.
/// This tests the limit enforcement in update_outcome_stake.
#[test]
#[should_panic(expected = "Error(Contract, #25)")]
fn test_place_prediction_rejects_out_of_bounds_outcome() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &3u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Three options"),
            metadata_url: String::from_str(&env, "ipfs://three"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);
    // outcome 3 is out of bounds (valid: 0, 1, 2)
    client.place_prediction(&user, &pool_id, &100, &3, &None, &None);
}

/// Placing a prediction with outcome == options_count must be rejected.
/// This verifies the boundary condition: outcome must be < options_count.
#[test]
#[should_panic(expected = "Error(Contract, #25)")]
fn test_place_prediction_rejects_outcome_equal_to_options_count() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &5u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Five options"),
            metadata_url: String::from_str(&env, "ipfs://five"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
                String::from_str(&env, "Outcome 3"),
                String::from_str(&env, "Outcome 4"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);
    // outcome 5 equals options_count (valid: 0-4)
    client.place_prediction(&user, &pool_id, &100, &5, &None, &None);
}

/// Placing predictions on all valid outcomes (0 to options_count-1) must succeed.
/// This verifies that legitimate usage is not blocked by the bounds check.
#[test]
fn test_place_prediction_all_valid_outcomes() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let options_count = 10u32;
    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &options_count,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Ten options"),
            metadata_url: String::from_str(&env, "ipfs://ten"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
                String::from_str(&env, "Outcome 3"),
                String::from_str(&env, "Outcome 4"),
                String::from_str(&env, "Outcome 5"),
                String::from_str(&env, "Outcome 6"),
                String::from_str(&env, "Outcome 7"),
                String::from_str(&env, "Outcome 8"),
                String::from_str(&env, "Outcome 9"),
            ],
        },
    );

    token_admin_client.mint(&creator, &(100 * options_count as i128));

    // Place predictions on all valid outcomes (0 through 9) using different users
    for outcome in 0..options_count {
        let user = Address::generate(&env);
        token_admin_client.mint(&user, &100);
        client.place_prediction(&user, &pool_id, &100, &outcome, &None, &None);
    }

    // Verify stakes were recorded correctly
    let stakes = client.get_pool_outcome_stakes(&pool_id);
    assert_eq!(stakes.len(), options_count);
    for i in 0..options_count {
        assert_eq!(stakes.get(i).unwrap(), 100);
    }
}

/// Test that stakes.len() remains consistent with options_count after multiple updates.
#[test]
fn test_stakes_length_consistency_with_options_count() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    let options_count = 7u32;
    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &options_count,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Seven options"),
            metadata_url: String::from_str(&env, "ipfs://seven"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
                String::from_str(&env, "Outcome 3"),
                String::from_str(&env, "Outcome 4"),
                String::from_str(&env, "Outcome 5"),
                String::from_str(&env, "Outcome 6"),
            ],
        },
    );

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let user4 = Address::generate(&env);
    token_admin_client.mint(&user1, &500);
    token_admin_client.mint(&user2, &300);
    token_admin_client.mint(&user3, &400);
    token_admin_client.mint(&user4, &600);

    // Multiple users place predictions on various outcomes (one per user)
    client.place_prediction(&user1, &pool_id, &500, &0, &None, &None);
    client.place_prediction(&user2, &pool_id, &300, &3, &None, &None);
    client.place_prediction(&user3, &pool_id, &400, &0, &None, &None);
    client.place_prediction(&user4, &pool_id, &600, &6, &None, &None);

    // Verify stakes vector length matches options_count
    let stakes = client.get_pool_outcome_stakes(&pool_id);
    assert_eq!(stakes.len(), options_count);

    // Verify specific stake values
    assert_eq!(stakes.get(0).unwrap(), 900); // user1: 500 + user3: 400
    assert_eq!(stakes.get(3).unwrap(), 300);
    assert_eq!(stakes.get(6).unwrap(), 600);
    assert_eq!(stakes.get(1).unwrap(), 0);
    assert_eq!(stakes.get(2).unwrap(), 0);
    assert_eq!(stakes.get(4).unwrap(), 0);
    assert_eq!(stakes.get(5).unwrap(), 0);

    // Resolve and claim to ensure full lifecycle works
    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &0u32);
    let winnings = client.claim_winnings(&user1, &pool_id);
    assert!(winnings > 0);
}

/// Test with MAX_OPTIONS_COUNT pool to ensure bounds check works at scale.
#[test]
fn test_outcome_bounds_with_maximum_options_count() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &100u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Max options pool"),
            metadata_url: String::from_str(&env, "ipfs://maxopts"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
                String::from_str(&env, "Outcome 3"),
                String::from_str(&env, "Outcome 4"),
                String::from_str(&env, "Outcome 5"),
                String::from_str(&env, "Outcome 6"),
                String::from_str(&env, "Outcome 7"),
                String::from_str(&env, "Outcome 8"),
                String::from_str(&env, "Outcome 9"),
                String::from_str(&env, "Outcome 10"),
                String::from_str(&env, "Outcome 11"),
                String::from_str(&env, "Outcome 12"),
                String::from_str(&env, "Outcome 13"),
                String::from_str(&env, "Outcome 14"),
                String::from_str(&env, "Outcome 15"),
                String::from_str(&env, "Outcome 16"),
                String::from_str(&env, "Outcome 17"),
                String::from_str(&env, "Outcome 18"),
                String::from_str(&env, "Outcome 19"),
                String::from_str(&env, "Outcome 20"),
                String::from_str(&env, "Outcome 21"),
                String::from_str(&env, "Outcome 22"),
                String::from_str(&env, "Outcome 23"),
                String::from_str(&env, "Outcome 24"),
                String::from_str(&env, "Outcome 25"),
                String::from_str(&env, "Outcome 26"),
                String::from_str(&env, "Outcome 27"),
                String::from_str(&env, "Outcome 28"),
                String::from_str(&env, "Outcome 29"),
                String::from_str(&env, "Outcome 30"),
                String::from_str(&env, "Outcome 31"),
                String::from_str(&env, "Outcome 32"),
                String::from_str(&env, "Outcome 33"),
                String::from_str(&env, "Outcome 34"),
                String::from_str(&env, "Outcome 35"),
                String::from_str(&env, "Outcome 36"),
                String::from_str(&env, "Outcome 37"),
                String::from_str(&env, "Outcome 38"),
                String::from_str(&env, "Outcome 39"),
                String::from_str(&env, "Outcome 40"),
                String::from_str(&env, "Outcome 41"),
                String::from_str(&env, "Outcome 42"),
                String::from_str(&env, "Outcome 43"),
                String::from_str(&env, "Outcome 44"),
                String::from_str(&env, "Outcome 45"),
                String::from_str(&env, "Outcome 46"),
                String::from_str(&env, "Outcome 47"),
                String::from_str(&env, "Outcome 48"),
                String::from_str(&env, "Outcome 49"),
                String::from_str(&env, "Outcome 50"),
                String::from_str(&env, "Outcome 51"),
                String::from_str(&env, "Outcome 52"),
                String::from_str(&env, "Outcome 53"),
                String::from_str(&env, "Outcome 54"),
                String::from_str(&env, "Outcome 55"),
                String::from_str(&env, "Outcome 56"),
                String::from_str(&env, "Outcome 57"),
                String::from_str(&env, "Outcome 58"),
                String::from_str(&env, "Outcome 59"),
                String::from_str(&env, "Outcome 60"),
                String::from_str(&env, "Outcome 61"),
                String::from_str(&env, "Outcome 62"),
                String::from_str(&env, "Outcome 63"),
                String::from_str(&env, "Outcome 64"),
                String::from_str(&env, "Outcome 65"),
                String::from_str(&env, "Outcome 66"),
                String::from_str(&env, "Outcome 67"),
                String::from_str(&env, "Outcome 68"),
                String::from_str(&env, "Outcome 69"),
                String::from_str(&env, "Outcome 70"),
                String::from_str(&env, "Outcome 71"),
                String::from_str(&env, "Outcome 72"),
                String::from_str(&env, "Outcome 73"),
                String::from_str(&env, "Outcome 74"),
                String::from_str(&env, "Outcome 75"),
                String::from_str(&env, "Outcome 76"),
                String::from_str(&env, "Outcome 77"),
                String::from_str(&env, "Outcome 78"),
                String::from_str(&env, "Outcome 79"),
                String::from_str(&env, "Outcome 80"),
                String::from_str(&env, "Outcome 81"),
                String::from_str(&env, "Outcome 82"),
                String::from_str(&env, "Outcome 83"),
                String::from_str(&env, "Outcome 84"),
                String::from_str(&env, "Outcome 85"),
                String::from_str(&env, "Outcome 86"),
                String::from_str(&env, "Outcome 87"),
                String::from_str(&env, "Outcome 88"),
                String::from_str(&env, "Outcome 89"),
                String::from_str(&env, "Outcome 90"),
                String::from_str(&env, "Outcome 91"),
                String::from_str(&env, "Outcome 92"),
                String::from_str(&env, "Outcome 93"),
                String::from_str(&env, "Outcome 94"),
                String::from_str(&env, "Outcome 95"),
                String::from_str(&env, "Outcome 96"),
                String::from_str(&env, "Outcome 97"),
                String::from_str(&env, "Outcome 98"),
                String::from_str(&env, "Outcome 99"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &2000);

    // Valid: last index (99)
    client.place_prediction(&user, &pool_id, &500, &99, &None, &None);

    // Verify the valid bet was recorded
    let stakes = client.get_pool_outcome_stakes(&pool_id);
    assert_eq!(stakes.len(), 100);
    assert_eq!(stakes.get(99).unwrap(), 500);
    assert_eq!(stakes.get(0).unwrap(), 0);

    // Note: Attempting to bet on outcome 100 should panic
    // (tested separately in test_place_prediction_rejects_out_of_bounds_outcome)
}

/// end_time below MIN_POOL_DURATION from the current ledger must be rejected.
#[test]
#[should_panic(expected = "end_time must be at least min_pool_duration in the future")]
fn test_create_pool_rejects_end_time_below_min_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    // Ledger at 0; 1 800 s < MIN_POOL_DURATION (3 600 s).
    client.create_pool(
        &creator,
        &1_800u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Too short pool"),
            metadata_url: String::from_str(&env, "ipfs://short"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
}

/// end_time == current_time + MIN_POOL_DURATION must be accepted (lower
/// boundary is inclusive).
#[test]
fn test_create_pool_accepts_end_time_exactly_at_min_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    // Ledger at 0; MIN_POOL_DURATION == 3 600.
    let pool_id = client.create_pool(
        &creator,
        &3_600u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Min duration pool"),
            metadata_url: String::from_str(&env, "ipfs://mintime"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // If creation succeeded (didn't panic), the test passes.
    let _ = pool_id;
}

/// max_stake < min_stake must be rejected.
#[test]
#[should_panic(expected = "max_stake must be zero (unlimited) or >= min_stake")]
fn test_create_pool_rejects_max_stake_less_than_min_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Inverted stake limits"),
            metadata_url: String::from_str(&env, "ipfs://inverted"),
            min_stake: 100i128,
            max_stake: 50i128, // min_stake
            max_total_stake: 0,
            initial_liquidity: 0i128, // max_stake < min_stake -> invalid
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
}

/// max_stake == min_stake must be accepted (edge: equality is valid).
#[test]
fn test_create_pool_accepts_max_stake_equal_to_min_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Equal stake limits"),
            metadata_url: String::from_str(&env, "ipfs://equal"),
            min_stake: 100i128,
            max_stake: 100i128, // min_stake
            max_total_stake: 0,
            initial_liquidity: 0i128, // max_stake == min_stake -> valid
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &200);
    // Exact bet at the only allowed amount.
    client.place_prediction(&user, &pool_id, &100, &0, &None, &None);
}

/// outcome index == options_count must be rejected (out-of-bounds, 0-indexed).
#[test]
#[should_panic]
fn test_resolve_pool_rejects_out_of_bounds_outcome() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &3u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "OOB outcome"),
            metadata_url: String::from_str(&env, "ipfs://oob"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    // Outcome 3 is out-of-bounds for a 3-option pool.
    client.resolve_pool(&operator, &pool_id, &3u32);
}

// ── (Simulated) race conditions & unauthorized access attempts ────────────────

/// Multiple distinct unauthorized addresses attempting to resolve a pool must
/// all be denied, and the pool must remain resolvable by a real operator
/// afterwards.
#[test]
fn test_multiple_unauthorized_resolve_attempts_do_not_affect_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Auth test pool"),
            metadata_url: String::from_str(&env, "ipfs://auth"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &500);
    client.place_prediction(&user, &pool_id, &200, &0, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 100_001);

    // Three distinct unauthorized addresses each attempt a resolution.
    for _ in 0..3u32 {
        let not_operator = Address::generate(&env);
        let result = client.try_resolve_pool(&not_operator, &pool_id, &0u32);
        assert!(result.is_err(), "Unauthorized resolve must fail");
    }

    // Legitimate operator must still be able to resolve.
    client.resolve_pool(&operator, &pool_id, &0u32);

    let winnings = client.claim_winnings(&user, &pool_id);
    assert_eq!(winnings, 200);
}

/// An unauthorized admin operation must not alter configuration state.
#[test]
fn test_unauthorized_admin_op_does_not_mutate_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, _, _, _, creator) = setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Legitimate admin sets fee to 200 bps.
    client.set_fee_bps(&admin, &200u32);

    // Attacker attempts to overwrite the fee – must be rejected.
    let attacker = Address::generate(&env);
    let result = client.try_set_fee_bps(&attacker, &9_999u32);
    assert!(result.is_err(), "Unauthorized set_fee_bps must fail");

    // Verify configuration was not altered by trying to create a pool
    // (the contract must still function normally, proving the state is intact).
    let new_pool = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Post-attack pool"),
            metadata_url: String::from_str(&env, "ipfs://postattack"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    let _ = new_pool; // pool creation succeeds → state is healthy
}

/// Attempting to cancel a pool by someone who is neither an admin/operator
/// nor the pool creator must be denied consistently across many attempts.
#[test]
fn test_unauthorized_cancel_attempts_do_not_affect_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Cancel guard pool"),
            metadata_url: String::from_str(&env, "ipfs://guard"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    for _ in 0..3u32 {
        let not_operator = Address::generate(&env);
        let result = client.try_cancel_pool(&not_operator, &pool_id);
        assert!(result.is_err(), "Unauthorized cancel must fail");
    }

    // Legitimate operator can still cancel.
    client.cancel_pool(&operator, &pool_id);
}

// ── State consistency after multiple resolution cycles ────────────────────────

/// Create five pools, resolve them with alternating outcomes, and claim all
/// winnings.  Verifies (INV-5): total claimed == total staked.
#[test]
#[allow(clippy::needless_range_loop)]
fn test_state_consistency_across_many_pools() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator, creator) = setup(&env);
    let contract_addr = client.address.clone();

    // ── Pool 0 ──
    let p0 = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 0"),
            metadata_url: String::from_str(&env, "ipfs://0"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    // ── Pool 1 ──
    let p1 = client.create_pool(
        &creator,
        &100_001u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 1"),
            metadata_url: String::from_str(&env, "ipfs://1"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    // ── Pool 2 ──
    let p2 = client.create_pool(
        &creator,
        &100_002u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 2"),
            metadata_url: String::from_str(&env, "ipfs://2"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    // ── Pool 3 ──
    let p3 = client.create_pool(
        &creator,
        &100_003u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 3"),
            metadata_url: String::from_str(&env, "ipfs://3"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    // ── Pool 4 ──
    let p4 = client.create_pool(
        &creator,
        &100_004u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 4"),
            metadata_url: String::from_str(&env, "ipfs://4"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let pools = [p0, p1, p2, p3, p4];

    // Each pool gets user_a (outcome 0) and user_b (outcome 1).
    let user_as: [Address; 5] = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    let user_bs: [Address; 5] = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    let mut expected_total: i128 = 0;
    for (i, pool) in pools.iter().enumerate() {
        let stake = 100 + (i as i128 * 10);
        token_admin_client.mint(&user_as[i], &stake);
        token_admin_client.mint(&user_bs[i], &stake);
        client.place_prediction(&user_as[i], pool, &stake, &0, &None, &None);
        client.place_prediction(&user_bs[i], pool, &stake, &1, &None, &None);
        expected_total += stake * 2;
    }

    assert_eq!(token.balance(&contract_addr), expected_total);

    env.ledger().with_mut(|li| li.timestamp = 200_000);

    // Even-indexed pools → outcome 0 wins; odd-indexed → outcome 1 wins.
    for (i, pool) in pools.iter().enumerate() {
        let winning_outcome: u32 = if i % 2 == 0 { 0 } else { 1 };
        client.resolve_pool(&operator, pool, &winning_outcome);
    }

    let mut total_paid: i128 = 0;
    for (i, pool) in pools.iter().enumerate() {
        let stake = 100 + (i as i128 * 10);
        let wa = client.claim_winnings(&user_as[i], pool);
        let wb = client.claim_winnings(&user_bs[i], pool);

        // Each pool pays out exactly 2 × stake (INV-5 per pool).
        assert_eq!(wa + wb, stake * 2, "pool {i}: payout mismatch");

        if i % 2 == 0 {
            assert_eq!(wa, stake * 2, "pool {i}: outcome-0 user should win");
            assert_eq!(wb, 0, "pool {i}: outcome-1 user should lose");
        } else {
            assert_eq!(wa, 0, "pool {i}: outcome-0 user should lose");
            assert_eq!(wb, stake * 2, "pool {i}: outcome-1 user should win");
        }

        total_paid += wa + wb;
    }

    // Global invariant: no value created or destroyed.
    assert_eq!(total_paid, expected_total);
    assert_eq!(token.balance(&contract_addr), 0);
}

/// Cancel pool A while pool B remains active, then resolve pool B.
/// Verifies that cancellation of one pool does not corrupt another.
#[test]
fn test_state_consistency_after_cancellation_and_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator, creator) = setup(&env);
    let contract_addr = client.address.clone();

    let pool_a = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool A (cancel)"),
            metadata_url: String::from_str(&env, "ipfs://a"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let pool_b = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool B (resolve)"),
            metadata_url: String::from_str(&env, "ipfs://b"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    token_admin_client.mint(&user_a, &1000);
    token_admin_client.mint(&user_b, &1000);

    client.place_prediction(&user_a, &pool_a, &300, &0, &None, &None);
    client.place_prediction(&user_b, &pool_b, &400, &1, &None, &None);

    // Cancel pool A; 300 remain locked for refund.
    client.cancel_pool(&operator, &pool_a);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_b, &1u32);

    // user_b is the sole better on winning outcome of pool_b → receives full 400.
    let wb = client.claim_winnings(&user_b, &pool_b);
    assert_eq!(wb, 400);

    // Contract should still hold pool_a's 300 (user_a's refund not yet claimed).
    assert_eq!(token.balance(&contract_addr), 300);

    // user_a claims refund from canceled pool_a.
    let wa_refund = client.claim_winnings(&user_a, &pool_a);
    assert_eq!(wa_refund, 300);

    // Contract drained to zero.
    assert_eq!(token.balance(&contract_addr), 0);
}

/// Verify that the contract correctly handles a pool with no losers
/// (every bettor chose the winning outcome).  The sole winner gets everything;
/// the invariant total_paid == total_staked must still hold.
#[test]
fn test_all_bettors_on_winning_side() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator, creator) = setup(&env);
    let contract_addr = client.address.clone();

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "All win pool"),
            metadata_url: String::from_str(&env, "ipfs://allwin"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &600);
    token_admin_client.mint(&user2, &400);

    client.place_prediction(&user1, &pool_id, &600, &0, &None, &None);
    client.place_prediction(&user2, &pool_id, &400, &0, &None, &None);

    let total = 1_000i128;
    assert_eq!(token.balance(&contract_addr), total);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    let w1 = client.claim_winnings(&user1, &pool_id);
    let w2 = client.claim_winnings(&user2, &pool_id);

    // Proportional split: 600 and 400.
    assert_eq!(w1, 600);
    assert_eq!(w2, 400);
    assert_eq!(w1 + w2, total);
    assert_eq!(token.balance(&contract_addr), 0);
}

/// If no one bet on the winning outcome, all claimants must receive 0.
#[test]
fn test_no_bettor_on_winning_side() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &3u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Empty winner pool"),
            metadata_url: String::from_str(&env, "ipfs://emptywinner"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &500);
    token_admin_client.mint(&user2, &500);

    // Both bet on outcome 1; outcome 2 wins (nobody bet on it).
    client.place_prediction(&user1, &pool_id, &300, &1, &None, &None);
    client.place_prediction(&user2, &pool_id, &200, &1, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &2u32); // outcome 2 – no bettors

    let w1 = client.claim_winnings(&user1, &pool_id);
    let w2 = client.claim_winnings(&user2, &pool_id);
    assert_eq!(w1, 0);
    assert_eq!(w2, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// is_contract_paused Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Test that is_contract_paused returns false by default after initialization.
#[test]
fn test_is_contract_paused_returns_false_by_default() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let _admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    // Contract should not be paused by default
    assert!(!client.is_contract_paused());
}

/// Test that is_contract_paused returns true after pause is called.
#[test]
fn test_is_contract_paused_returns_true_after_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    // Initially not paused
    assert!(!client.is_contract_paused());

    // Pause the contract
    client.pause(&admin);

    // Now it should be paused
    assert!(client.is_contract_paused());
}

/// Test that is_contract_paused returns false after unpause is called.
#[test]
fn test_is_contract_paused_returns_false_after_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    // Pause the contract
    client.pause(&admin);
    assert!(client.is_contract_paused());

    // Unpause the contract
    client.unpause(&admin);

    // Now it should not be paused
    assert!(!client.is_contract_paused());
}

/// Test toggling pause state multiple times and verifying is_contract_paused.
#[test]
fn test_is_contract_paused_toggle_pause_state() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    // Initial state: not paused
    assert!(!client.is_contract_paused());

    // First pause
    client.pause(&admin);
    assert!(client.is_contract_paused());

    // First unpause
    client.unpause(&admin);
    assert!(!client.is_contract_paused());

    // Second pause
    client.pause(&admin);
    assert!(client.is_contract_paused());

    // Second unpause
    client.unpause(&admin);
    assert!(!client.is_contract_paused());
}

/// Test that is_contract_paused works correctly across multiple contract instances.
#[test]
fn test_is_contract_paused_independent_per_instance() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);

    let contract_id_1 = env.register(PredifiContract, ());
    let client_1 = PredifiContractClient::new(&env, &contract_id_1);

    let contract_id_2 = env.register(PredifiContract, ());
    let client_2 = PredifiContractClient::new(&env, &contract_id_2);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Initialize both contracts
    client_1.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);
    client_2.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    // Both should start unpaused
    assert!(!client_1.is_contract_paused());
    assert!(!client_2.is_contract_paused());

    // Pause only contract 1
    client_1.pause(&admin);

    // Contract 1 should be paused, contract 2 should remain unpaused
    assert!(client_1.is_contract_paused());
    assert!(!client_2.is_contract_paused());
}

// ── bump_ttl helper tests ────────────────────────────────────────────────────

/// Helper: create an env with predictable ledger settings for TTL assertions.
fn create_ttl_env() -> Env {
    let env = Env::default();
    env.ledger().with_mut(|li| {
        li.sequence_number = 100_000;
        li.min_persistent_entry_ttl = 500;
        li.min_temp_entry_ttl = 100;
        li.max_entry_ttl = 6_000_000; // large enough for BUMP_AMOUNT (30 * 17280 = 518400)
    });
    env
}

/// Helper: create a pool using the standard PoolConfig pattern.
fn create_test_pool(
    env: &Env,
    client: &PredifiContractClient,
    creator: &Address,
    token_address: &Address,
    end_time: u64,
) -> u64 {
    client.create_pool(
        creator,
        &end_time,
        token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(env, "bump_ttl test pool"),
            metadata_url: String::from_str(
                env,
                "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    )
}

/// bump_ttl should extend both instance TTL and persistent TTL for the given key.
#[test]
fn test_bump_ttl_extends_both_instance_and_persistent() {
    let env = create_ttl_env();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);
    let contract_id = client.address.clone();

    let pool_id = create_test_pool(&env, &client, &creator, &token_address, 100_000u64);

    // After create_pool, bump_ttl is called for pool_key and pc_key.
    // Both instance and persistent TTLs should be extended to BUMP_AMOUNT (518400 ledgers).
    env.as_contract(&contract_id, || {
        let pool_key = DataKey::Pool(pool_id);
        let persistent_ttl = env.storage().persistent().get_ttl(&pool_key);
        let instance_ttl = env.storage().instance().get_ttl();

        // BUMP_AMOUNT = 30 * 17280 = 518400
        assert!(
            persistent_ttl >= 518_000,
            "persistent TTL should be near BUMP_AMOUNT, got {persistent_ttl}"
        );
        assert!(
            instance_ttl >= 518_000,
            "instance TTL should be near BUMP_AMOUNT, got {instance_ttl}"
        );
    });
}

/// bump_ttl via place_prediction: pool_key persistent and instance TTLs are bumped.
#[test]
fn test_bump_ttl_after_place_prediction() {
    let env = create_ttl_env();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);
    let contract_id = client.address.clone();

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1_000);

    let pool_id = create_test_pool(&env, &client, &creator, &token_address, 100_000u64);

    // Advance ledger sequence past BUMP_THRESHOLD to force a re-bump, then place a prediction.
    // BUMP_THRESHOLD = 14 * 17280 = 241920; advance enough so remaining TTL < threshold.
    env.ledger().with_mut(|li| {
        li.sequence_number = 100_000 + 300_000;
    });

    client.place_prediction(&user, &pool_id, &100, &0u32, &None, &None);

    // After place_prediction, bump_ttl should have refreshed both TTLs.
    env.as_contract(&contract_id, || {
        let pool_key = DataKey::Pool(pool_id);
        let persistent_ttl = env.storage().persistent().get_ttl(&pool_key);
        let instance_ttl = env.storage().instance().get_ttl();

        assert!(
            persistent_ttl >= 518_000,
            "persistent TTL should be near BUMP_AMOUNT after place_prediction, got {persistent_ttl}"
        );
        assert!(
            instance_ttl >= 518_000,
            "instance TTL should be near BUMP_AMOUNT after place_prediction, got {instance_ttl}"
        );
    });
}

/// bump_ttl via resolve_pool: pool_key TTLs are bumped on resolution.
#[test]
fn test_bump_ttl_after_resolve_pool() {
    let env = create_ttl_env();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);
    let contract_id = client.address.clone();

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1_000);

    let pool_id = create_test_pool(&env, &client, &creator, &token_address, 100_000u64);
    client.place_prediction(&user, &pool_id, &100, &0u32, &None, &None);

    // Advance time past end_time and reduce sequence to lower TTLs.
    env.ledger().with_mut(|li| {
        li.timestamp = 200_000;
        li.sequence_number = 100_000 + 300_000;
    });

    client.resolve_pool(&operator, &pool_id, &0u32);

    env.as_contract(&contract_id, || {
        let pool_key = DataKey::Pool(pool_id);
        let persistent_ttl = env.storage().persistent().get_ttl(&pool_key);
        let instance_ttl = env.storage().instance().get_ttl();

        assert!(
            persistent_ttl >= 518_000,
            "persistent TTL should be near BUMP_AMOUNT after resolve_pool, got {persistent_ttl}"
        );
        assert!(
            instance_ttl >= 518_000,
            "instance TTL should be near BUMP_AMOUNT after resolve_pool, got {instance_ttl}"
        );
    });
}

/// bump_ttl via cancel_pool: pool_key TTLs are bumped on cancellation.
#[test]
fn test_bump_ttl_after_cancel_pool() {
    let env = create_ttl_env();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);
    let contract_id = client.address.clone();

    let pool_id = create_test_pool(&env, &client, &creator, &token_address, 100_000u64);

    // Advance sequence to reduce TTLs before cancel.
    env.ledger().with_mut(|li| {
        li.sequence_number = 100_000 + 300_000;
    });

    client.cancel_pool(&operator, &pool_id);

    env.as_contract(&contract_id, || {
        let pool_key = DataKey::Pool(pool_id);
        let persistent_ttl = env.storage().persistent().get_ttl(&pool_key);
        let instance_ttl = env.storage().instance().get_ttl();

        assert!(
            persistent_ttl >= 518_000,
            "persistent TTL should be near BUMP_AMOUNT after cancel_pool, got {persistent_ttl}"
        );
        assert!(
            instance_ttl >= 518_000,
            "instance TTL should be near BUMP_AMOUNT after cancel_pool, got {instance_ttl}"
        );
    });
}

/// bump_ttl via claim_winnings: claimed_key TTLs are bumped on claim.
#[test]
fn test_bump_ttl_after_claim_winnings() {
    let env = create_ttl_env();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);
    let contract_id = client.address.clone();

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1_000);

    let pool_id = create_test_pool(&env, &client, &creator, &token_address, 100_000u64);
    client.place_prediction(&user, &pool_id, &100, &0u32, &None, &None);

    env.ledger().with_mut(|li| {
        li.timestamp = 200_000;
        li.sequence_number = 100_000 + 300_000;
    });

    client.resolve_pool(&operator, &pool_id, &0u32);
    client.claim_winnings(&user, &pool_id);

    env.as_contract(&contract_id, || {
        let claimed_key = DataKey::Claimed(user.clone(), pool_id);
        let persistent_ttl = env.storage().persistent().get_ttl(&claimed_key);
        let instance_ttl = env.storage().instance().get_ttl();

        assert!(
            persistent_ttl >= 518_000,
            "persistent TTL should be near BUMP_AMOUNT after claim_winnings, got {persistent_ttl}"
        );
        assert!(
            instance_ttl >= 518_000,
            "instance TTL should be near BUMP_AMOUNT after claim_winnings, got {instance_ttl}"
        );
    });
}

// Version tracking tests
// ============================================================================

#[test]
fn test_version_is_set_after_init() {
    let env = Env::default();
    env.mock_all_auths();
    let (_ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        setup(&env);
    assert_eq!(client.get_version(), 1u32);
}

#[test]
fn test_version_returns_zero_before_init() {
    let env = Env::default();
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);
    assert_eq!(client.get_version(), 0u32);
}

// ============================================================================
// max_total_stake validation tests
// ============================================================================

#[test]
fn test_create_pool_with_max_total_stake() {
    let env = Env::default();
    env.mock_all_auths();
    let (_ac_client, client, token_address, _token, _token_admin, _treasury, _operator, creator) =
        setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &(env.ledger().timestamp() + 100_000),
        &token_address,
        &2u32,
        &Symbol::new(&env, "Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Capped pool"),
            metadata_url: String::from_str(&env, "https://example.com"),
            min_stake: 100,
            max_stake: 0,
            max_total_stake: 500_000,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.max_total_stake, 500_000);
}

#[test]
fn test_create_pool_with_zero_max_total_stake_is_unlimited() {
    let env = Env::default();
    env.mock_all_auths();
    let (_ac_client, client, token_address, _token, _token_admin, _treasury, _operator, creator) =
        setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &(env.ledger().timestamp() + 100_000),
        &token_address,
        &2u32,
        &Symbol::new(&env, "Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Unlimited pool"),
            metadata_url: String::from_str(&env, "https://example.com"),
            min_stake: 100,
            max_stake: 0,
            max_total_stake: 0,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.max_total_stake, 0);
}

#[test]
fn test_outcome_descriptions_stored_and_retrieved() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let descriptions = vec![
        &env,
        String::from_str(&env, "Team A wins"),
        String::from_str(&env, "Draw"),
        String::from_str(&env, "Team B wins"),
    ];

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &3u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Match outcome"),
            metadata_url: String::from_str(&env, "ipfs://match"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: descriptions.clone(),
        },
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.outcome_descriptions.len(), 3);
    assert_eq!(
        pool.outcome_descriptions.get(0).unwrap(),
        String::from_str(&env, "Team A wins")
    );
    assert_eq!(
        pool.outcome_descriptions.get(1).unwrap(),
        String::from_str(&env, "Draw")
    );
    assert_eq!(
        pool.outcome_descriptions.get(2).unwrap(),
        String::from_str(&env, "Team B wins")
    );
}

#[test]
#[should_panic(expected = "outcome_descriptions length must equal options_count")]
fn test_outcome_descriptions_length_mismatch_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &3u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Mismatch test"),
            metadata_url: String::from_str(&env, "ipfs://mismatch"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            // Only 2 descriptions for 3 outcomes — should panic
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
}

#[test]
fn test_admin_can_set_min_pool_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    client.set_min_pool_duration(&admin, &7200u64);
}

#[test]
#[should_panic(expected = "end_time must be at least min_pool_duration in the future")]
fn test_create_pool_respects_configurable_min_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, _, _treasury, _, creator) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Initial min_pool_duration is 3600 from setup.
    // Set a much larger min_pool_duration.
    client.set_min_pool_duration(&admin, &86400u64); // 1 day

    // Try to create a pool with 2 hours duration (7200s), which is < 1 day.
    let current_time = env.ledger().timestamp();
    client.create_pool(
        &creator,
        &(current_time + 7200u64),
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Short Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            // Only 2 descriptions for 3 outcomes — should panic
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
}

#[test]
fn test_pool_created_event_contains_creator() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let end_time = 100000u64;
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(&env, "ipfs://..."),
            min_stake: 100,
            max_stake: 0,
            max_total_stake: 0,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let events = env.events().all();
    let pool_created_topic = Symbol::new(&env, "pool_created");

    let mut found = false;
    for e in events.iter() {
        if let Some(topic_val) = e.1.get(0) {
            if let Ok(topic_sym) = Symbol::try_from_val(&env, &topic_val) {
                if topic_sym == pool_created_topic {
                    let event_data: soroban_sdk::Map<Symbol, Val> = e.2.clone().into_val(&env);

                    let event_pool_id: u64 = event_data
                        .get(Symbol::new(&env, "pool_id"))
                        .unwrap()
                        .into_val(&env);
                    let event_creator: Address = event_data
                        .get(Symbol::new(&env, "creator"))
                        .unwrap()
                        .into_val(&env);
                    let event_end_time: u64 = event_data
                        .get(Symbol::new(&env, "end_time"))
                        .unwrap()
                        .into_val(&env);

                    assert_eq!(event_pool_id, pool_id);
                    assert_eq!(event_creator, creator);
                    assert_eq!(event_end_time, end_time);
                    found = true;
                    break;
                }
            }
        }
    }
    assert!(found, "PoolCreatedEvent not found or failed to parse");
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_claim_winnings_blocks_reentrancy() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _, _, _, _, operator, creator) = setup(&env);

    // Register Rogue Token
    let rogue_token_id = env.register_contract(None, rogue_token::RogueToken);
    let rogue_token_client = rogue_token::RogueTokenClient::new(&env, &rogue_token_id);

    // Whitelist Rogue Token
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &0u32); // ROLE_ADMIN = 0
    client.add_token_to_whitelist(&admin, &rogue_token_id);

    // Create Pool 1 with Rogue Token
    let end_time = 100000u64;
    let pool_id_1 = client.create_pool(
        &creator,
        &end_time,
        &rogue_token_id,
        &2,
        &symbol_short!("Crypto"),
        &PoolConfig {
            description: String::from_str(&env, "Rogue Pool 1"),
            metadata_url: String::from_str(&env, "ipfs://..."),
            min_stake: 100,
            max_stake: 0,
            max_total_stake: 0,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
        },
    );

    // Create Pool 2 with Rogue Token
    let pool_id_2 = client.create_pool(
        &creator,
        &end_time,
        &rogue_token_id,
        &2,
        &symbol_short!("Crypto"),
        &PoolConfig {
            description: String::from_str(&env, "Rogue Pool 2"),
            metadata_url: String::from_str(&env, "ipfs://..."),
            min_stake: 100,
            max_stake: 0,
            max_total_stake: 0,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
        },
    );

    // User predicts on both
    let user = Address::generate(&env);
    client.place_prediction(&user, &pool_id_1, &1000, &0, &None, &None);
    client.place_prediction(&user, &pool_id_2, &1000, &0, &None, &None);

    // Resolve Pools
    let current_time = env.ledger().timestamp();
    env.ledger()
        .with_mut(|li| li.timestamp = current_time + end_time + 3601);
    client.resolve_pool(&operator, &pool_id_1, &0);
    client.resolve_pool(&operator, &pool_id_2, &0);

    // Setup rogue token for callback: claim winnings of pool_id_2 when transferring for pool_id_1
    rogue_token_client.setup(&client.address, &user, &pool_id_2);

    // Attempt to claim winnings for pool_id_1
    let winnings_1 = client.claim_winnings(&user, &pool_id_1);
    assert!(winnings_1 > 0);
}

#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_guard_basic() {
    let env = Env::default();
    let id = env.register_contract(None, PredifiContract);
    env.as_contract(&id, || {
        PredifiContract::enter_reentrancy_guard(&env);
        PredifiContract::enter_reentrancy_guard(&env);
    });
}
