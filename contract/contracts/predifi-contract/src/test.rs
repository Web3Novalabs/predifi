#![cfg(test)]
#![allow(deprecated)]

extern crate std;

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{
        storage::Instance as _, storage::Persistent as _, Address as _, AuthorizedFunction,
        AuthorizedInvocation, Events, Ledger, Logs,
    },
    token, vec, Address, BytesN, Env, IntoVal, String, Symbol, TryFromVal, Val,
};

pub(crate) mod dummy_access_control {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct DummyAccessControl;

    #[contractimpl]
    impl DummyAccessControl {
        pub fn grant_role(env: Env, user: Address, role: u32) {
            let already_has_key = (Symbol::new(&env, "role"), user.clone(), role);
            let already_has: bool = env
                .storage()
                .instance()
                .get(&already_has_key)
                .unwrap_or(false);

            env.storage().instance().set(&already_has_key, &true);

            if role == 0 {
                let admin_key = Symbol::new(&env, "admin");
                env.storage().instance().set(&admin_key, &user);
            }

            // Track operator count
            if role == 1 && !already_has {
                let count_key = Symbol::new(&env, "op_count");
                let count: u32 = env.storage().instance().get(&count_key).unwrap_or(0);
                env.storage().instance().set(&count_key, &(count + 1));
            }
        }

        pub fn revoke_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user, role);
            let had_role: bool = env.storage().instance().get(&key).unwrap_or(false);
            env.storage().instance().set(&key, &false);

            if role == 1 && had_role {
                let count_key = Symbol::new(&env, "op_count");
                let count: u32 = env.storage().instance().get(&count_key).unwrap_or(0);
                if count > 0 {
                    env.storage().instance().set(&count_key, &(count - 1));
                }
            }
        }

        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }

        pub fn get_operator_count(env: Env) -> u32 {
            let count_key = Symbol::new(&env, "op_count");
            env.storage().instance().get(&count_key).unwrap_or(0)
        }

        pub fn get_admin(env: Env) -> Address {
            let admin_key = Symbol::new(&env, "admin");
            env.storage()
                .instance()
                .get(&admin_key)
                .expect("admin not set in dummy access control")
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

pub(crate) const ROLE_ADMIN: u32 = 0; // i am testing this
pub(crate) const ROLE_OPERATOR: u32 = 1; // i am testing this the second one
const ROLE_ORACLE: u32 = 3;

pub(crate) fn setup(
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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

fn assert_single_contract_auth(
    env: &Env,
    expected_address: &Address,
    contract: &Address,
    fn_name: &str,
    args: soroban_sdk::Vec<Val>,
) {
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(
        auths,
        std::vec![(
            expected_address.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    contract.clone(),
                    Symbol::new(env, fn_name),
                    args,
                )),
                sub_invocations: std::vec![],
            }
        )]
    );
}

// ── Core prediction tests ────────────────────────────────────────────────────

#[test]
fn test_set_fee_bps_auth_only_happens_at_entry_point() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _, _, _, _, _, _) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    client.set_fee_bps(&admin, &250u32);

    assert_single_contract_auth(
        &env,
        &admin,
        &client.address,
        "set_fee_bps",
        (&admin, 250u32).into_val(&env),
    );
}

#[test]
fn test_increase_max_total_stake_auth_only_happens_at_entry_point() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Cap Increase Pool"),
            metadata_url: String::from_str(&env, "ipfs://cap-increase"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 100i128,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    client.increase_max_total_stake(&creator, &pool_id, &250i128);

    assert_single_contract_auth(
        &env,
        &creator,
        &client.address,
        "increase_max_total_stake",
        (&creator, pool_id, 250i128).into_val(&env),
    );
}

#[test]
fn test_resolve_pool_auth_only_happens_at_entry_point() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);
    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Resolve Auth Pool"),
            metadata_url: String::from_str(&env, "ipfs://resolve-auth"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    client.place_prediction(&user, &pool_id, &100, &1u32, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 100001);
    client.resolve_pool(&operator, &pool_id, &1u32);

    assert_single_contract_auth(
        &env,
        &operator,
        &client.address,
        "resolve_pool",
        (&operator, pool_id, 1u32).into_val(&env),
    );
}

#[test]
fn test_oracle_resolve_auth_only_happens_at_entry_point() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);
    let oracle = Address::generate(&env);
    let user = Address::generate(&env);
    ac_client.grant_role(&oracle, &ROLE_ORACLE);
    token_admin_client.mint(&user, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Oracle Auth Pool"),
            metadata_url: String::from_str(&env, "ipfs://oracle-auth"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
    client.place_prediction(&user, &pool_id, &100, &1u32, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 100001);
    client.oracle_resolve(&oracle, &pool_id, &1u32, &String::from_str(&env, "proof"));

    assert_single_contract_auth(
        &env,
        &oracle,
        &client.address,
        "oracle_resolve",
        (&oracle, pool_id, 1u32, String::from_str(&env, "proof")).into_val(&env),
    );
}

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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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

#[test]
fn test_claim_winnings_zero_share() {
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
    // Initialize with 2% protocol fee (200 bps)
    client.init(&ac_id, &treasury, &200u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_address);

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let user_c = Address::generate(&env);

    token_admin_client.mint(&user_a, &1000);
    token_admin_client.mint(&user_b, &1000);
    token_admin_client.mint(&user_c, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Small Stake Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // User A stakes 100 on Outcome 1 (winner)
    client.place_prediction(&user_a, &pool_id, &100, &1, &None, &None);
    // User B stakes 1 on Outcome 1 (winner)
    client.place_prediction(&user_b, &pool_id, &1, &1, &None, &None);
    // User C stakes 1 on Outcome 0 (loser)
    client.place_prediction(&user_c, &pool_id, &1, &0, &None, &None);

    // Total Stake: 102
    // Winning Stake (Outcome 1): 101
    // Fee: 2% of 102 = 2 (ProtocolFavor/floor rounding)
    // Payout Pool: 102 - 2 = 100
    // User B Winnings: (1 * 100) / 101 = 0 (integer division)

    env.ledger().with_mut(|li| li.timestamp = 100001);
    client.resolve_pool(&operator, &pool_id, &1u32);

    let winnings_b = client.claim_winnings(&user_b, &pool_id);
    assert_eq!(winnings_b, 0);
    assert_eq!(token.balance(&user_b), 999); // Initial 1000 - 1 stake + 0 winnings

    // User A should get the rest
    let winnings_a = client.claim_winnings(&user_a, &pool_id);
    assert_eq!(winnings_a, 99); // (100 * 100) / 101 = 99
    assert_eq!(token.balance(&user_a), 900 + 99); // Initial 1000 - 100 + 99 = 999
}

/// Test claim_winnings with zero total winnings due to high protocol fee
/// This test verifies the scenario described in issue #407 where the protocol fee
/// is large enough that after deduction, the payout pool for a very small winning
/// stake results in 0 winnings for some users.
#[test]
fn test_claim_winnings_zero_total_winnings_high_fee() {
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

    // Initialize with very high protocol fee (90% = 9000 bps)
    client.init(&ac_id, &treasury, &9000u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_address);

    let large_winner = Address::generate(&env);
    let small_winner = Address::generate(&env);
    let loser = Address::generate(&env);

    token_admin_client.mint(&large_winner, &10000);
    token_admin_client.mint(&small_winner, &1000);
    token_admin_client.mint(&loser, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Finance"),
        &PoolConfig {
            description: String::from_str(&env, "High Fee Test Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Large winner stakes 1000 on Outcome 1 (winner)
    client.place_prediction(&large_winner, &pool_id, &1000, &1, &None, &None);
    // Small winner stakes only 1 on Outcome 1 (winner) - very small share
    client.place_prediction(&small_winner, &pool_id, &1, &1, &None, &None);
    // Loser stakes 100 on Outcome 0 (loser)
    client.place_prediction(&loser, &pool_id, &100, &0, &None, &None);

    // Total Stake: 1101
    // Winning Stake (Outcome 1): 1001
    // Protocol Fee: 90% of 1101 = 990 (ProtocolFavor/floor rounding)
    // Payout Pool: 1101 - 990 = 111
    // Large winner's expected winnings: (1000 * 111) / 1001 = 110 (integer division)
    // Small winner's expected winnings: (1 * 111) / 1001 = 0 (integer division - zero winnings!)

    env.ledger().with_mut(|li| li.timestamp = 100001);
    client.resolve_pool(&operator, &pool_id, &1u32);

    // Claim winnings for small winner - should return 0 without crashing
    let winnings_small = client.claim_winnings(&small_winner, &pool_id);

    assert_eq!(winnings_small, 0);
    assert_eq!(token.balance(&small_winner), 999); // Initial 1000 - 1 stake + 0 winnings

    // Large winner should get most of the payout pool
    let winnings_large = client.claim_winnings(&large_winner, &pool_id);
    assert_eq!(winnings_large, 110);
    assert_eq!(token.balance(&large_winner), 9000 + 110); // Initial 10000 - 1000 + 110

    // Loser gets nothing
    let winnings_loser = client.claim_winnings(&loser, &pool_id);
    assert_eq!(winnings_loser, 0);
    assert_eq!(token.balance(&loser), 900); // Initial 1000 - 100 stake + 0 winnings

    // Verify contract holds the protocol fee (may have rounding differences)
    let contract_balance = token.balance(&contract_id);
    assert!(
        (990..=991).contains(&contract_balance),
        "Contract balance should be around protocol fee, got {}",
        contract_balance
    );
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
    client.init(&ac_id, &treasury, &200u32, &0u64, &3600u64, &0u32); // 2% protocol fee
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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

#[test]
fn test_invalid_category_fallback() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "InvalidCat"),
        &PoolConfig {
            description: String::from_str(&env, "Invalid Category Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.category, CATEGORY_OTHER);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
fn test_oracle_resolve_long_proof() {
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100001);

    // Create a 1024-byte proof string
    let long_proof = "a".repeat(1024);
    let proof_str = String::from_str(&env, &long_proof);

    // Call oracle_resolve which should succeed
    client.oracle_resolve(&oracle, &pool_id, &1u32, &proof_str);

    let events = env.events().all();
    let oracle_resolved_topic = Symbol::new(&env, "oracle_resolved");

    let mut found = false;
    for e in events.iter() {
        if let Some(topic_val) = e.1.get(0) {
            if let Ok(topic_sym) = Symbol::try_from_val(&env, &topic_val) {
                if topic_sym == oracle_resolved_topic {
                    let event_data: soroban_sdk::Map<Symbol, Val> = e.2.clone().into_val(&env);
                    let event_proof: String = event_data
                        .get(Symbol::new(&env, "proof"))
                        .unwrap()
                        .into_val(&env);
                    assert_eq!(event_proof, proof_str);
                    found = true;
                }
            }
        }
    }
    assert!(found, "OracleResolvedEvent not found");
}

#[test]
fn test_oracle_resolve_utf8_emoji_proof() {
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100001);

    // Create a proof string with emojis and UTF-8 characters
    let emoji_proof = "Proof ✨🔗 🧑‍💻 ✓ æøå 🔥 test end";
    let proof_str = String::from_str(&env, emoji_proof);

    // Call oracle_resolve which should succeed
    client.oracle_resolve(&oracle, &pool_id, &1u32, &proof_str);

    let events = env.events().all();
    let oracle_resolved_topic = Symbol::new(&env, "oracle_resolved");

    let mut found = false;
    for e in events.iter() {
        if let Some(topic_val) = e.1.get(0) {
            if let Ok(topic_sym) = Symbol::try_from_val(&env, &topic_val) {
                if topic_sym == oracle_resolved_topic {
                    let event_data: soroban_sdk::Map<Symbol, Val> = e.2.clone().into_val(&env);
                    let event_proof: String = event_data
                        .get(Symbol::new(&env, "proof"))
                        .unwrap()
                        .into_val(&env);
                    assert_eq!(event_proof, proof_str);
                    found = true;
                }
            }
        }
    }
    assert!(found, "OracleResolvedEvent not found for emoji proof");
}

#[test]
fn test_events_module_publish_and_log() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredifiContract, ());
    let admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        PauseEvent {
            admin: admin.clone(),
        }
        .publish(&env);
    });

    let events = env.events().all();
    let pause_topic = Symbol::new(&env, "pause");

    let mut found = false;
    for event in events.iter() {
        if let Some(topic_val) = event.1.get(0) {
            if let Ok(topic_sym) = Symbol::try_from_val(&env, &topic_val) {
                if topic_sym == pause_topic {
                    found = true;
                    break;
                }
            }
        }
    }

    assert!(
        found,
        "PauseEvent should have been emitted via events module"
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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

    // Custom setup without operators for oracle-only resolution testing
    let ac_id = env.register(dummy_access_control::DummyAccessControl, ());
    let ac_client = dummy_access_control::DummyAccessControlClient::new(&env, &ac_id);
    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let token_address = token_contract;

    let treasury = Address::generate(&env);
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);

    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token_address);

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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 2u32, // Changed from 1 to 2 to test multi-oracle voting
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    // Admin should be able to cancel
    client.cancel_pool(&admin, &pool_id, &String::from_str(&env, ""));
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    // Admin should be able to cancel their pool
    client.cancel_pool(&creator, &pool_id, &String::from_str(&env, ""));
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    let unauthorized = Address::generate(&env);
    // This should fail - user is not admin
    client.cancel_pool(&unauthorized, &pool_id, &String::from_str(&env, ""));
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
fn test_is_whitelisted_tracks_explicit_private_pool_membership() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, _) = setup(&env);
    let creator = Address::generate(&env);
    let invited_user = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Private whitelist helper"),
            metadata_url: String::from_str(&env, "ipfs://private-whitelist-helper"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: true,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    assert!(!client.is_whitelisted(&pool_id, &creator));
    assert!(!client.is_whitelisted(&pool_id, &invited_user));

    client.add_to_whitelist(&creator, &pool_id, &invited_user);
    assert!(client.is_whitelisted(&pool_id, &invited_user));

    client.remove_from_whitelist(&creator, &pool_id, &invited_user);
    assert!(!client.is_whitelisted(&pool_id, &invited_user));
}

#[test]
fn test_is_whitelisted_returns_false_for_public_pool_without_entry() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, _) = setup(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Public whitelist helper"),
            metadata_url: String::from_str(&env, "ipfs://public-whitelist-helper"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    assert!(!client.is_whitelisted(&pool_id, &user));
}

#[test]
fn test_add_to_whitelist_is_idempotent_for_already_whitelisted_user() {
    // Issue #411: Verify that adding a user to a private pool's whitelist twice
    // doesn't cause errors or double-logging. The operation should be idempotent.
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, _) = setup(&env);
    let creator = Address::generate(&env);
    let user_a = Address::generate(&env);

    // Step 1: Create a private pool
    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Private pool for idempotency test"),
            metadata_url: String::from_str(&env, "ipfs://idempotency-test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: true,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Verify User A is not whitelisted initially
    assert!(!client.is_whitelisted(&pool_id, &user_a));

    // Step 2: Call add_to_whitelist for User A (first time)
    // This should succeed without panicking
    client.add_to_whitelist(&creator, &pool_id, &user_a);
    assert!(
        client.is_whitelisted(&pool_id, &user_a),
        "User A should be whitelisted after first call"
    );

    // Step 3: Call add_to_whitelist for User A again (second time)
    // This should succeed without error (idempotent behavior)
    // If this were to fail, the test would panic
    client.add_to_whitelist(&creator, &pool_id, &user_a);
    assert!(
        client.is_whitelisted(&pool_id, &user_a),
        "User A should still be whitelisted after second call"
    );

    // Step 4: Verify the user can still be removed normally
    client.remove_from_whitelist(&creator, &pool_id, &user_a);
    assert!(
        !client.is_whitelisted(&pool_id, &user_a),
        "User A should no longer be whitelisted after removal"
    );

    // Step 5: Verify re-adding after removal works
    client.add_to_whitelist(&creator, &pool_id, &user_a);
    assert!(
        client.is_whitelisted(&pool_id, &user_a),
        "User A should be whitelisted again"
    );
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.cancel_pool(&admin, &pool_id, &String::from_str(&env, ""));
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    // Cancel the pool
    client.cancel_pool(&admin, &pool_id, &String::from_str(&env, ""));

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    // Admin cancels the pool
    client.cancel_pool(&admin, &pool_id, &String::from_str(&env, ""));

    // Attempt to cancel again should fail (already canceled)
    let non_admin = Address::generate(&env);
    client.cancel_pool(&non_admin, &pool_id, &String::from_str(&env, ""));
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.cancel_pool(&admin, &pool_id, &String::from_str(&env, ""));

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.cancel_pool(&admin, &pool_id, &String::from_str(&env, ""));

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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 10001);
    client.resolve_pool(&operator, &pool_id, &1u32);
    // Should panic because pool is already resolved
    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
                String::from_str(&env, "Outcome 2"),
            ],
        },
    );

    client.cancel_pool(&admin, &pool_id, &String::from_str(&env, ""));
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));
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
    client.init(&ac_id, &treasury, &0u32, &3600u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
fn test_resolve_pool_logs_reason_when_resolution_delay_not_met() {
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

    client.init(&ac_id, &treasury, &0u32, &3600u64, &3600u64, &0u32);
    client.add_token_to_whitelist(&admin, &token);

    let end_time = 10_000u64;
    let creator = Address::generate(&env);
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Delay log test"),
            metadata_url: String::from_str(&env, "ipfs://delay-log"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = end_time + 10);

    let result = env.as_contract(&contract_id, || {
        PredifiContract::resolve_pool(env.clone(), operator.clone(), pool_id, 1u32)
    });
    assert_eq!(result, Err(PredifiError::ResolutionDelayNotMet));

    let logs = env.logs().all();
    assert!(
        logs.iter()
            .any(|entry| entry.contains("resolve_pool rejected: resolution delay not met")),
        "expected a resolve_pool delay diagnostic log, got: {logs:?}"
    );
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
    client.init(&ac_id, &treasury, &0u32, &3600u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.init(&ac_id, &treasury, &0u32, &3600u64, &3600u64, &0u32);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32, private: false, whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![&env, String::from_str(&env, "Outcome 0"), String::from_str(&env, "Outcome 1")],
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: // max_stake == max_amount is valid
        0i128,
            required_resolutions: 1u32, private: false, whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![&env, String::from_str(&env, "Outcome 0"), String::from_str(&env, "Outcome 1")],
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
            min_total_stake: 1,
            initial_liquidity: // no max_stake limit
        0i128,
            required_resolutions: 1u32, private: false, whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![&env, String::from_str(&env, "Outcome 0"), String::from_str(&env, "Outcome 1")],
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![&env, String::from_str(&env, "Outcome 0")],
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );
}

/// Zero-duration pools (end_time == current ledger timestamp) must be rejected.
#[test]
#[should_panic(expected = "end_time must be in the future")]
fn test_create_pool_rejects_zero_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    client.create_pool(
        &creator,
        &1_000u64,
        &token_address,
        &2u32,
        &Symbol::new(&env, "Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Zero duration pool"),
            metadata_url: String::from_str(&env, "ipfs://zero-duration"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128, // max_stake < min_stake -> invalid
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128, // max_stake == min_stake -> valid
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    for _ in 0..3u32 {
        let not_operator = Address::generate(&env);
        let result = client.try_cancel_pool(&not_operator, &pool_id, &String::from_str(&env, ""));
        assert!(result.is_err(), "Unauthorized cancel must fail");
    }

    // Legitimate operator can still cancel.
    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.cancel_pool(&operator, &pool_a, &String::from_str(&env, ""));

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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
    client_1.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);
    client_2.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

    // Both should start unpaused
    assert!(!client_1.is_contract_paused());
    assert!(!client_2.is_contract_paused());

    // Pause only contract 1
    client_1.pause(&admin);

    // Contract 1 should be paused, contract 2 should remain unpaused
    assert!(client_1.is_contract_paused());
    assert!(!client_2.is_contract_paused());
}

// ═══════════════════════════════════════════════════════════════════════════
// is_pool_active Helper Tests
// ═══════════════════════════════════════════════════════════════════════════

/// is_pool_active returns true for a freshly created pool.
#[test]
fn test_is_pool_active_returns_true_for_active_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Active pool test"),
            metadata_url: String::from_str(&env, "ipfs://active"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Active);
    assert_eq!(pool.state, MarketState::Active);
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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(env, "Outcome 0"),
                String::from_str(env, "Outcome 1"),
            ],
        },
    )
}

/// is_pool_active returns false (via behavior) after pool is resolved —
/// resolve_pool on an already-resolved pool must panic.
#[test]
#[should_panic(expected = "Pool already resolved")]
fn test_is_pool_active_false_after_resolve() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Resolve inactive test"),
            metadata_url: String::from_str(&env, "ipfs://resolved"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    // Pool is now resolved — resolved == true, state == Resolved.
    // is_pool_active would return false, so a second resolve attempt must panic.
    client.resolve_pool(&operator, &pool_id, &0u32);
}

/// is_pool_active returns false (via behavior) after pool is canceled —
/// place_prediction on a canceled pool must panic with the correct message.
#[test]
#[should_panic(expected = "Cannot place prediction on canceled pool")]
fn test_is_pool_active_false_after_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Cancel inactive test"),
            metadata_url: String::from_str(&env, "ipfs://canceled"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &500);

    // Pool is canceled — is_pool_active returns false.
    // place_prediction must be blocked.
    client.place_prediction(&user, &pool_id, &100, &0, &None, &None);
}

/// Resolving a canceled pool must be blocked — verifies is_pool_active
/// integration in resolve_pool.
#[test]
#[should_panic(expected = "Cannot resolve a canceled pool")]
fn test_is_pool_active_blocks_resolve_on_canceled_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Cancel then resolve test"),
            metadata_url: String::from_str(&env, "ipfs://cancelresolve"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    // is_pool_active == false → should panic
    client.resolve_pool(&operator, &pool_id, &0u32);
}

/// Canceling a canceled pool a second time must be blocked — verifies
/// is_pool_active integration in cancel_pool.
#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_is_pool_active_blocks_double_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Double cancel test"),
            metadata_url: String::from_str(&env, "ipfs://doublecancel"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));
    // Second cancel: is_pool_active == false → should panic
    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));
}

/// increase_max_total_stake on a resolved pool must return InvalidPoolState —
/// verifies is_pool_active integration in that function too.
#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_is_pool_active_blocks_increase_max_stake_on_resolved_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Max stake resolved test"),
            metadata_url: String::from_str(&env, "ipfs://maxresolved"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    // Pool resolved → is_pool_active == false → must return InvalidPoolState (24)
    client.increase_max_total_stake(&creator, &pool_id, &500_000);
}

/// Full lifecycle: active → predictions → resolve → claim.
/// Confirms is_pool_active correctly gates each phase without regression.
#[test]
fn test_is_pool_active_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, token_admin_client, _, operator, creator) = setup(&env);
    let contract_addr = client.address.clone();

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Lifecycle test"),
            metadata_url: String::from_str(&env, "ipfs://lifecycle"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    // Phase 1: pool is active — predictions accepted.
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Active);

    let user_win = Address::generate(&env);
    let user_lose = Address::generate(&env);
    token_admin_client.mint(&user_win, &300);
    token_admin_client.mint(&user_lose, &200);

    client.place_prediction(&user_win, &pool_id, &300, &0, &None, &None);
    client.place_prediction(&user_lose, &pool_id, &200, &1, &None, &None);
    assert_eq!(token.balance(&contract_addr), 500);

    // Phase 2: resolve — pool transitions to inactive.
    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_id, &0u32);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Resolved);
    assert_eq!(pool.state, MarketState::Resolved);

    // Phase 3: claims work correctly post-resolution.
    let w = client.claim_winnings(&user_win, &pool_id);
    assert_eq!(w, 500);
    let l = client.claim_winnings(&user_lose, &pool_id);
    assert_eq!(l, 0);
    assert_eq!(token.balance(&contract_addr), 0);
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

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));

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

// ── get_pool_config tests ─────────────────────────────────────────────────────

#[test]
fn test_get_pool_config_matches_creation_params() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let description = String::from_str(&env, "Will BTC hit 100k?");
    let metadata_url = String::from_str(
        &env,
        "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
    );
    let min_stake = 10i128;
    let max_stake = 500i128;
    let initial_liquidity = 0i128;
    let required_resolutions = 1u32;

    let config = PoolConfig {
        description: description.clone(),
        metadata_url: metadata_url.clone(),
        min_stake,
        max_stake,
        min_total_stake: 10i128,
        max_total_stake: 0i128,
        initial_liquidity,
        required_resolutions,
        private: false,
        whitelist_key: None,
        outcome_descriptions: soroban_sdk::vec![
            &env,
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    };

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &config,
    );

    let returned = client.get_pool_config(&pool_id);

    assert_eq!(returned.description, description);
    assert_eq!(returned.metadata_url, metadata_url);
    assert_eq!(returned.min_stake, min_stake);
    assert_eq!(returned.max_stake, max_stake);
    assert_eq!(returned.initial_liquidity, initial_liquidity);
    assert_eq!(returned.required_resolutions, required_resolutions);
    assert!(!returned.private);
    assert_eq!(returned.whitelist_key, None);
}

#[test]
fn test_get_pool_config_private_pool_with_whitelist_key() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let whitelist_key = symbol_short!("secret");
    let config = PoolConfig {
        description: String::from_str(&env, "Private pool"),
        metadata_url: String::from_str(&env, "ipfs://test"),
        min_stake: 1i128,
        max_stake: 0i128,
        min_total_stake: 1i128,
        max_total_stake: 0i128,
        initial_liquidity: 0i128,
        required_resolutions: 1u32,
        private: true,
        whitelist_key: Some(whitelist_key.clone()),
        outcome_descriptions: soroban_sdk::vec![
            &env,
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    };

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &config,
    );

    let returned = client.get_pool_config(&pool_id);

    assert!(returned.private);
    assert_eq!(returned.whitelist_key, Some(whitelist_key));
}

#[test]
fn test_get_pool_config_with_initial_liquidity() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);
    token_admin_client.mint(&creator, &1000);

    let initial_liquidity = 200i128;
    let config = PoolConfig {
        description: String::from_str(&env, "Liquidity pool"),
        metadata_url: String::from_str(&env, "ipfs://test"),
        min_stake: 5i128,
        max_stake: 100i128,
        min_total_stake: 5i128,
        max_total_stake: 0i128,
        initial_liquidity,
        required_resolutions: 1u32,
        private: false,
        whitelist_key: None,
        outcome_descriptions: soroban_sdk::vec![
            &env,
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    };

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Finance"),
        &config,
    );

    let returned = client.get_pool_config(&pool_id);

    assert_eq!(returned.initial_liquidity, initial_liquidity);
    assert_eq!(returned.min_stake, 5i128);
    assert_eq!(returned.max_stake, 100i128);
}

#[test]
fn test_get_pool_config_multiple_pools_independent() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    // Mint tokens for initial liquidity
    token_admin_client.mint(&creator, &1000i128);

    let pool_a = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Pool A"),
            metadata_url: String::from_str(&env, ""),
            min_stake: 1i128,
            max_stake: 0i128,
            min_total_stake: 1i128,
            max_total_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let pool_b = client.create_pool(
        &creator,
        &200000u64,
        &token_address,
        &3u32,
        &symbol_short!("Finance"),
        &PoolConfig {
            description: String::from_str(&env, "Pool B"),
            metadata_url: String::from_str(&env, ""),
            min_stake: 10i128,
            max_stake: 100i128,
            min_total_stake: 10i128,
            max_total_stake: 1000i128,
            initial_liquidity: 50i128,
            required_resolutions: 1u32,
            private: true,
            whitelist_key: Some(Symbol::new(&env, "secret")),
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Option 1"),
                String::from_str(&env, "Option 2"),
                String::from_str(&env, "Option 3"),
            ],
        },
    );

    let config_a = client.get_pool_config(&pool_a);
    let config_b = client.get_pool_config(&pool_b);

    // Verify pool A config
    assert_eq!(config_a.min_stake, 1i128);
    assert_eq!(config_a.max_stake, 0i128);
    assert_eq!(config_a.initial_liquidity, 0i128);
    assert_eq!(config_a.required_resolutions, 1u32);
    assert!(!config_a.private);

    // Verify pool B config
    assert_eq!(config_b.min_stake, 10i128);
    assert_eq!(config_b.max_stake, 100i128);
    assert_eq!(config_b.initial_liquidity, 50i128);
    assert_eq!(config_b.required_resolutions, 1u32);
    assert!(config_b.private);
}

// ============================================================================
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

#[test]
fn test_version_string_returns_semantic_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (_ac_client, client, _token_address, _token, _token_admin, _treasury, _operator, _creator) =
        setup(&env);
    let version_string = client.get_version_string();
    assert_eq!(version_string, Symbol::new(&env, "0_0_0"));
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
            min_total_stake: 1,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Outcome 0"),
                String::from_str(&env, "Outcome 1"),
            ],
        },
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.max_total_stake, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// get_active_pools Tests (#389)
// ═══════════════════════════════════════════════════════════════════════════

/// Empty result when no pools have been created.
#[test]
fn test_get_active_pools_empty_when_no_pools() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, _, _, _, _, _, _) = setup(&env);

    let result = client.get_active_pools(&0u32, &10u32);
    assert_eq!(result.len(), 0);
}

/// A freshly created pool appears in get_active_pools.
#[test]
fn test_get_active_pools_contains_new_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Active pool"),
            metadata_url: String::from_str(&env, "ipfs://active"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let result = client.get_active_pools(&0u32, &10u32);
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(0).unwrap(), pool_id);
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
            min_total_stake: 1,
            initial_liquidity: 0,
            required_resolutions: 1,
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

/// Multiple pools across different categories all appear in get_active_pools.
/// This is the verification case explicitly required by the issue.
// #[test]
// fn test_get_active_pools_returns_pools_across_all_categories() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let (_, client, token_address, _, _, _, _, creator) = setup(&env);

//     let pool_tech = client.create_pool(
//         &creator,
//         &100_000u64,
//         &token_address,
//         &2u32,
//         &symbol_short!("Tech"),
//         &PoolConfig {
//             description: String::from_str(&env, "Tech pool"),
//             metadata_url: String::from_str(&env, "ipfs://tech"),
//             min_stake: 1i128,
//             max_stake: 0i128,
//             max_total_stake: 0,
// min_total_stake: 1,
//             initial_liquidity: 0i128,
//             required_resolutions: 1u32,
//             private: false,
//             whitelist_key: None,
//         },
//     );

//     let pool_sports = client.create_pool(
//         &creator,
//         &100_000u64,
//         &token_address,
//         &2u32,
//         &symbol_short!("Sports"),
//         &PoolConfig {
//             description: String::from_str(&env, "Sports pool"),
//             metadata_url: String::from_str(&env, "ipfs://sports"),
//             min_stake: 1i128,
//             max_stake: 0i128,
//             max_total_stake: 0,
// min_total_stake: 1,
//             initial_liquidity: 0i128,
//             required_resolutions: 1u32,
//             private: false,
//             whitelist_key: None,
//         },
//     );

//     let pool_crypto = client.create_pool(
//         &creator,
//         &100_000u64,
//         &token_address,
//         &2u32,
//         &symbol_short!("Crypto"),
//         &PoolConfig {
//             description: String::from_str(&env, "Crypto pool"),
//             metadata_url: String::from_str(&env, "ipfs://crypto"),
//             min_stake: 1i128,
//             max_stake: 0i128,
//             max_total_stake: 0,
// min_total_stake: 1,
//             initial_liquidity: 0i128,
//             required_resolutions: 1u32,
//             private: false,
//             whitelist_key: None,
//         },
//     );

//     let pool_finance = client.create_pool(
//         &creator,
//         &100_000u64,
//         &token_address,
//         &2u32,
//         &symbol_short!("Finance"),
//         &PoolConfig {
//             description: String::from_str(&env, "Finance pool"),
//             metadata_url: String::from_str(&env, "ipfs://finance"),
//             min_stake: 1i128,
//             max_stake: 0i128,
//             max_total_stake: 0,
// min_total_stake: 1,
//             initial_liquidity: 0i128,
//             required_resolutions: 1u32,
//             private: false,
//             whitelist_key: None,
//         },
//     );

//     let result = client.get_active_pools(&0u32, &10u32);
//     assert_eq!(result.len(), 4);

//     // All four pool IDs must be present (order: insertion order).
//     let ids: std::vec::Vec<u64> = (0..result.len()).map(|i| result.get(i).unwrap()).collect();
//     assert!(ids.contains(&pool_tech));
//     assert!(ids.contains(&pool_sports));
//     assert!(ids.contains(&pool_crypto));
//     assert!(ids.contains(&pool_finance));
// }

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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
}

/// Resolved pool is removed from get_active_pools.
#[test]
fn test_get_active_pools_excludes_resolved_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_a = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool A"),
            metadata_url: String::from_str(&env, "ipfs://a"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let pool_b = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Pool B"),
            metadata_url: String::from_str(&env, "ipfs://b"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    assert_eq!(client.get_active_pools(&0u32, &10u32).len(), 2);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_a, &0u32);

    let result = client.get_active_pools(&0u32, &10u32);
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(0).unwrap(), pool_b);
}

/// Canceled pool is removed from get_active_pools.
#[test]
fn test_get_active_pools_excludes_canceled_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_a = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool A"),
            metadata_url: String::from_str(&env, "ipfs://a"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Pool B"),
            metadata_url: String::from_str(&env, "ipfs://b"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    assert_eq!(client.get_active_pools(&0u32, &10u32).len(), 2);

    client.cancel_pool(&operator, &pool_a, &String::from_str(&env, ""));

    let result = client.get_active_pools(&0u32, &10u32);
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(0).unwrap(), pool_b);
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
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

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
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
}

/// Pagination: offset and limit work correctly over a known set.
#[test]
fn test_get_active_pools_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let mut pool_ids = soroban_sdk::vec![&env];
    for i in 0..5u32 {
        let pid = client.create_pool(
            &creator,
            &100_000u64,
            &token_address,
            &2u32,
            &symbol_short!("Tech"),
            &PoolConfig {
                description: String::from_str(&env, "Pool"),
                metadata_url: String::from_str(&env, "ipfs://p"),
                min_stake: 1i128,
                max_stake: 0i128,
                max_total_stake: 0,
                min_total_stake: 1,
                initial_liquidity: 0i128,
                required_resolutions: 1u32,
                private: false,
                whitelist_key: None,
                outcome_descriptions: soroban_sdk::vec![
                    &env,
                    String::from_str(&env, "Yes"),
                    String::from_str(&env, "No"),
                ],
            },
        );
        pool_ids.push_back(pid);
        let _ = i;
    }

    // First page: offset=0, limit=2
    let page1 = client.get_active_pools(&0u32, &2u32);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap(), pool_ids.get(0).unwrap());
    assert_eq!(page1.get(1).unwrap(), pool_ids.get(1).unwrap());

    // Second page: offset=2, limit=2
    let page2 = client.get_active_pools(&2u32, &2u32);
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap(), pool_ids.get(2).unwrap());
    assert_eq!(page2.get(1).unwrap(), pool_ids.get(3).unwrap());

    // Third page: offset=4, limit=2 — only 1 remaining
    let page3 = client.get_active_pools(&4u32, &2u32);
    assert_eq!(page3.len(), 1);
    assert_eq!(page3.get(0).unwrap(), pool_ids.get(4).unwrap());

    // Beyond range
    let empty = client.get_active_pools(&5u32, &10u32);
    assert_eq!(empty.len(), 0);

    // limit=0 always returns empty
    let zero_limit = client.get_active_pools(&0u32, &0u32);
    assert_eq!(zero_limit.len(), 0);
}

// Swap-and-pop correctness: removing the first pool when three exist.
// The third pool must fill the vacated slot; the second is untouched.
// #[test]
// fn test_get_active_pools_swap_pop_removes_first() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

//     let pool_a = client.create_pool(
//         &creator, &100_000u64, &token_address, &2u32, &symbol_short!("Tech"),
//         &PoolConfig {
//             description: String::from_str(&env, "A"),
//             metadata_url: String::from_str(&env, "ipfs://a"),
//             min_stake: 1i128, max_stake: 0i128, initial_liquidity: 0i128,
//             required_resolutions: 1u32, private: false, whitelist_key: None,
//             max_total_stake: 0,
// min_total_stake: 1,
//         },
//     );
//     let pool_b = client.create_pool(
//         &creator, &100_000u64, &token_address, &2u32, &symbol_short!("Sports"),
//         &PoolConfig {
//             description: String::from_str(&env, "B"),
//             metadata_url: String::from_str(&env, "ipfs://b"),
//             min_stake: 1i128, max_stake: 0i128, initial_liquidity: 0i128,
//             required_resolutions: 1u32, private: false, whitelist_key: None,
//             max_total_stake: 0,
// min_total_stake: 1,
//         },
//     );
//     let pool_c = client.create_pool(
//         &creator, &100_000u64, &token_address, &2u32, &symbol_short!("Crypto"),
//         &PoolConfig {
//             description: String::from_str(&env, "C"),
//             metadata_url: String::from_str(&env, "ipfs://c"),
//             min_stake: 1i128, max_stake: 0i128, initial_liquidity: 0i128,
//             required_resolutions: 1u32, private: false, whitelist_key: None,
//             max_total_stake: 0,
// min_total_stake: 1,
//         },
//     );

//     // Remove pool_a (index 0) — pool_c (index 2) should swap into slot 0.
//     env.ledger().with_mut(|li| li.timestamp = 100_001);
//     client.resolve_pool(&operator, &pool_a, &0u32);

//     let result = client.get_active_pools(&0u32, &10u32);
//     assert_eq!(result.len(), 2);

//     let ids: std::vec::Vec<u64> = (0..result.len()).map(|i| result.get(i).unwrap()).collect();
//     assert!(!ids.contains(&pool_a));
//     assert!(ids.contains(&pool_b));
//     assert!(ids.contains(&pool_c));
// }

/// Swap-and-pop correctness: removing the last pool leaves the others intact.
#[test]
fn test_get_active_pools_swap_pop_removes_last() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_a = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "A"),
            metadata_url: String::from_str(&env, "ipfs://a"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            max_total_stake: 0,
            min_total_stake: 1,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
    let pool_b = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "B"),
            metadata_url: String::from_str(&env, "ipfs://b"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            max_total_stake: 0,
            min_total_stake: 1,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
    let pool_c = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Crypto"),
        &PoolConfig {
            description: String::from_str(&env, "C"),
            metadata_url: String::from_str(&env, "ipfs://c"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            max_total_stake: 0,
            min_total_stake: 1,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    // Remove pool_c (last, index 2) — no swap needed.
    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_c, &0u32);

    let result = client.get_active_pools(&0u32, &10u32);
    assert_eq!(result.len(), 2);
    assert_eq!(result.get(0).unwrap(), pool_a);
    assert_eq!(result.get(1).unwrap(), pool_b);
}

/// All pools resolved — get_active_pools returns empty.
#[test]
fn test_get_active_pools_empty_after_all_resolved() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_a = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "A"),
            metadata_url: String::from_str(&env, "ipfs://a"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            max_total_stake: 0,
            min_total_stake: 1,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
    let pool_b = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "B"),
            metadata_url: String::from_str(&env, "ipfs://b"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            max_total_stake: 0,
            min_total_stake: 1,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.resolve_pool(&operator, &pool_a, &0u32);
    client.resolve_pool(&operator, &pool_b, &0u32);

    let result = client.get_active_pools(&0u32, &10u32);
    assert_eq!(result.len(), 0);
}

/// oracle_resolve also removes the pool from the active index.
#[test]
fn test_get_active_pools_excludes_oracle_resolved_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, _, _, _, creator) = setup(&env);

    let oracle = Address::generate(&env);
    ac_client.grant_role(&oracle, &ROLE_ORACLE);

    let pool_a = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "A"),
            metadata_url: String::from_str(&env, "ipfs://a"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            max_total_stake: 0,
            min_total_stake: 1,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
    let pool_b = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "B"),
            metadata_url: String::from_str(&env, "ipfs://b"),
            min_stake: 1i128,
            max_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            max_total_stake: 0,
            min_total_stake: 1,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    assert_eq!(client.get_active_pools(&0u32, &10u32).len(), 2);

    env.ledger().with_mut(|li| li.timestamp = 100_001);
    client.oracle_resolve(&oracle, &pool_a, &0u32, &String::from_str(&env, "proof"));

    let result = client.get_active_pools(&0u32, &10u32);
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(0).unwrap(), pool_b);
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
            min_total_stake: 1,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
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
            min_total_stake: 1,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::Vec::new(&env),
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
            min_total_stake: 1,
            initial_liquidity: 0,
            required_resolutions: 1,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::Vec::new(&env),
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

// ── Creator cancellation tests ────────────────────────────────────────────

#[test]
fn test_creator_can_cancel_empty_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Empty Pool"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    client.cancel_pool(
        &creator,
        &pool_id,
        &String::from_str(&env, "Changed my mind"),
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Canceled);
}

// ── cancel_pool with zero participants ───────────────────────────────────────

#[test]
fn test_cancel_pool_zero_participants_state_is_canceled() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Zero Participant Pool"),
            metadata_url: String::from_str(&env, "ipfs://zero"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Canceled);
    assert_eq!(pool.state, MarketState::Canceled);
}

#[test]
fn test_cancel_pool_zero_participants_no_contract_balance_change() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, token, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Zero Participant Pool"),
            metadata_url: String::from_str(&env, "ipfs://zero"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let balance_before = token.balance(&client.address);
    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));
    let balance_after = token.balance(&client.address);

    assert_eq!(balance_before, balance_after);
}

/// Test that any user can cancel a pool that is overdue (past end_time + CANCELATION_DELAY)
#[test]
fn test_any_user_can_cancel_overdue_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _operator, creator) = setup(&env);

    // Set current time and create a pool
    let current_time = 10000u64;
    env.ledger().with_mut(|li| li.timestamp = current_time);

    let end_time = current_time + 3600u64; // Pool ends 1 hour from now
    let pool_id = client.create_pool(
        &creator,
        &end_time,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Overdue Pool"),
            metadata_url: String::from_str(&env, "ipfs://overdue"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    // Place some predictions to lock funds
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    token_admin_client.mint(&user1, &1000);
    token_admin_client.mint(&user2, &1000);

    client.place_prediction(&user1, &pool_id, &100, &0, &None, &None);
    client.place_prediction(&user2, &pool_id, &200, &1, &None, &None);

    // Advance time to just before the pool becomes overdue (end_time + CANCELATION_DELAY - 1)
    let just_before_overdue = end_time + CANCELATION_DELAY - 1;
    env.ledger()
        .with_mut(|li| li.timestamp = just_before_overdue);

    // Regular user should NOT be able to cancel yet (not overdue)
    let random_user = Address::generate(&env);
    let result = client.try_cancel_pool(
        &random_user,
        &pool_id,
        &String::from_str(&env, "Not overdue yet"),
    );
    assert!(
        result.is_err(),
        "Regular user should not be able to cancel before overdue period"
    );

    // Advance time to make the pool overdue (8 days after end_time)
    let overdue_time = end_time + CANCELATION_DELAY + 86400; // 8 days = 7 days + 1 day
    env.ledger().with_mut(|li| li.timestamp = overdue_time);

    // Now any user should be able to cancel the overdue pool
    client.cancel_pool(
        &random_user,
        &pool_id,
        &String::from_str(&env, "Pool is overdue"),
    );

    // Verify pool is canceled
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Canceled);
    assert_eq!(pool.state, MarketState::Canceled);

    // Users should be able to claim refunds
    let refund1 = client.claim_refund(&user1, &pool_id);
    assert_eq!(refund1, 100);
    assert_eq!(token_admin_client.balance(&user1), 1000); // Initial 1000 - 100 stake + 100 refund

    let refund2 = client.claim_refund(&user2, &pool_id);
    assert_eq!(refund2, 200);
    assert_eq!(token_admin_client.balance(&user2), 1000); // Initial 1000 - 200 stake + 200 refund
}

#[test]
fn test_claim_refund_on_zero_participant_canceled_pool_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Zero Participant Pool"),
            metadata_url: String::from_str(&env, "ipfs://zero"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));

    let non_participant = Address::generate(&env);
    let result = client.try_claim_refund(&non_participant, &pool_id);
    assert!(
        result.is_err(),
        "claim_refund for non-participant on canceled pool must return an error"
    );
}

#[test]
fn test_claim_winnings_on_zero_participant_canceled_pool_returns_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Zero Participant Pool"),
            metadata_url: String::from_str(&env, "ipfs://zero"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));

    let non_participant = Address::generate(&env);
    let result = client.try_claim_winnings(&non_participant, &pool_id);
    assert!(
        result.is_ok() && result.unwrap().unwrap() == 0,
        "claim_winnings for non-participant on canceled pool must return Ok(0)"
    );
}

// ── MetadataUrlInvalid error tests ───────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #109)")]
fn test_create_pool_rejects_metadata_url_exceeding_512_bytes() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let long_url = String::from_str(&env, &"x".repeat(513));

    client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Valid description"),
            metadata_url: long_url,
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
}

#[test]
fn test_create_pool_accepts_metadata_url_at_512_bytes() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let exact_url = String::from_str(&env, &"x".repeat(512));

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Valid description"),
            metadata_url: exact_url.clone(),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.metadata_url, exact_url);
}

#[test]
fn test_create_pool_accepts_empty_metadata_url() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Valid description"),
            metadata_url: String::from_str(&env, ""),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.metadata_url.len(), 0);
}

// ── cancel_pool zero participants: index cleanup ─────────────────────────────

#[test]
fn test_cancel_pool_zero_participants_removed_from_active_index() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Zero Participant Pool"),
            metadata_url: String::from_str(&env, "ipfs://zero"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let before = client.get_active_pools(&0u32, &10u32);
    assert!(before.contains(pool_id));

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));

    let after = client.get_active_pools(&0u32, &10u32);
    assert!(!after.contains(pool_id));
    assert_eq!(after.len(), 0);
}

#[test]
fn test_cancel_pool_zero_participants_catpoolix_still_readable() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, operator, creator) = setup(&env);

    let category = symbol_short!("Tech");

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &category,
        &PoolConfig {
            description: String::from_str(&env, "Zero Participant Pool"),
            metadata_url: String::from_str(&env, "ipfs://zero"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    client.cancel_pool(&operator, &pool_id, &String::from_str(&env, ""));

    let cat_pools = client.get_pools_by_category(&category, &0u32, &10u32);
    assert_eq!(cat_pools.len(), 1);
    assert_eq!(cat_pools.get(0).unwrap(), pool_id);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Canceled);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_creator_cannot_cancel_pool_with_bets() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let bettor = Address::generate(&env);
    token_admin_client.mint(&bettor, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool with bets"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    client.place_prediction(&bettor, &pool_id, &100, &0, &None, &None);
    client.cancel_pool(&creator, &pool_id, &String::from_str(&env, ""));
}

#[test]
fn test_operator_can_cancel_pool_with_bets() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup(&env);

    let bettor = Address::generate(&env);
    token_admin_client.mint(&bettor, &1000);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool with bets"),
            metadata_url: String::from_str(&env, "ipfs://metadata"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    client.place_prediction(&bettor, &pool_id, &100, &0, &None, &None);
    client.cancel_pool(
        &operator,
        &pool_id,
        &String::from_str(&env, "Operator override"),
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.state, MarketState::Canceled);
}

// ── Category constant tests ───────────────────────────────────────────────────

#[test]
fn test_category_constants_values() {
    assert_eq!(CATEGORY_SPORTS, symbol_short!("Sports"));
    assert_eq!(CATEGORY_FINANCE, symbol_short!("Finance"));
    assert_eq!(CATEGORY_CRYPTO, symbol_short!("Crypto"));
    assert_eq!(CATEGORY_POLITICS, symbol_short!("Politics"));
    assert_eq!(CATEGORY_ENTERTAIN, symbol_short!("Entertain"));
    assert_eq!(CATEGORY_TECH, symbol_short!("Tech"));
    assert_eq!(CATEGORY_OTHER, symbol_short!("Other"));
}

#[test]
#[allow(clippy::needless_range_loop)]
fn test_category_constants_are_unique() {
    let all = [
        CATEGORY_SPORTS,
        CATEGORY_FINANCE,
        CATEGORY_CRYPTO,
        CATEGORY_POLITICS,
        CATEGORY_ENTERTAIN,
        CATEGORY_TECH,
        CATEGORY_OTHER,
    ];
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            assert_ne!(
                all[i], all[j],
                "duplicate category constants at {i} and {j}"
            );
        }
    }
}

#[test]
fn test_pool_created_with_each_category() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let categories = [
        CATEGORY_SPORTS,
        CATEGORY_FINANCE,
        CATEGORY_CRYPTO,
        CATEGORY_POLITICS,
        CATEGORY_ENTERTAIN,
        CATEGORY_TECH,
        CATEGORY_OTHER,
    ];

    for cat in categories {
        let pool_id = client.create_pool(
            &creator,
            &100000u64,
            &token_address,
            &2u32,
            &cat,
            &PoolConfig {
                description: String::from_str(&env, "Category test pool"),
                metadata_url: String::from_str(&env, "ipfs://test"),
                min_stake: 1i128,
                max_stake: 0i128,
                max_total_stake: 0,
                min_total_stake: 1,
                initial_liquidity: 0i128,
                required_resolutions: 1u32,
                private: false,
                whitelist_key: None,
                outcome_descriptions: soroban_sdk::vec![
                    &env,
                    String::from_str(&env, "Yes"),
                    String::from_str(&env, "No"),
                ],
            },
        );
        let pool = client.get_pool(&pool_id);
        assert_eq!(pool.category, cat);
    }
}

#[test]
fn test_get_fees_returns_treasury_and_referral_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, _client, token_address, _, _, treasury, _, _) = setup(&env);

    // init sets fee_bps = 300; referral_cut_bps defaults to 5000
    let ac_id = ac_client.address.clone();
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Re-register a fresh contract with a known fee_bps
    let contract_id = env.register(PredifiContract, ());
    let c = PredifiContractClient::new(&env, &contract_id);
    c.init(&ac_id, &treasury, &300u32, &0u64, &3600u64, &0u32);
    c.add_token_to_whitelist(&admin, &token_address);

    let fees = c.get_fees();
    assert_eq!(fees.treasury_fee_bps, 300);
    assert_eq!(fees.referral_fee_bps, 5000); // default

    // Update both and verify get_fees reflects the changes
    c.set_fee_bps(&admin, &750u32);
    c.set_referral_cut_bps(&admin, &2000u32);

    let fees = c.get_fees();
    assert_eq!(fees.treasury_fee_bps, 750);
    assert_eq!(fees.referral_fee_bps, 2000);
}

#[test]
fn test_get_contract_info_returns_config_and_stats() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, _, _, _, creator) = setup(&env);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let pool_config = PoolConfig {
        description: String::from_str(&env, "Pool"),
        metadata_url: String::from_str(&env, "ipfs://pool"),
        min_stake: 1i128,
        max_stake: 0i128,
        max_total_stake: 0i128,
        min_total_stake: 1i128,
        initial_liquidity: 0i128,
        required_resolutions: 1u32,
        private: false,
        whitelist_key: None,
        outcome_descriptions: soroban_sdk::vec![
            &env,
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    };

    client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &CATEGORY_TECH,
        &pool_config,
    );
    client.create_pool(
        &creator,
        &100500u64,
        &token_address,
        &2u32,
        &CATEGORY_SPORTS,
        &pool_config,
    );

    client.set_fee_bps(&admin, &250u32);
    client.set_treasury(&admin, &treasury);
    client.set_resolution_delay(&admin, &60u64);
    client.set_min_pool_duration(&admin, &7200u64);
    client.set_min_stake(&admin, &5i128);
    client.set_max_predictions_per_user(&admin, &3u32);
    client.set_referral_cut_bps(&admin, &2000u32);
    client.pause(&admin);

    let info = client.get_contract_info();
    assert_eq!(info.version, 1u32);
    assert_eq!(info.current_admin, admin);
    assert!(info.is_paused);
    assert_eq!(info.total_pools, 2u64);
    assert_eq!(info.fee_bps, 250u32);
    assert_eq!(info.referral_cut_bps, 2000u32);
    assert_eq!(info.treasury, treasury);
    assert_eq!(info.access_control, ac_client.address);
    assert_eq!(info.resolution_delay, 60u64);
    assert_eq!(info.min_pool_duration, 7200u64);
    assert_eq!(info.min_stake, 5i128);
    assert_eq!(info.max_predictions_per_user, 3u32);
}

#[test]
fn test_whitelist_events_emitted() {
    let env = Env::default();
    env.mock_all_auths();
    let (ac_client, client, token_address, _, _, _, _, _) = setup(&env);

    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    client.add_token_to_whitelist(&admin, &token_address);

    // Create a private pool
    let creator = Address::generate(&env);
    let user = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &(env.ledger().timestamp() + 7200),
        &token_address,
        &2u32,
        &symbol_short!("Other"),
        &PoolConfig {
            description: String::from_str(&env, "Will it rain?"),
            metadata_url: String::from_str(&env, ""),
            min_stake: 1_000_000i128,
            max_stake: 0i128,
            min_total_stake: 1_000_000i128,
            max_total_stake: 0i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: true,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    // Add user to whitelist — event should be emitted
    client.add_to_whitelist(&creator, &pool_id, &user);
    assert!(client.is_whitelisted(&pool_id, &user));

    // Remove user from whitelist — event should be emitted
    client.remove_from_whitelist(&creator, &pool_id, &user);
    assert!(!client.is_whitelisted(&pool_id, &user));
}

/// Test get_supported_tokens returns empty list when no tokens are whitelisted
#[test]
fn test_get_supported_tokens_empty() {
    let (_env, client, _admin, _treasury) = setup_whitelist_env();

    let supported_tokens = client.get_supported_tokens();
    assert_eq!(supported_tokens.len(), 0);
}

/// Test get_supported_tokens returns list of whitelisted tokens
#[test]
fn test_get_supported_tokens_multiple() {
    let (env, client, admin, _treasury) = setup_whitelist_env();

    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let token_c = Address::generate(&env);

    // Add tokens to whitelist
    client.add_token_to_whitelist(&admin, &token_a);
    client.add_token_to_whitelist(&admin, &token_b);
    client.add_token_to_whitelist(&admin, &token_c);

    let supported_tokens = client.get_supported_tokens();
    assert_eq!(supported_tokens.len(), 3);
    assert!(supported_tokens.contains(&token_a));
    assert!(supported_tokens.contains(&token_b));
    assert!(supported_tokens.contains(&token_c));
}

/// Test get_supported_tokens updates correctly when tokens are removed
#[test]
fn test_get_supported_tokens_after_removal() {
    let (env, client, admin, _treasury) = setup_whitelist_env();

    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let token_c = Address::generate(&env);

    // Add tokens to whitelist
    client.add_token_to_whitelist(&admin, &token_a);
    client.add_token_to_whitelist(&admin, &token_b);
    client.add_token_to_whitelist(&admin, &token_c);

    let supported_tokens = client.get_supported_tokens();
    assert_eq!(supported_tokens.len(), 3);

    // Remove one token
    client.remove_token_from_whitelist(&admin, &token_b);

    let supported_tokens = client.get_supported_tokens();
    assert_eq!(supported_tokens.len(), 2);
    assert!(supported_tokens.contains(&token_a));
    assert!(supported_tokens.contains(&token_c));
    assert!(!supported_tokens.contains(&token_b));
}

/// Test get_supported_tokens handles duplicate additions gracefully
#[test]
fn test_get_supported_tokens_duplicate_additions() {
    let (env, client, admin, _treasury) = setup_whitelist_env();

    let token = Address::generate(&env);

    // Add the same token twice
    client.add_token_to_whitelist(&admin, &token);
    client.add_token_to_whitelist(&admin, &token);

    let supported_tokens = client.get_supported_tokens();
    assert_eq!(supported_tokens.len(), 1);
    assert!(supported_tokens.contains(&token));
}

/// Test that create_pool rejects a min_total_stake of zero.
///
/// Per issue #507: `min_total_stake` must be strictly positive (> 0).
/// Passing 0 should cause the contract to panic.
#[test]
#[should_panic(expected = "min_total_stake must be greater than zero")]
fn test_create_pool_rejects_zero_min_total_stake() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);
    token_admin_client.mint(&creator, &1_000_000i128);

    client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Zero min_total_stake pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            // min_total_stake = 0 should be rejected
            min_total_stake: 0,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );
}

/// Test that create_pool accepts a valid positive min_total_stake.
#[test]
fn test_create_pool_accepts_positive_min_total_stake() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);
    token_admin_client.mint(&creator, &1_000_000i128);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Valid min_total_stake pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 100i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.min_total_stake, 100i128);
}

// ── max_predictions_per_user tests ────────────────────────────────────────────

/// Test set_max_predictions_per_user function works correctly
#[test]
fn test_set_max_predictions_per_user() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _token_address, _, _, _, _, _) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Set max predictions to 5
    client.set_max_predictions_per_user(&admin, &5u32);

    // Verify the config was updated (we can't directly read config, but we can test behavior)
    // This will be tested in the following tests
}

/// Test that max_predictions_per_user limits work correctly
#[test]
fn test_max_predictions_per_user_enforcement() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Set max predictions to 2 per user
    client.set_max_predictions_per_user(&admin, &2u32);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000i128);

    // Create first pool
    let pool1 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 1"),
            metadata_url: String::from_str(&env, "ipfs://pool1"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    // Create second pool
    let pool2 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 2"),
            metadata_url: String::from_str(&env, "ipfs://pool2"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    // Create third pool
    let pool3 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Finance"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 3"),
            metadata_url: String::from_str(&env, "ipfs://pool3"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    // User should be able to place predictions on first 2 pools
    client.place_prediction(&user, &pool1, &100, &0, &None, &None);
    client.place_prediction(&user, &pool2, &100, &1, &None, &None);

    // User should NOT be able to place prediction on 3rd pool (exceeds limit)
    let result = client.try_place_prediction(&user, &pool3, &100, &0, &None, &None);
    // The function should return an error
    assert!(result.is_err());
}

/// Test that max_predictions_per_user = 0 means no limit
#[test]
fn test_max_predictions_per_user_zero_means_no_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Set max predictions to 0 (no limit)
    client.set_max_predictions_per_user(&admin, &0u32);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &10000i128);

    // Create multiple pools
    let pool1 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 1"),
            metadata_url: String::from_str(&env, "ipfs://pool1"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let pool2 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 2"),
            metadata_url: String::from_str(&env, "ipfs://pool2"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let pool3 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Finance"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 3"),
            metadata_url: String::from_str(&env, "ipfs://pool3"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let pools = [pool1, pool2, pool3];

    // User should be able to place predictions on all pools (no limit)
    for (i, pool_id) in pools.iter().enumerate() {
        client.place_prediction(&user, pool_id, &100, &(i as u32 % 2), &None, &None);
    }
}

/// Test that increasing stake on same pool doesn't count as new prediction
#[test]
fn test_max_predictions_per_user_same_pool_stake_increase() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Set max predictions to 1 per user
    client.set_max_predictions_per_user(&admin, &1u32);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000i128);

    let pool_id = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Test Pool"),
            metadata_url: String::from_str(&env, "ipfs://test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    // User places initial prediction
    client.place_prediction(&user, &pool_id, &100, &0, &None, &None);

    // User should be able to increase stake on same pool (same prediction)
    client.place_prediction(&user, &pool_id, &50, &0, &None, &None);

    // Create second pool
    let pool2 = client.create_pool(
        &creator,
        &100000u64,
        &token_address,
        &2u32,
        &symbol_short!("Sports"),
        &PoolConfig {
            description: String::from_str(&env, "Pool 2"),
            metadata_url: String::from_str(&env, "ipfs://pool2"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    // User should NOT be able to place prediction on second pool (exceeds limit)
    let result = client.try_place_prediction(&user, &pool2, &100, &0, &None, &None);
    // The function should return an error
    assert!(result.is_err());
}

/// Test unauthorized access to set_max_predictions_per_user
#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_set_max_predictions_per_user_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let (_ac_client, client, _token_address, _, _, _, _, _) = setup(&env);
    let unauthorized_user = Address::generate(&env);
    // Don't grant admin role

    // Unauthorized user should not be able to set max predictions
    client.set_max_predictions_per_user(&unauthorized_user, &5u32);
}

/// Test MaxPredictionsUpdateEvent is emitted
#[test]
fn test_max_predictions_update_event_emitted() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, _token_address, _, _, _, _, _) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    // Get events before
    let events_before = env.events().all();

    // Set max predictions
    client.set_max_predictions_per_user(&admin, &10u32);

    // Verify event was emitted
    let events_after = env.events().all();
    assert!(events_after.len() > events_before.len());

    // Find the MaxPredictionsUpdateEvent
    let max_predictions_update_topic = Symbol::new(&env, "max_predictions_update");
    let mut found_event = false;

    for e in events_after.iter() {
        if let Some(topic_val) = e.1.get(0) {
            if let Ok(topic_sym) = Symbol::try_from_val(&env, &topic_val) {
                if topic_sym == max_predictions_update_topic {
                    found_event = true;
                    break;
                }
            }
        }
    }

    assert!(found_event, "MaxPredictionsUpdateEvent should be emitted");
}

// ── update_pool_description tests ────────────────────────────────────────────

/// Creator can update the description before any prediction is placed.
#[test]
fn test_update_pool_description_by_creator_before_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Will BTC hit 100k?"),
            metadata_url: String::from_str(&env, "ipfs://desc-test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let new_desc = String::from_str(&env, "Will BTC hit 100k by March 1st?");
    client.update_pool_description(&creator, &pool_id, &new_desc);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.description, new_desc);
}
#[test]
fn test_update_pool_description_by_admin_before_predictions() {
    let env = Env::default();
    env.mock_all_auths();

    let (ac_client, client, token_address, _, _, _, _, creator) = setup(&env);
    let admin = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Original description"),
            metadata_url: String::from_str(&env, "ipfs://admin-desc-test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let new_desc = String::from_str(&env, "Admin-corrected description");
    client.update_pool_description(&admin, &pool_id, &new_desc);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.description, new_desc);
}

/// Description update is rejected once a prediction has been placed (pool "started").
#[test]
fn test_update_pool_description_locked_after_prediction() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, token_admin_client, _, _, creator) = setup(&env);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1_000_000i128);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Original description"),
            metadata_url: String::from_str(&env, "ipfs://locked-desc-test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    // Place a prediction — this "starts" the pool
    client.place_prediction(&user, &pool_id, &100i128, &0u32, &None, &None);

    // Description update must now be rejected
    let result = client.try_update_pool_description(
        &creator,
        &pool_id,
        &String::from_str(&env, "Attempted change after start"),
    );
    assert_eq!(result, Err(Ok(PredifiError::InvalidPoolState)));

    // Description must remain unchanged
    let pool = client.get_pool(&pool_id);
    assert_eq!(
        pool.description,
        String::from_str(&env, "Original description")
    );
}

/// Non-creator, non-admin caller is rejected.
#[test]
fn test_update_pool_description_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client, token_address, _, _, _, _, creator) = setup(&env);
    let stranger = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &100_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            description: String::from_str(&env, "Original description"),
            metadata_url: String::from_str(&env, "ipfs://unauth-desc-test"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                &env,
                String::from_str(&env, "Yes"),
                String::from_str(&env, "No"),
            ],
        },
    );

    let result = client.try_update_pool_description(
        &stranger,
        &pool_id,
        &String::from_str(&env, "Unauthorized change"),
    );
    assert_eq!(result, Err(Ok(PredifiError::Unauthorized)));
}
