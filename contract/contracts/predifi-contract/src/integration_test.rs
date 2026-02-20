#![cfg(test)]

use super::*;
use crate::test_utils::TokenTestContext;
use soroban_sdk::{testutils::Address as _, Address, Env};

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

fn setup_integration(env: &Env) -> (
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
    client.init(&ac_id, &treasury, &0u32);

    let token_ctx = TokenTestContext::deploy(env, &admin);

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
    let end_time = 1000u64;
    let pool_id = client.create_pool(&end_time, &token_ctx.token_address);

    // 2. Place Predictions
    client.place_prediction(&user1, &pool_id, &100, &1); // User 1 bets 100 on Outcome 1
    client.place_prediction(&user2, &pool_id, &200, &2); // User 2 bets 200 on Outcome 2
    client.place_prediction(&user3, &pool_id, &300, &1); // User 3 bets 300 on Outcome 1 (Total Outcome 1 = 400)

    // Total stake = 100 + 200 + 300 = 600
    assert_eq!(token_ctx.token.balance(&client.address), 600);

    // 3. Resolve Pool
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
