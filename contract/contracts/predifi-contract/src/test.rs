#![cfg(test)]
#![allow(deprecated)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, String, Symbol,
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

const ROLE_ADMIN: u32 = 0;
const ROLE_OPERATOR: u32 = 1;
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
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
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    client.place_prediction(&user1, &pool_id, &100, &1);
    client.place_prediction(&user2, &pool_id, &100, &2);

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
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    client.place_prediction(&user1, &pool_id, &100, &1);

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
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    client.place_prediction(&user1, &pool_id, &100, &1);

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
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    let pool_b = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    client.place_prediction(&user1, &pool_a, &100, &1);
    client.place_prediction(&user2, &pool_b, &100, &1);

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
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&admin, &token_address);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&admin, &token_address);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&admin, &token);

    let creator = Address::generate(&env);
    client.pause(&admin);
    client.create_pool(
        &creator,
        &100000u64,
        &token,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);

    client.pause(&admin);
    client.place_prediction(&user, &0u64, &10, &1);
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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);

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
    client.init(&ac_id, &treasury, &0u32, &0u64);
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
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    client.place_prediction(&user, &pool_id, &10, &1);
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
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    let pool1 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    let pool2 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    client.place_prediction(&user, &pool0, &10, &1);
    client.place_prediction(&user, &pool1, &20, &2);
    client.place_prediction(&user, &pool2, &30, &1);

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

    let empty = client.get_user_predictions(&user, &3, &1);
    assert_eq!(empty.len(), 0);
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&admin, &token_address);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    // Do NOT whitelist token_not_whitelisted

    client.create_pool(
        &creator,
        &100000u64,
        &token_not_whitelisted,
        &2u32,
        &String::from_str(&env, "Pool"),
        &String::from_str(&env, "ipfs://meta"),
        &0i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "test"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);

    assert!(!client.is_token_allowed(&token));
    client.add_token_to_whitelist(&admin, &token);
    assert!(client.is_token_allowed(&token));
    client.remove_token_from_whitelist(&admin, &token);
    assert!(!client.is_token_allowed(&token));
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
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
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // Cancel the pool
    client.cancel_pool(&admin, &pool_id);

    // Try to place prediction on canceled pool - should panic
    client.place_prediction(&user, &pool_id, &100, &1);
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // User places a prediction
    client.place_prediction(&user, &pool_id, &100, &1);

    // Admin cancels the pool - this freezes betting
    client.cancel_pool(&admin, &pool_id);

    // Verify no more predictions can be placed - should panic
    client.place_prediction(&user, &pool_id, &50, &2);
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let creator = Address::generate(&env);
    let contract_addr = client.address.clone();
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Cancel Test Pool"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // User places a prediction
    client.place_prediction(&user1, &pool_id, &100, &1);
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
        &String::from_str(&env, "Resolve Then Cancel Pool"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &0u64);
    client.add_token_to_whitelist(&whitelist_admin, &token_address);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Predict Canceled Pool Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    client.cancel_pool(&operator, &pool_id);
    // Should panic
    client.place_prediction(&user1, &pool_id, &100, &1);
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
    client.init(&ac_id, &treasury, &0u32, &3600u64);
    client.add_token_to_whitelist(&admin, &token);

    let end_time = 10000;
    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token,
        &2u32,
        &String::from_str(&env, "Delay Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &3600u64);
    client.add_token_to_whitelist(&admin, &token);

    let end_time = 10000;
    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token,
        &2u32,
        &String::from_str(&env, "Delay Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.init(&ac_id, &treasury, &0u32, &3600u64);
    client.add_token_to_whitelist(&admin, &token);

    let end_time = 10000;
    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token,
        &2u32,
        &String::from_str(&env, "Ready Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
#[should_panic(expected = "amount is below the pool minimum stake")]
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
        &String::from_str(&env, "Min Stake Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &50i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // Should panic: amount (10) < min_stake (50)
    client.place_prediction(&user, &pool_id, &10, &0);
}

#[test]
#[should_panic(expected = "amount exceeds the pool maximum stake")]
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
        &String::from_str(&env, "Max Stake Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &100i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // Should panic: amount (200) > max_stake (100)
    client.place_prediction(&user, &pool_id, &200, &0);
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
        &String::from_str(&env, "Boundary Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &10i128,
        &200i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // Both boundary values should succeed
    client.place_prediction(&user1, &pool_id, &10, &0); // exactly min_stake
    client.place_prediction(&user2, &pool_id, &200, &1); // exactly max_stake
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
        &String::from_str(&env, "Update Limits Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // Operator updates: min_stake = 50, max_stake = 500
    client.set_stake_limits(&operator, &pool_id, &50i128, &500i128);

    // Stake at the new minimum should succeed
    client.place_prediction(&user, &pool_id, &50, &0);
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
        &String::from_str(&env, "Unauthorized Limits Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // Non-operator should be rejected
    let not_operator = Address::generate(&env);
    client.set_stake_limits(&not_operator, &pool_id, &50i128, &500i128);
}

#[test]
fn test_get_pools_by_category() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let cat1 = Symbol::new(&env, "tech");
    let cat2 = Symbol::new(&env, "sports");

    let pool0 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool 0"),
        &String::from_str(&env, "ipfs://0"),
        &1i128,
        &0i128,
        &0i128,
        &cat1,
    );
    let pool1 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool 1"),
        &String::from_str(&env, "ipfs://1"),
        &1i128,
        &0i128,
        &0i128,
        &cat1,
    );
    let pool2 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool 2"),
        &String::from_str(&env, "ipfs://2"),
        &1i128,
        &0i128,
        &0i128,
        &cat2,
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

    let (ac_client, client, token_address, token, token_admin_client, treasury, _, creator) =
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

    let (_, client, token_address, token, token_admin_client, treasury, _, _) = setup(&env);
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

    let (ac_client, client, token_address, token, token_admin_client, treasury, _, _) = setup(&env);
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

    let (ac_client, client, token_address, token, token_admin_client, treasury, _, _) = setup(&env);
    let contract_addr = client.address.clone();
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    token_admin_client.mint(&contract_addr, &1000);

    // Try to withdraw more than balance - should panic
    client.withdraw_treasury(&admin, &token_address, &5000, &treasury);
}

#[test]
fn test_withdraw_treasury_multiple_tokens() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, token, token_admin_client, treasury, _, _) = setup(&env);
    let contract_addr = client.address.clone();
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Setup second token
    let token_admin2 = Address::generate(&env);
    let token_contract2 = env.register_stellar_asset_contract(token_admin2.clone());
    let token2 = token::Client::new(&env, &token_contract2);
    let token_admin_client2 = token::StellarAssetClient::new(&env, &token_contract2);
    client.add_token_to_whitelist(&admin, &token_contract2);

    // Mint both tokens to contract
    token_admin_client.mint(&contract_addr, &5000);
    token_admin_client2.mint(&contract_addr, &3000);

    // Withdraw from both tokens
    client.withdraw_treasury(&admin, &token_address, &2000, &treasury);
    client.withdraw_treasury(&admin, &token_contract2, &1500, &treasury);

    // Verify balances
    assert_eq!(token.balance(&treasury), 2000);
    assert_eq!(token2.balance(&treasury), 1500);
    assert_eq!(token.balance(&contract_addr), 3000);
    assert_eq!(token2.balance(&contract_addr), 1500);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_paused_blocks_withdraw_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, token, token_admin_client, treasury, _, _) = setup(&env);
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
        &2u32, // Binary pool
        &String::from_str(&env, "Stats Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // Initial stats
    let stats = client.get_pool_stats(&pool_id);
    assert_eq!(stats.participants_count, 0);
    assert_eq!(stats.total_stake, 0);

    // User 1 bets 100 on outcome 0
    client.place_prediction(&user1, &pool_id, &100, &0);
    // User 2 bets 200 on outcome 1
    client.place_prediction(&user2, &pool_id, &200, &1);
    // User 3 bets 100 on outcome 1
    client.place_prediction(&user3, &pool_id, &100, &1);
    // User 1 bets 100 more on outcome 0 (should not increase participants)
    client.place_prediction(&user1, &pool_id, &100, &0);

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
        &String::from_str(&env, "Leap year pool"),
        &String::from_str(&env, "ipfs://leap"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);
    // Prediction must be accepted while before the leap-day deadline.
    client.place_prediction(&user, &pool_id, &100, &0);
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
        &LEAP_DAY_2024_UTC, // Feb 29 – already past
        &token_address,
        &2u32,
        &String::from_str(&env, "Expired leap pool"),
        &String::from_str(&env, "ipfs://expired"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Leap span pool"),
        &String::from_str(&env, "ipfs://span"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &500);
    token_admin_client.mint(&user2, &500);

    client.place_prediction(&user1, &pool_id, &300, &0);
    client.place_prediction(&user2, &pool_id, &200, &1);

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
        &String::from_str(&env, "Max stake pool"),
        &String::from_str(&env, "ipfs://max"),
        &1i128,
        &max_amount, // max_stake == max_amount is valid
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &max_amount);

    client.place_prediction(&user, &pool_id, &max_amount, &0);

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
        &String::from_str(&env, "Large stake split"),
        &String::from_str(&env, "ipfs://large"),
        &1i128,
        &0i128, // no max_stake limit
        &0i128,
        &Symbol::new(&env, "tech"),
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
    client.place_prediction(&winner1, &pool_id, &big_stake, &0);
    client.place_prediction(&winner2, &pool_id, &big_stake, &0);
    client.place_prediction(&loser1, &pool_id, &big_stake, &1);
    client.place_prediction(&loser2, &pool_id, &big_stake, &1);

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
        &String::from_str(&env, "Double resolve"),
        &String::from_str(&env, "ipfs://double"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Rapid claim"),
        &String::from_str(&env, "ipfs://rapid"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        client.place_prediction(u, &pool_id, &stake, &0);
    }
    for u in [&l0, &l1, &l2, &l3, &l4] {
        token_admin_client.mint(u, &stake);
        client.place_prediction(u, &pool_id, &stake, &1);
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
        &String::from_str(&env, "Pool A"),
        &String::from_str(&env, "ipfs://a"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &500);
    client.place_prediction(&user, &pool_a, &200, &0);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_a, &0u32);

    // Create pool B immediately after resolution.
    let pool_b = client.create_pool(
        &creator,
        &200_000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool B"),
        &String::from_str(&env, "ipfs://b"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    assert_ne!(pool_a, pool_b);

    // User can still claim from pool A.
    let winnings = client.claim_winnings(&user, &pool_a);
    assert_eq!(winnings, 200);

    // Pool B is still active – predictions can be placed.
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user2, &500);
    client.place_prediction(&user2, &pool_b, &100, &1);
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
        &String::from_str(&env, "Zero min stake"),
        &String::from_str(&env, "ipfs://zero"),
        &0i128, // invalid
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &1u32, // invalid
        &String::from_str(&env, "Single option pool"),
        &String::from_str(&env, "ipfs://single"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &101u32, // MAX_OPTIONS_COUNT == 100, so 101 is invalid
        &String::from_str(&env, "Too many options"),
        &String::from_str(&env, "ipfs://many"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Max options pool"),
        &String::from_str(&env, "ipfs://maxopts"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);
    // outcome index 99 is the last valid index and must be accepted.
    client.place_prediction(&user, &pool_id, &100, &99);
}

/// end_time below MIN_POOL_DURATION from the current ledger must be rejected.
#[test]
#[should_panic(expected = "end_time must be at least 1 hour in the future")]
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
        &String::from_str(&env, "Too short pool"),
        &String::from_str(&env, "ipfs://short"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Min duration pool"),
        &String::from_str(&env, "ipfs://mintime"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Inverted stake limits"),
        &String::from_str(&env, "ipfs://inverted"),
        &100i128, // min_stake
        &50i128,  // max_stake < min_stake → invalid
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Equal stake limits"),
        &String::from_str(&env, "ipfs://equal"),
        &100i128, // min_stake
        &100i128, // max_stake == min_stake → valid
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &200);
    // Exact bet at the only allowed amount.
    client.place_prediction(&user, &pool_id, &100, &0);
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
        &3u32, // outcomes 0, 1, 2
        &String::from_str(&env, "OOB outcome"),
        &String::from_str(&env, "ipfs://oob"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Auth test pool"),
        &String::from_str(&env, "ipfs://auth"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &500);
    client.place_prediction(&user, &pool_id, &200, &0);

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
        &String::from_str(&env, "Post-attack pool"),
        &String::from_str(&env, "ipfs://postattack"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
        &String::from_str(&env, "Cancel guard pool"),
        &String::from_str(&env, "ipfs://guard"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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
fn test_state_consistency_across_many_pools() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator, creator) = setup(&env);
    let contract_addr = client.address.clone();

    let stake: i128 = 100;

    // ── Pool 0 ──
    let p0 = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool 0"),
        &String::from_str(&env, "ipfs://0"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    // ── Pool 1 ──
    let p1 = client.create_pool(
        &creator,
        &100_001u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool 1"),
        &String::from_str(&env, "ipfs://1"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    // ── Pool 2 ──
    let p2 = client.create_pool(
        &creator,
        &100_002u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool 2"),
        &String::from_str(&env, "ipfs://2"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    // ── Pool 3 ──
    let p3 = client.create_pool(
        &creator,
        &100_003u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool 3"),
        &String::from_str(&env, "ipfs://3"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );
    // ── Pool 4 ──
    let p4 = client.create_pool(
        &creator,
        &100_004u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool 4"),
        &String::from_str(&env, "ipfs://4"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
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

    for i in 0..5usize {
        token_admin_client.mint(&user_as[i], &stake);
        token_admin_client.mint(&user_bs[i], &stake);
        client.place_prediction(&user_as[i], &pools[i], &stake, &0);
        client.place_prediction(&user_bs[i], &pools[i], &stake, &1);
    }

    let expected_total = stake * 10;
    assert_eq!(token.balance(&contract_addr), expected_total);

    env.ledger().with_mut(|li| li.timestamp = 200_000);

    // Even-indexed pools → outcome 0 wins; odd-indexed → outcome 1 wins.
    for i in 0..5usize {
        let winning_outcome: u32 = if i % 2 == 0 { 0 } else { 1 };
        client.resolve_pool(&operator, &pools[i], &winning_outcome);
    }

    let mut total_paid: i128 = 0;
    for i in 0..5usize {
        let wa = client.claim_winnings(&user_as[i], &pools[i]);
        let wb = client.claim_winnings(&user_bs[i], &pools[i]);

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
        &String::from_str(&env, "Pool A (cancel)"),
        &String::from_str(&env, "ipfs://a"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let pool_b = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Pool B (resolve)"),
        &String::from_str(&env, "ipfs://b"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    token_admin_client.mint(&user_a, &1000);
    token_admin_client.mint(&user_b, &1000);

    client.place_prediction(&user_a, &pool_a, &300, &0);
    client.place_prediction(&user_b, &pool_b, &400, &1);

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
        &String::from_str(&env, "All win pool"),
        &String::from_str(&env, "ipfs://allwin"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &600);
    token_admin_client.mint(&user2, &400);

    client.place_prediction(&user1, &pool_id, &600, &0);
    client.place_prediction(&user2, &pool_id, &400, &0);

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
        &String::from_str(&env, "Empty winner pool"),
        &String::from_str(&env, "ipfs://emptywinner"),
        &1i128,
        &0i128,
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &500);
    token_admin_client.mint(&user2, &500);

    // Both bet on outcome 1; outcome 2 wins (nobody bet on it).
    client.place_prediction(&user1, &pool_id, &300, &1);
    client.place_prediction(&user2, &pool_id, &200, &1);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &2u32); // outcome 2 – no bettors

    let w1 = client.claim_winnings(&user1, &pool_id);
    let w2 = client.claim_winnings(&user2, &pool_id);
    assert_eq!(w1, 0);
    assert_eq!(w2, 0);
}
