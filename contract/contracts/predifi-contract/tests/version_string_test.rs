//! Integration test for get_version_string function

use predifi_contract::{PredifiContract, PredifiContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

// Dummy access control contract for testing
mod dummy_ac {
    use soroban_sdk::{contract, contractimpl, Address, Env};

    #[contract]
    pub struct DummyAC;

    #[contractimpl]
    impl DummyAC {
        pub fn grant_role(_env: Env, _user: Address, _role: u32) {}
        pub fn has_role(_env: Env, _user: Address, _role: u32) -> bool {
            true
        }
        pub fn get_operator_count(_env: Env) -> u32 {
            10
        }
    }
}

#[test]
fn test_get_version_string_returns_semantic_version() {
    let env = Env::default();
    env.mock_all_auths();

    // Register access control contract
    let ac_id = env.register(dummy_ac::DummyAC, ());
    let _ac_client = dummy_ac::DummyACClient::new(&env, &ac_id);

    // Register predifi contract
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    // Initialize contract
    let treasury = Address::generate(&env);
    client.init(&ac_id, &treasury, &500u32, &3600u64, &3600u64, &0u32);

    // Test get_version_string
    let version_string = client.get_version_string();
    assert_eq!(version_string, Symbol::new(&env, "0_0_0"));
}

#[test]
fn test_get_version_string_without_init() {
    let env = Env::default();

    // Register predifi contract without initialization
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    // get_version_string should work even without initialization
    let version_string = client.get_version_string();
    assert_eq!(version_string, Symbol::new(&env, "0_0_0"));
}
