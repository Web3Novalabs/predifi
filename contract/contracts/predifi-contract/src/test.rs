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

    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    (
        ac_client,
        client,
        token_address,
        token,
        token_admin_client,
        treasury,
        operator,
    )
}

// ── Core prediction tests ────────────────────────────────────────────────────

#[test]
fn test_claim_winnings() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator) = setup(&env);
    let contract_addr = client.address.clone();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );
    client.place_prediction(&user1, &pool_id, &100, &0);
    client.place_prediction(&user2, &pool_id, &100, &1);

    assert_eq!(token.balance(&contract_addr), 200);

    env.ledger().with_mut(|li| li.timestamp = 10001);

    client.resolve_pool(&operator, &pool_id, &0u32);

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

    let (_, client, token_address, _, token_admin_client, _, operator) = setup(&env);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );
    client.place_prediction(&user1, &pool_id, &100, &1);

    env.ledger().with_mut(|li| li.timestamp = 10001);

    client.resolve_pool(&operator, &pool_id, &1u32);

    client.claim_winnings(&user1, &pool_id);
    client.claim_winnings(&user1, &pool_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_claim_unresolved() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _) = setup(&env);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );
    client.place_prediction(&user1, &pool_id, &100, &1);

    client.claim_winnings(&user1, &pool_id);
}

#[test]
fn test_multiple_pools_independent() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator) = setup(&env);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    let pool_a = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );
    let pool_b = client.create_pool(
        &20000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );

    client.place_prediction(&user1, &pool_a, &100, &1);
    client.place_prediction(&user2, &pool_b, &100, &0);

    env.ledger().with_mut(|li| li.timestamp = 20001);

    client.resolve_pool(&operator, &pool_a, &1u32);
    client.resolve_pool(&operator, &pool_b, &0u32);

    let w1 = client.claim_winnings(&user1, &pool_a);
    assert_eq!(w1, 100);

    let w2 = client.claim_winnings(&user2, &pool_b);
    assert_eq!(w2, 100);
}

// ── Access control tests ─────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_unauthorized_set_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, _, _, _, _, _) = setup(&env);
    let not_admin = Address::generate(&env);
    client.set_fee_bps(&not_admin, &999u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_unauthorized_set_treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, _, _, _, _, _) = setup(&env);
    let not_admin = Address::generate(&env);
    let new_treasury = Address::generate(&env);
    client.set_treasury(&not_admin, &new_treasury);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_unauthorized_resolve_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _) = setup(&env);
    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );
    let not_operator = Address::generate(&env);
    env.ledger().with_mut(|li| li.timestamp = 10001);
    client.resolve_pool(&not_operator, &pool_id, &1u32);
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

    client.pause(&admin);
    client.create_pool(
        &10000u64,
        &token,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
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
    token_admin_client.mint(&user, &1000);

    client.pause(&admin);
    client.unpause(&admin);

    let pool_id = client.create_pool(
        &10000u64,
        &token_contract,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );
    client.place_prediction(&user, &pool_id, &10, &1);
}

// ── Pagination tests ──────────────────────────────────────────────────────────

#[test]
fn test_get_user_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let pool0 = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );
    let pool1 = client.create_pool(
        &20000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );
    let pool2 = client.create_pool(
        &30000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &1i128,
        &0i128,
    );

    client.place_prediction(&user, &pool0, &10, &0);
    client.place_prediction(&user, &pool1, &20, &1);
    client.place_prediction(&user, &pool2, &30, &0);

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

// ── Pool Cancelation & State Guard Tests ────────────────────────────────────────

#[test]
fn test_cancel_pool_refunds_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator) = setup(&env);
    let contract_addr = client.address.clone();

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Cancel Test Pool"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
    );
    client.place_prediction(&user1, &pool_id, &100, &1);

    assert_eq!(token.balance(&contract_addr), 100);

    // Cancel pool before end time
    client.cancel_pool(&operator, &pool_id);

    // Claim refunds
    let refund = client.claim_winnings(&user1, &pool_id);
    assert_eq!(refund, 100);
    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.balance(&contract_addr), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_cannot_cancel_resolved_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator) = setup(&env);

    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Resolve Then Cancel Pool"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
    );

    env.ledger().with_mut(|li| li.timestamp = 10001);
    client.resolve_pool(&operator, &pool_id, &1u32);
    // Should panic because pool is not active
    client.cancel_pool(&operator, &pool_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_cannot_resolve_canceled_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator) = setup(&env);

    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Resolve Canceled Pool Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
    );

    client.cancel_pool(&operator, &pool_id);
    env.ledger().with_mut(|li| li.timestamp = 10001);
    // Should panic because pool is not active
    client.resolve_pool(&operator, &pool_id, &1u32);
}

#[test]
#[should_panic(expected = "Pool is not active")]
fn test_cannot_predict_on_canceled_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator) = setup(&env);
    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Predict Canceled Pool Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
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

    let end_time = 10000;
    let pool_id = client.create_pool(
        &end_time,
        &token,
        &2u32,
        &String::from_str(&env, "Delay Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
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

    let end_time = 10000;
    let pool_id = client.create_pool(
        &end_time,
        &token,
        &2u32,
        &String::from_str(&env, "Delay Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
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
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let treasury = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&ac_id, &treasury, &0u32, &3600u64);

    let end_time = 10000;
    let pool_id = client.create_pool(
        &end_time,
        &token,
        &2u32,
        &String::from_str(&env, "Ready Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
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

    let (_, client, token_address, _, token_admin_client, _, _) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    // Create pool with min_stake = 50
    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Min Stake Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &50i128,
        &0i128,
    );

    // Should panic: amount (10) < min_stake (50)
    client.place_prediction(&user, &pool_id, &10, &0);
}

#[test]
#[should_panic(expected = "amount exceeds the pool maximum stake")]
fn test_stake_above_maximum_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    // Create pool with min_stake = 1, max_stake = 100
    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Max Stake Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &100i128,
    );

    // Should panic: amount (200) > max_stake (100)
    client.place_prediction(&user, &pool_id, &200, &0);
}

#[test]
fn test_stake_at_boundaries_accepted() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _) = setup(&env);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    // Create pool with min_stake = 10, max_stake = 200
    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Boundary Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &10i128,
        &200i128,
    );

    // Both boundary values should succeed
    client.place_prediction(&user1, &pool_id, &10, &0); // exactly min_stake
    client.place_prediction(&user2, &pool_id, &200, &1); // exactly max_stake
}

#[test]
fn test_set_stake_limits_by_operator() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    // Create pool with min_stake = 1
    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Update Limits Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
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

    let (_, client, token_address, _, _, _, _) = setup(&env);

    let pool_id = client.create_pool(
        &10000u64,
        &token_address,
        &2u32,
        &String::from_str(&env, "Unauthorized Limits Test"),
        &String::from_str(&env, "ipfs://metadata"),
        &1i128,
        &0i128,
    );

    // Non-operator should be rejected
    let not_operator = Address::generate(&env);
    client.set_stake_limits(&not_operator, &pool_id, &50i128, &500i128);
}
