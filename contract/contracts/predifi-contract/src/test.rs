#![cfg(test)]
#![allow(deprecated)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn setup(
    env: &Env,
) -> (
    PredifiContractClient,
    Address,
    Address,
    token::Client,
    token::StellarAssetClient,
) {
    env.mock_all_auths();

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);

    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::Client::new(env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(env, &token_contract);

    let admin = Address::generate(env);
    client.init(&admin);

    (
        client,
        admin,
        token_contract,
        token_client,
        token_admin_client,
    )
}

#[test]
fn test_claim_winnings() {
    let env = Env::default();
    let (client, _admin, token_address, token, token_admin_client) = setup(&env);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    let pool_id = client.create_pool(&100, &token_address);

    client.place_prediction(&user1, &pool_id, &100, &1);
    client.place_prediction(&user2, &pool_id, &100, &2);

    let contract_id = client.address.clone();
    assert_eq!(token.balance(&contract_id), 200);

    client.resolve_pool(&pool_id, &1);

    let winnings = client.claim_winnings(&user1, &pool_id);
    assert_eq!(winnings, 200);
    assert_eq!(token.balance(&user1), 1100);

    let winnings2 = client.claim_winnings(&user2, &pool_id);
    assert_eq!(winnings2, 0);
    assert_eq!(token.balance(&user2), 900);
}

#[test]
#[should_panic(expected = "Already claimed")]
fn test_double_claim() {
    let env = Env::default();
    let (client, _admin, token_address, _token, token_admin_client) = setup(&env);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);
    client.resolve_pool(&pool_id, &1);

    client.claim_winnings(&user1, &pool_id);
    client.claim_winnings(&user1, &pool_id);
}

#[test]
#[should_panic(expected = "Pool not resolved")]
fn test_claim_unresolved() {
    let env = Env::default();
    let (client, _admin, token_address, _token, token_admin_client) = setup(&env);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);

    client.claim_winnings(&user1, &pool_id);
}

#[test]
fn test_get_user_predictions() {
    let env = Env::default();
    let (client, _admin, token_address, _token, token_admin_client) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let pool0 = client.create_pool(&100, &token_address);
    let pool1 = client.create_pool(&200, &token_address);
    let pool2 = client.create_pool(&300, &token_address);

    client.place_prediction(&user, &pool0, &10, &1);
    client.place_prediction(&user, &pool1, &20, &2);
    client.place_prediction(&user, &pool2, &30, &1);

    let all = client.get_user_predictions(&user, &0, &10);
    assert_eq!(all.len(), 3);
    assert_eq!(all.get(0).unwrap().pool_id, pool0);
    assert_eq!(all.get(1).unwrap().pool_id, pool1);
    assert_eq!(all.get(2).unwrap().pool_id, pool2);
    assert_eq!(all.get(0).unwrap().amount, 10);
    assert_eq!(all.get(0).unwrap().user_outcome, 1);

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

#[test]
fn test_cancel_pool_refunds() {
    let env = Env::default();
    let (client, _admin, token_address, token, token_admin_client) = setup(&env);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    let pool_id = client.create_pool(&100, &token_address);

    client.place_prediction(&user1, &pool_id, &100, &1);
    client.place_prediction(&user2, &pool_id, &200, &2);

    let contract_id = client.address.clone();
    assert_eq!(token.balance(&contract_id), 300);

    client.cancel_pool(&pool_id);

    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.balance(&user2), 1000);
    assert_eq!(token.balance(&contract_id), 0);

    let pool: Pool = env
        .storage()
        .instance()
        .get(&DataKey::Pool(pool_id))
        .unwrap();
    assert!(pool.cancelled);
    assert_eq!(pool.total_stake, 0);
}

#[test]
#[should_panic]
fn test_cannot_place_after_cancel() {
    let env = Env::default();
    let (client, _admin, token_address, _token, token_admin_client) = setup(&env);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);

    client.cancel_pool(&pool_id);

    // Should panic due to guard_pool_not_final / cancelled check
    client.place_prediction(&user1, &pool_id, &50, &1);
}

#[test]
#[should_panic]
fn test_cannot_resolve_after_cancel() {
    let env = Env::default();
    let (client, _admin, token_address, _token, token_admin_client) = setup(&env);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);

    client.cancel_pool(&pool_id);

    // Should panic because pool is already finalized via cancellation
    client.resolve_pool(&pool_id, &1);
}

#[test]
#[should_panic]
fn test_cannot_claim_after_cancel() {
    let env = Env::default();
    let (client, _admin, token_address, _token, token_admin_client) = setup(&env);

    let user1 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);

    let pool_id = client.create_pool(&100, &token_address);
    client.place_prediction(&user1, &pool_id, &100, &1);

    client.cancel_pool(&pool_id);

    // Should panic because pool.cancelled is true
    client.claim_winnings(&user1, &pool_id);
}
