//! Stress Tests for Issues #1530 / #1897 — Maximum Pools Active Simultaneously
//!
//! Coverage:
//! - Create the maximum number of simultaneously active pools
//! - Pool enumeration performance with pagination
//! - Cross-pool user prediction queries
//! - Verify no storage collision between pools

#![cfg(test)]

extern crate std;

use crate::{MarketState, PoolConfig, PredifiContract, PredifiContractClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};

// ─── Shared dummy access-control stub ────────────────────────────────────────

mod ac_stub_1530 {
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

    #[contract]
    pub struct AcStub1530;

    #[contractimpl]
    impl AcStub1530 {
        pub fn grant_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user.clone(), role);
            let already: bool = env.storage().instance().get(&key).unwrap_or(false);
            env.storage().instance().set(&key, &true);
            if role == 1 && !already {
                let ck = Symbol::new(&env, "op_count");
                let c: u32 = env.storage().instance().get(&ck).unwrap_or(0);
                env.storage().instance().set(&ck, &(c + 1));
            }
        }

        pub fn revoke_role(env: Env, user: Address, role: u32) {
            let key = (Symbol::new(&env, "role"), user, role);
            let had: bool = env.storage().instance().get(&key).unwrap_or(false);
            env.storage().instance().set(&key, &false);
            if role == 1 && had {
                let ck = Symbol::new(&env, "op_count");
                let c: u32 = env.storage().instance().get(&ck).unwrap_or(0);
                if c > 0 {
                    env.storage().instance().set(&ck, &(c - 1));
                }
            }
        }

        pub fn has_role(env: Env, user: Address, role: u32) -> bool {
            let key = (Symbol::new(&env, "role"), user, role);
            env.storage().instance().get(&key).unwrap_or(false)
        }

        pub fn get_operator_count(env: Env) -> u32 {
            env.storage()
                .instance()
                .get(&Symbol::new(&env, "op_count"))
                .unwrap_or(0)
        }
    }
}

// ─── Helper: build a fresh environment with the contract ready ────────────────

fn max_pools_setup(
    env: &Env,
) -> (
    PredifiContractClient,
    Address, // admin
    Address, // operator
    Address, // token_address
    token::StellarAssetClient,
) {
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.protocol_version = 23;
        li.timestamp = 1_000;
    });

    let ac_id = env.register(ac_stub_1530::AcStub1530, ());
    let ac = ac_stub_1530::AcStub1530Client::new(env, &ac_id);

    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let treasury = Address::generate(env);

    ac.grant_role(&admin, &0u32);
    ac.grant_role(&operator, &1u32);

    let contract_id = env.register(PredifiContract, ());
    let client = PredifiContractClient::new(env, &contract_id);
    // min_pool_duration = 3600; resolution_delay = 0
    client.init(&ac_id, &treasury, &0u32, &0u64, &3600u64, &0u32);

    let token_admin_addr = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr);
    let token_address = token_contract.address();
    let token_admin = token::StellarAssetClient::new(env, &token_address);

    client.add_token_to_whitelist(&admin, &token_address);

    (client, admin, operator, token_address, token_admin)
}

/// `create_pool` requires one description per outcome; these pools are binary.
fn binary_outcomes(env: &Env) -> Vec<String> {
    Vec::from_array(
        env,
        [String::from_str(env, "No"), String::from_str(env, "Yes")],
    )
}

/// Build a minimal PoolConfig for bulk creation.
fn make_pool_config_1530(env: &Env, _label: u32) -> PoolConfig {
    PoolConfig {
        start_time: 0u64,
        description: String::from_str(env, "Pool config for stress test"),
        metadata_url: String::from_str(env, "https://predifi.app/pool"),
        min_stake: 1_000i128,
        max_stake: 0i128,
        min_total_stake: 1_000i128,
        max_total_stake: 0i128,
        initial_liquidity: 0i128,
        required_resolutions: 1u32,
        private: false,
        whitelist_key: None,
        outcome_descriptions: binary_outcomes(env),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// #1530 — Maximum Pools Active Simultaneously
// ═══════════════════════════════════════════════════════════════════════════

/// Creating 60 pools simultaneously must result in an active-pool count that
/// exactly matches the number of pools created.
#[test]
fn test_1530_max_pools_active_count_is_accurate() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, token_admin) = max_pools_setup(&env);

    let num_pools: u32 = 60;
    for i in 0..num_pools {
        let creator = Address::generate(&env);
        token_admin.mint(&creator, &10_000i128);
        client.create_pool(
            &creator,
            &(10_000u64 + i as u64),
            &token_address,
            &2u32,
            &symbol_short!("Tech"),
            &make_pool_config_1530(&env, i),
        );
    }

    let active_count = client.get_active_pools_count();
    assert_eq!(
        active_count, num_pools,
        "active pool count must equal the number of created pools"
    );
}

/// All pool IDs returned by the active-pool index must be unique — no storage
/// collisions or duplicate registrations must occur.
#[test]
fn test_1530_no_storage_collision_between_pools() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, token_admin) = max_pools_setup(&env);

    let num_pools: u32 = 50;
    let mut created_ids: std::vec::Vec<u64> = std::vec::Vec::new();

    for i in 0..num_pools {
        let creator = Address::generate(&env);
        token_admin.mint(&creator, &10_000i128);

        let pool_id = client.create_pool(
            &creator,
            &(20_000u64 + i as u64),
            &token_address,
            &2u32,
            &symbol_short!("Tech"),
            &make_pool_config_1530(&env, i),
        );
        created_ids.push(pool_id);
    }

    // All IDs must be unique.
    let mut unique: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for &id in &created_ids {
        assert!(
            unique.insert(id),
            "duplicate pool ID {} detected — storage collision",
            id
        );
    }
    assert_eq!(
        unique.len(),
        num_pools as usize,
        "all {} pool IDs must be unique",
        num_pools
    );

    // Every created pool must be independently retrievable.
    for &pool_id in &created_ids {
        let pool = client.get_pool(&pool_id);
        // Verify at least one pool-specific field is sensible.
        assert_eq!(
            pool.token, token_address,
            "retrieved pool token must match registered token"
        );
    }
}

/// Pool enumeration via `get_active_pools` with pagination must return every
/// created pool exactly once across all pages.
#[test]
fn test_1530_pool_enumeration_returns_all_pools_across_pages() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, token_admin) = max_pools_setup(&env);

    let num_pools: u32 = 60;
    let mut created_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();

    for i in 0..num_pools {
        let creator = Address::generate(&env);
        token_admin.mint(&creator, &10_000i128);
        let pool_id = client.create_pool(
            &creator,
            &(30_000u64 + i as u64),
            &token_address,
            &2u32,
            &symbol_short!("Tech"),
            &make_pool_config_1530(&env, i),
        );
        created_ids.insert(pool_id);
    }

    // Paginate in chunks of 20 and collect all returned IDs.
    let page_size: u32 = 20;
    let mut retrieved: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut offset: u32 = 0;

    loop {
        let page = client.get_active_pools(&offset, &page_size);
        if page.is_empty() {
            break;
        }
        for pool_id in page.iter() {
            retrieved.insert(pool_id);
        }
        offset += page_size;
        if offset >= num_pools + page_size {
            break;
        }
    }

    // Every created pool must appear in the combined pagination result.
    for &expected_id in &created_ids {
        assert!(
            retrieved.contains(&expected_id),
            "pool {} was created but not returned by pagination",
            expected_id
        );
    }

    // No extra pools may appear that were not created in this test.
    assert_eq!(
        retrieved.len(),
        created_ids.len(),
        "pagination must return exactly {} pools, got {}",
        created_ids.len(),
        retrieved.len()
    );
}

/// Pool enumeration performance: retrieving all active pools must complete
/// without errors when the pool count is large. Validates no catastrophic
/// performance degradation in the enumeration path.
#[test]
fn test_1530_pool_enumeration_single_page_retrieval() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, token_admin) = max_pools_setup(&env);

    let num_pools: u32 = 50;

    for i in 0..num_pools {
        let creator = Address::generate(&env);
        token_admin.mint(&creator, &10_000i128);
        client.create_pool(
            &creator,
            &(40_000u64 + i as u64),
            &token_address,
            &2u32,
            &symbol_short!("Tech"),
            &make_pool_config_1530(&env, i),
        );
    }

    // Single-page retrieval of all pools must succeed.
    let all_pools = client.get_active_pools(&0u32, &num_pools);
    assert_eq!(
        all_pools.len() as u32,
        num_pools,
        "single-page retrieval must return all {} pools",
        num_pools
    );

    // Verify no gaps: count matches length.
    let count = client.get_active_pools_count();
    assert_eq!(
        count,
        all_pools.len() as u32,
        "get_active_pools_count() must match the length of get_active_pools()"
    );
}

/// Cross-pool user prediction queries: a single user placing predictions across
/// multiple pools must be tracked correctly per pool, with no cross-pool
/// contamination of participant data.
#[test]
fn test_1530_cross_pool_user_prediction_queries() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, token_admin) = max_pools_setup(&env);

    let num_pools: u32 = 10;
    let mut pool_ids: std::vec::Vec<u64> = std::vec::Vec::new();

    for i in 0..num_pools {
        let creator = Address::generate(&env);
        token_admin.mint(&creator, &10_000i128);
        let pool_id = client.create_pool(
            &creator,
            &(50_000u64 + i as u64),
            &token_address,
            &2u32,
            &symbol_short!("Tech"),
            &make_pool_config_1530(&env, i),
        );
        pool_ids.push(pool_id);
    }

    // A single user places one prediction per pool.
    let user = Address::generate(&env);
    token_admin.mint(&user, &(1_000i128 * num_pools as i128));

    for &pool_id in &pool_ids {
        client.place_prediction(&user, &pool_id, &1_000i128, &0u32, &None, &None);
    }

    // Each pool must record exactly 1 participant (the user).
    for &pool_id in &pool_ids {
        let participants = client.get_pool_participants_count(&pool_id);
        assert_eq!(
            participants, 1,
            "pool {} must have exactly 1 participant",
            pool_id
        );
    }

    // Each pool must have the correct total stake.
    for &pool_id in &pool_ids {
        let pool = client.get_pool(&pool_id);
        assert_eq!(
            pool.total_stake, 1_000,
            "pool {} total stake must be 1_000",
            pool_id
        );
    }
}

/// Predictions placed in pool A must not appear in pool B — data isolation
/// across the maximum set of simultaneous pools must be airtight.
#[test]
fn test_1530_pool_data_isolation_no_cross_pool_contamination() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, token_admin) = max_pools_setup(&env);

    // Create two pools with different configurations.
    let creator_a = Address::generate(&env);
    let creator_b = Address::generate(&env);
    token_admin.mint(&creator_a, &10_000i128);
    token_admin.mint(&creator_b, &10_000i128);

    let pool_a = client.create_pool(
        &creator_a,
        &60_000u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Pool A"),
            metadata_url: String::from_str(&env, "ipfs://pool-a"),
            min_stake: 500i128,
            max_stake: 10_000i128,
            min_total_stake: 500i128,
            max_total_stake: 100_000i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: binary_outcomes(&env),
        },
    );

    let pool_b = client.create_pool(
        &creator_b,
        &60_001u64,
        &token_address,
        &2u32,
        &symbol_short!("Tech"),
        &PoolConfig {
            start_time: 0,
            description: String::from_str(&env, "Pool B"),
            metadata_url: String::from_str(&env, "ipfs://pool-b"),
            min_stake: 1_000i128,
            max_stake: 50_000i128,
            min_total_stake: 1_000i128,
            max_total_stake: 200_000i128,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: binary_outcomes(&env),
        },
    );

    // Place stakes only in pool_a.
    let user_a = Address::generate(&env);
    token_admin.mint(&user_a, &5_000i128);
    client.place_prediction(&user_a, &pool_a, &5_000i128, &0u32, &None, &None);

    // pool_b must remain untouched.
    assert_eq!(
        client.get_pool(&pool_b).total_stake,
        0,
        "pool_b must have zero stake — not contaminated by pool_a predictions"
    );
    assert_eq!(
        client.get_pool_participants_count(&pool_b),
        0,
        "pool_b must have zero participants"
    );

    // pool_a must reflect only user_a's stake.
    assert_eq!(client.get_pool(&pool_a).total_stake, 5_000);
    assert_eq!(client.get_pool_participants_count(&pool_a), 1);

    // Each pool's configuration must remain independent.
    let config_a = client.get_pool_config(&pool_a);
    let config_b = client.get_pool_config(&pool_b);
    assert_eq!(config_a.min_stake, 500i128);
    assert_eq!(config_b.min_stake, 1_000i128);
    assert_ne!(
        config_a.min_stake, config_b.min_stake,
        "pool configurations must be independent"
    );
}

/// Pool category indexing must be consistent: pools with the same category
/// must all appear in the category index with no missing or cross-category entries.
#[test]
fn test_1530_category_index_consistent_across_max_pools() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, token_admin) = max_pools_setup(&env);

    let num_tech_pools: u32 = 20;
    let num_sports_pools: u32 = 10;

    let mut tech_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut sports_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();

    // Create Tech pools.
    for i in 0..num_tech_pools {
        let creator = Address::generate(&env);
        token_admin.mint(&creator, &10_000i128);
        let pool_id = client.create_pool(
            &creator,
            &(70_000u64 + i as u64),
            &token_address,
            &2u32,
            &symbol_short!("Tech"),
            &make_pool_config_1530(&env, i),
        );
        tech_ids.insert(pool_id);
    }

    // Create Sports pools.
    for i in 0..num_sports_pools {
        let creator = Address::generate(&env);
        token_admin.mint(&creator, &10_000i128);
        let pool_id = client.create_pool(
            &creator,
            &(80_000u64 + i as u64),
            &token_address,
            &2u32,
            &symbol_short!("Sports"),
            &make_pool_config_1530(&env, num_tech_pools + i),
        );
        sports_ids.insert(pool_id);
    }

    // Category query must return all Tech pools.
    let tech_result =
        client.get_pools_by_category(&symbol_short!("Tech"), &0u32, &(num_tech_pools + 10));
    assert_eq!(
        tech_result.len() as u32,
        num_tech_pools,
        "Tech category must contain exactly {} pools",
        num_tech_pools
    );

    for returned_id in tech_result.iter() {
        assert!(
            tech_ids.contains(&returned_id),
            "pool {} in Tech category was not created as a Tech pool",
            returned_id
        );
        assert!(
            !sports_ids.contains(&returned_id),
            "pool {} from Sports appeared in Tech category",
            returned_id
        );
    }

    // Category query must return all Sports pools.
    let sports_result =
        client.get_pools_by_category(&symbol_short!("Sports"), &0u32, &(num_sports_pools + 10));
    assert_eq!(
        sports_result.len() as u32,
        num_sports_pools,
        "Sports category must contain exactly {} pools",
        num_sports_pools
    );

    for returned_id in sports_result.iter() {
        assert!(
            sports_ids.contains(&returned_id),
            "pool {} in Sports category was not created as a Sports pool",
            returned_id
        );
    }
}

/// Auto-incrementing pool IDs must be strictly monotonically increasing when
/// pools are created sequentially. No gaps and no reuse.
#[test]
fn test_1530_pool_ids_are_monotonically_increasing() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, token_admin) = max_pools_setup(&env);

    let num_pools: u32 = 30;
    let mut last_id: Option<u64> = None;

    for i in 0..num_pools {
        let creator = Address::generate(&env);
        token_admin.mint(&creator, &10_000i128);
        let pool_id = client.create_pool(
            &creator,
            &(90_000u64 + i as u64),
            &token_address,
            &2u32,
            &symbol_short!("Tech"),
            &make_pool_config_1530(&env, i),
        );

        if let Some(prev) = last_id {
            assert!(
                pool_id > prev,
                "pool ID {} must be strictly greater than previous ID {}",
                pool_id,
                prev
            );
        }
        last_id = Some(pool_id);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// #1897 — user queries, index churn and enumeration cost across many pools
// ═══════════════════════════════════════════════════════════════════════════

/// Create a binary "Tech" pool with the shared stress config.
fn create_binary_pool(
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
        &make_pool_config_1530(env, 0),
    )
}

/// One user predicting across many pools must see exactly their own
/// predictions, in order and across pages, and every pool must hold only the
/// stake that was placed on it (no bleed between pool storage entries).
#[test]
fn test_1897_cross_pool_user_queries_are_isolated_per_pool() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, token_admin) = max_pools_setup(&env);
    let token = token::Client::new(&env, &token_address);

    let num_pools: u32 = 24;
    let creator = Address::generate(&env);
    let pool_ids: std::vec::Vec<u64> = (0..num_pools)
        .map(|i| create_binary_pool(&env, &client, &creator, &token_address, 10_000 + i as u64))
        .collect();

    // alice predicts in every pool (outcome = i % 2, stake grows with i);
    // bob only in the even pools, always on outcome 1.
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    token_admin.mint(&alice, &1_000_000i128);
    token_admin.mint(&bob, &1_000_000i128);
    let alice_stake = |i: u32| 1_000i128 + 10 * i as i128;
    let bob_stake = |i: u32| 2_000i128 + i as i128;

    let mut total_staked = 0i128;
    for (i, &pool_id) in pool_ids.iter().enumerate() {
        let i = i as u32;
        client.place_prediction(&alice, &pool_id, &alice_stake(i), &(i % 2), &None, &None);
        total_staked += alice_stake(i);
        if i.is_multiple_of(2) {
            client.place_prediction(&bob, &pool_id, &bob_stake(i), &1u32, &None, &None);
            total_staked += bob_stake(i);
        }
    }

    // Each pool holds exactly the stake placed on it.
    for (i, &pool_id) in pool_ids.iter().enumerate() {
        let i = i as u32;
        let mut expected = [0i128, 0i128];
        expected[(i % 2) as usize] += alice_stake(i);
        let mut participants = 1u32;
        if i.is_multiple_of(2) {
            expected[1] += bob_stake(i);
            participants = 2;
        }

        let stakes = client.get_pool_outcome_stakes(&pool_id);
        assert_eq!(
            stakes.get(0).unwrap(),
            expected[0],
            "pool {} outcome 0",
            pool_id
        );
        assert_eq!(
            stakes.get(1).unwrap(),
            expected[1],
            "pool {} outcome 1",
            pool_id
        );
        assert_eq!(
            client.get_pool(&pool_id).total_stake,
            expected[0] + expected[1]
        );
        assert_eq!(client.get_pool_participants_count(&pool_id), participants);
    }
    assert_eq!(token.balance(&client.address), total_staked);

    // alice's whole history in one page: one entry per pool, in order.
    let history = client.get_user_predictions(&alice, &0u32, &num_pools);
    assert_eq!(history.len(), num_pools);
    for (i, detail) in history.iter().enumerate() {
        let i = i as u32;
        assert_eq!(detail.pool_id, pool_ids[i as usize]);
        assert_eq!(detail.amount, alice_stake(i));
        assert_eq!(detail.user_outcome, i % 2);
        assert_eq!(detail.pool_state, MarketState::Active);
    }

    // The same history read in pages of 5 adds up to the same list.
    let mut paged_ids: std::vec::Vec<u64> = std::vec::Vec::new();
    let mut offset = 0u32;
    loop {
        let page = client.get_user_predictions(&alice, &offset, &5u32);
        if page.is_empty() {
            break;
        }
        assert!(page.len() <= 5);
        paged_ids.extend(page.iter().map(|detail| detail.pool_id));
        offset += 5;
    }
    assert_eq!(paged_ids, pool_ids);

    // bob only sees the even pools; a stranger sees nothing.
    let bob_history = client.get_user_predictions(&bob, &0u32, &num_pools);
    assert_eq!(bob_history.len(), num_pools / 2);
    for (k, detail) in bob_history.iter().enumerate() {
        assert_eq!(detail.pool_id, pool_ids[2 * k]);
        assert_eq!(detail.amount, bob_stake(2 * k as u32));
        assert_eq!(detail.user_outcome, 1);
    }
    let stranger = Address::generate(&env);
    assert!(client
        .get_user_predictions(&stranger, &0u32, &num_pools)
        .is_empty());
    assert!(client
        .get_user_predictions(&alice, &num_pools, &5u32)
        .is_empty());
}

/// Pools leaving the active set (canceled here) must be removed from the
/// enumeration without leaving gaps or duplicates, and later pools must still
/// be indexed and numbered correctly.
#[test]
fn test_1897_active_index_stays_consistent_when_pools_leave() {
    let env = Env::default();
    let (client, _admin, operator, token_address, _token_admin) = max_pools_setup(&env);

    let num_pools: u32 = 36;
    let creator = Address::generate(&env);
    let pool_ids: std::vec::Vec<u64> = (0..num_pools)
        .map(|i| create_binary_pool(&env, &client, &creator, &token_address, 10_000 + i as u64))
        .collect();
    assert_eq!(client.get_active_pools_count(), num_pools);

    // Cancel every third pool.
    let reason = String::from_str(&env, "stress churn");
    let mut canceled: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut remaining: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for (i, &pool_id) in pool_ids.iter().enumerate() {
        if i % 3 == 0 {
            client.cancel_pool(&operator, &pool_id, &reason);
            canceled.insert(pool_id);
        } else {
            remaining.insert(pool_id);
        }
    }
    assert_eq!(client.get_active_pools_count(), remaining.len() as u32);

    // Paging through the index yields exactly the surviving pools, once each.
    let collect_active = || {
        let mut seen: std::vec::Vec<u64> = std::vec::Vec::new();
        let mut offset = 0u32;
        loop {
            let page = client.get_active_pools(&offset, &8u32);
            if page.is_empty() {
                break;
            }
            seen.extend(page.iter());
            offset += 8;
        }
        seen
    };
    let seen = collect_active();
    assert_eq!(
        seen.len(),
        remaining.len(),
        "no gaps or duplicates in the index"
    );
    let seen_set: std::collections::HashSet<u64> = seen.iter().copied().collect();
    assert_eq!(seen_set, remaining);
    assert!(seen_set.is_disjoint(&canceled));

    // Canceled pools are still readable and say so.
    for &pool_id in &canceled {
        assert_eq!(client.get_pool(&pool_id).state, MarketState::Canceled);
    }

    // A pool created after the churn joins the index with a fresh id.
    let newest = create_binary_pool(&env, &client, &creator, &token_address, 20_000);
    assert!(pool_ids.iter().all(|&id| newest > id));
    assert_eq!(client.get_active_pools_count(), remaining.len() as u32 + 1);
    assert!(collect_active().contains(&newest));
}

/// Reading one page of the active-pool index costs the same no matter how
/// many pools are active: enumeration is O(page size), not O(pool count).
/// Measured with the host's CPU instruction budget so the check is
/// deterministic rather than wall-clock based.
#[test]
fn test_1897_enumeration_cost_is_independent_of_total_pool_count() {
    let env = Env::default();
    let (client, _admin, _operator, token_address, _token_admin) = max_pools_setup(&env);
    let creator = Address::generate(&env);

    let page_cpu_cost = |offset: u32| {
        env.cost_estimate().budget().reset_default();
        let page = client.get_active_pools(&offset, &10u32);
        assert_eq!(page.len(), 10);
        env.cost_estimate().budget().cpu_instruction_cost()
    };

    for i in 0..10u64 {
        create_binary_pool(&env, &client, &creator, &token_address, 10_000 + i);
    }
    let small_set_cost = page_cpu_cost(0);

    for i in 10..40u64 {
        create_binary_pool(&env, &client, &creator, &token_address, 10_000 + i);
    }
    assert_eq!(client.get_active_pools_count(), 40);
    let first_page_cost = page_cpu_cost(0);
    let last_page_cost = page_cpu_cost(30);

    // Allow 50% slack for per-entry size differences; a scan over all pools
    // would cost roughly 4x more here.
    let limit = small_set_cost + small_set_cost / 2;
    assert!(
        first_page_cost <= limit && last_page_cost <= limit,
        "page cost grew with pool count: 10 pools = {}, 40 pools = {} (first page) / {} (last page)",
        small_set_cost,
        first_page_cost,
        last_page_cost
    );
}
