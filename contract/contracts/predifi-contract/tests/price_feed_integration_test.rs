#![cfg(test)]

use predifi_contract::{MarketState, PoolConfig, PredifiContract, PredifiContractClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, Env, String,
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
    PredifiContractClient<'_>,
    Address,
    Address,
    Address,
    Address,
) {
    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(env, &ac_id);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let creator = Address::generate(env);
    let treasury = Address::generate(env);

    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    ac_client.grant_role(&operator, &ROLE_OPERATOR);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    (client, token, admin, operator, creator)
}

#[test]
fn test_price_based_pool_mock_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, token, _admin, operator, creator) = setup(&env);

    let end_time = env.ledger().timestamp() + 7200;
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token,
        &2u32,
        &symbol_short!("Crypto"),
        &PoolConfig {
            description: String::from_str(&env, "Will ETH > $4000?"),
            metadata_url: String::from_str(&env, "ipfs://eth-price-pool"),
            min_stake: 100,
            max_stake: 0,
            max_total_stake: 0,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: vec![
                &env,
                String::from_str(&env, "No"),
                String::from_str(&env, "Yes"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = end_time + 1);

    // Resolve: outcome 1 = "Yes" (condition met: ETH > $4000)
    client.resolve_pool(&operator, &pool_id, &1u32);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.outcome, 1);
    assert_eq!(
        pool.outcome_descriptions.get(1).unwrap(),
        String::from_str(&env, "Yes")
    );
}
