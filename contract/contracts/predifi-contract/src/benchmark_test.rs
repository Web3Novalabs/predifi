#[cfg(test)]
mod benchmark_tests {
    use crate::{DataKey, PoolConfig, PredifiContract, PredifiContractClient};
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Ledger},
        token, vec, Address, Env, String, Vec,
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
        token::Client<'_>,
        token::StellarAssetClient<'_>,
    ) {
        env.mock_all_auths();
        let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
        let ac_client = dummy_access_control::DummyAccessControlClient::new(env, &ac_id);
        let contract_id = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let treasury = Address::generate(env);
        ac_client.grant_role(&admin, &ROLE_ADMIN);
        client.init(&ac_id, &treasury, &500, &3600, &3600u64);
        let token_admin = Address::generate(env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();
        let token_client = token::Client::new(env, &token_id);
        let token_admin_client = token::StellarAssetClient::new(env, &token_id);
        client.add_token_to_whitelist(&admin, &token_id);
        (client, admin, token_client, token_admin_client)
    }

    #[test]
    fn test_bench_100_outcomes() {
        let env = Env::default();
        let (client, admin, token_client, token_admin_client) = setup(&env);
        let creator = Address::generate(&env);
        let options_count = 100;

        let mut outcome_descriptions = Vec::new(&env);
        for i in 0..options_count {
            outcome_descriptions.push_back(String::from_str(&env, "Outcome"));
        }

        // Measure create_pool
        env.budget().reset_default();
        let pool_id = client.create_pool(
            &creator,
            &(env.ledger().timestamp() + 10000),
            &token_client.address,
            &options_count,
            &symbol_short!("Tech"),
            &PoolConfig {
                description: String::from_str(&env, "Bench"),
                metadata_url: String::from_str(&env, "ipfs://bench"),
                min_stake: 10i128,
                max_stake: 0,
                max_total_stake: 0,
                initial_liquidity: 0,
                required_resolutions: 1,
                private: false,
                whitelist_key: None,
                outcome_descriptions,
            },
        );
        let budget_create = env.budget().cpu_insns();
        let storage_reads_create = env.budget().storage_read_count();
        eprintln!(
            "CREATE_POOL ({} outcomes) -> CPU: {}, Reads: {}",
            options_count, budget_create, storage_reads_create
        );

        // Measure first prediction (triggers fallback if not initialized)
        let user1 = Address::generate(&env);
        token_admin_client.mint(&user1, &1000);
        env.budget().reset_default();
        client.place_prediction(&user1, &pool_id, &1000, &0, &None, &None);
        let budget_pred1 = env.budget().cpu_insns();
        let storage_reads_pred1 = env.budget().storage_read_count();
        eprintln!(
            "FIRST_PREDICTION -> CPU: {}, Reads: {}",
            budget_pred1, storage_reads_pred1
        );

        // Measure second prediction (should use batch key)
        let user2 = Address::generate(&env);
        token_admin_client.mint(&user2, &1000);
        env.budget().reset_default();
        client.place_prediction(&user2, &pool_id, &1000, &1, &None, &None);
        let budget_pred2 = env.budget().cpu_insns();
        let storage_reads_pred2 = env.budget().storage_read_count();
        eprintln!(
            "SECOND_PREDICTION -> CPU: {}, Reads: {}",
            budget_pred2, storage_reads_pred2
        );

        // Resolve
        env.ledger().with_mut(|li| li.timestamp += 20000);
        client.resolve_pool(&admin, &pool_id, &0);

        // Measure claim_winnings
        env.budget().reset_default();
        client.claim_winnings(&user1, &pool_id);
        let budget_claim = env.budget().cpu_insns();
        let storage_reads_claim = env.budget().storage_read_count();
        eprintln!(
            "CLAIM_WINNINGS -> CPU: {}, Reads: {}",
            budget_claim, storage_reads_claim
        );
    }
}
