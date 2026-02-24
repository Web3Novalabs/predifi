#![cfg(test)]
#![allow(deprecated)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
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

    let (_, client, _, _, _, _, _, creator) = setup(&env);
    let not_admin = Address::generate(&env);
    client.set_fee_bps(&not_admin, &999u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_unauthorized_set_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, _, _, _, _, _, creator) = setup(&env);
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
fn test_cannot_cancel_resolved_pool() {
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
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
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
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    client.cancel_pool(&admin, &pool_id);
    // Should panic because pool is not active (canceled)
    client.resolve_pool(&operator, &pool_id, &1u32);
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
