#![cfg(test)]

use super::*;
use crate::test_utils::TokenTestContext;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
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

fn setup_integration(
    env: &Env,
) -> (
    PredifiContractClient<'static>,
    TokenTestContext,
    Address, // Admin
    Address, // Operator
    Address, // Treasury
) {
    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let treasury = Address::generate(env);

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(env, &ac_id);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);
    client.init(&ac_id, &treasury, &0u32, &0u64);

    let token_ctx = TokenTestContext::deploy(env, &admin);
    client.add_token_to_whitelist(&admin, &token_ctx.token_address);

    (client, token_ctx, admin, operator, treasury)
}

#[test]
fn test_full_market_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, token_ctx, _admin, operator, _treasury) = setup_integration(&env);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    token_ctx.mint(&user1, 1000);
    token_ctx.mint(&user2, 1000);
    token_ctx.mint(&user3, 1000);

    // 1. Create Pool
    let end_time = 3600u64;
    let pool_id = client.create_pool(
        &user1,
        &end_time,
        &token_ctx.token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // 2. Place Predictions
    client.place_prediction(&user1, &pool_id, &100, &1); // User 1 bets 100 on Outcome 1
    client.place_prediction(&user2, &pool_id, &200, &2); // User 2 bets 200 on Outcome 2
    client.place_prediction(&user3, &pool_id, &300, &1); // User 3 bets 300 on Outcome 1 (Total Outcome 1 = 400)

    // Total stake = 100 + 200 + 300 = 600
    assert_eq!(token_ctx.token.balance(&client.address), 600);

    // 3. Resolve Pool (advance time past end_time=3600)
    env.ledger().with_mut(|li| li.timestamp = 3601);
    client.resolve_pool(&operator, &pool_id, &1u32); // Outcome 1 wins

    // 4. Claim Winnings
    // User 1 Winnings: (100 / 400) * 600 = 150
    let w1 = client.claim_winnings(&user1, &pool_id);
    assert_eq!(w1, 150);
    assert_eq!(token_ctx.token.balance(&user1), 1050); // 1000 - 100 + 150

    // User 3 Winnings: (300 / 400) * 600 = 450
    let w3 = client.claim_winnings(&user3, &pool_id);
    assert_eq!(w3, 450);
    assert_eq!(token_ctx.token.balance(&user3), 1150); // 1000 - 300 + 450

    // User 2 Winnings: 0 (loser)
    let w2 = client.claim_winnings(&user2, &pool_id);
    assert_eq!(w2, 0);
    assert_eq!(token_ctx.token.balance(&user2), 800); // 1000 - 200

    // Contract balance should be 0
    assert_eq!(token_ctx.token.balance(&client.address), 0);
}

#[test]
fn test_multi_user_betting_and_balance_verification() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, token_ctx, _admin, operator, _treasury) = setup_integration(&env);

    // 5 users
    let users: soroban_sdk::Vec<Address> = soroban_sdk::Vec::from_array(
        &env,
        [
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ],
    );

    for user in users.iter() {
        token_ctx.mint(&user, 5000);
    }

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &4000u64,
        &token_ctx.token_address,
        &4u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // Bets:
    // U0: 500 on 1
    // U1: 1000 on 2
    // U2: 500 on 1
    // U3: 1500 on 3
    // U4: 500 on 1
    // Total 1: 1500, Total 2: 1000, Total 3: 1500. Total Stake: 4000.

    client.place_prediction(&users.get(0).unwrap(), &pool_id, &500, &1);
    client.place_prediction(&users.get(1).unwrap(), &pool_id, &1000, &2);
    client.place_prediction(&users.get(2).unwrap(), &pool_id, &500, &1);
    client.place_prediction(&users.get(3).unwrap(), &pool_id, &1500, &3);
    client.place_prediction(&users.get(4).unwrap(), &pool_id, &500, &1);

    assert_eq!(token_ctx.token.balance(&client.address), 4000);

    // Resolve to Outcome 3 (advance time past end_time=4000)
    env.ledger().with_mut(|li| li.timestamp = 4001);
    client.resolve_pool(&operator, &pool_id, &3u32);

    // Winner: U3
    // Winnings: (1500 / 1500) * 4000 = 4000
    let w3 = client.claim_winnings(&users.get(3).unwrap(), &pool_id);
    assert_eq!(w3, 4000);
    assert_eq!(token_ctx.token.balance(&users.get(3).unwrap()), 7500); // 5000 - 1500 + 4000

    // Losers check
    let w0 = client.claim_winnings(&users.get(0).unwrap(), &pool_id);
    assert_eq!(w0, 0);
    assert_eq!(token_ctx.token.balance(&users.get(0).unwrap()), 4500); // 5000 - 500

    assert_eq!(token_ctx.token.balance(&client.address), 0);
}

#[test]
fn test_market_resolution_multiple_winners() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, token_ctx, _admin, operator, _treasury) = setup_integration(&env);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    token_ctx.mint(&user1, 1000);
    token_ctx.mint(&user2, 1000);
    token_ctx.mint(&user3, 1000);

    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &3600u64,
        &token_ctx.token_address,
        &3u32,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(
            &env,
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ),
        &0i128,
        &Symbol::new(&env, "tech"),
    );

    // Bets:
    // U1: 200 on 1
    // U2: 300 on 1
    // U3: 500 on 2
    // Total 1: 500, Total 2: 500. Total Stake: 1000.

    client.place_prediction(&user1, &pool_id, &200, &1);
    client.place_prediction(&user2, &pool_id, &300, &1);
    client.place_prediction(&user3, &pool_id, &500, &2);

    // Advance time past end_time=3600, then resolve
    env.ledger().with_mut(|li| li.timestamp = 3601);
    client.resolve_pool(&operator, &pool_id, &1u32); // Outcome 1 wins

    // U1 Winnings: (200 / 500) * 1000 = 400
    // U2 Winnings: (300 / 500) * 1000 = 600

    let w1 = client.claim_winnings(&user1, &pool_id);
    let w2 = client.claim_winnings(&user2, &pool_id);

    assert_eq!(w1, 400);
    assert_eq!(w2, 600);
    assert_eq!(token_ctx.token.balance(&user1), 1200); // 1000 - 200 + 400
    assert_eq!(token_ctx.token.balance(&user2), 1300); // 1000 - 300 + 600
    assert_eq!(token_ctx.token.balance(&client.address), 0);
}
