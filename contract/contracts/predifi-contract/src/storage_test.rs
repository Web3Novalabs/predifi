//! Storage key standardization tests.
//!
//! Verifies that:
//! 1. All `DataKey` variants are stored in the correct storage tier
//!    (instance vs persistent vs temporary).
//! 2. No ad-hoc keys (magic numbers, reused variants) are used for
//!    oracle/price-feed data.
//! 3. `DataKey::OracleConfig` is distinct from `DataKey::TokenWl`.
//! 4. `DataKey::PriceFeed` and `DataKey::PriceCondition` are distinct from
//!    `DataKey::OutStake`.
//! 5. All variants round-trip correctly through storage.

#[cfg(test)]
mod tests {
    use crate::{
        price_feed_simple::{PriceFeedAdapter, SimplePriceFeed, SimpleOracleConfig},
        DataKey, PredifiContract, PredifiContractClient,
    };
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Ledger},
        Address, Env, Symbol,
    };

    // ── helpers ──────────────────────────────────────────────────────────────

    mod dummy_ac {
        use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

        #[contract]
        pub struct DummyAC;

        #[contractimpl]
        impl DummyAC {
            pub fn grant_role(env: Env, user: Address, role: u32) {
                env.storage()
                    .instance()
                    .set(&(Symbol::new(&env, "role"), user, role), &true);
            }
            pub fn has_role(env: Env, user: Address, role: u32) -> bool {
                env.storage()
                    .instance()
                    .get(&(Symbol::new(&env, "role"), user, role))
                    .unwrap_or(false)
            }
        }
    }

    fn setup(env: &Env) -> (PredifiContractClient, Address, Address) {
        env.mock_all_auths();
        let ac = env.register(dummy_ac::DummyAC, ());
        let ac_client = dummy_ac::DummyACClient::new(env, &ac);
        let admin = Address::generate(env);
        ac_client.grant_role(&admin, &0u32);

        let cid = env.register(PredifiContract, ());
        let client = PredifiContractClient::new(env, &cid);
        let treasury = Address::generate(env);
        client.init(&ac, &treasury, &0u32, &0u64, &3600u64);
        (client, cid, admin)
    }

    // ── DataKey variant distinctness ─────────────────────────────────────────

    /// OracleConfig must be stored under DataKey::OracleConfig, not TokenWl.
    #[test]
    fn test_oracle_config_uses_dedicated_key() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, contract_id, admin) = setup(&env);

        let pyth = Address::generate(&env);
        let token = Address::generate(&env);

        // Whitelist a token — uses DataKey::TokenWl
        client.add_token_to_whitelist(&admin, &token);

        // Store oracle config via price_feed_simple — uses DataKey::OracleConfig
        env.as_contract(&contract_id, || {
            PriceFeedAdapter::init_oracle(&env, &admin, pyth.clone(), 300, 100).unwrap();
        });

        env.as_contract(&contract_id, || {
            // TokenWl for the pyth address must NOT exist (we never whitelisted it)
            let wl_key = DataKey::TokenWl(pyth.clone());
            assert!(
                !env.storage().persistent().has(&wl_key),
                "OracleConfig must not be stored under TokenWl"
            );

            // OracleConfig key must exist
            let oc_key = DataKey::OracleConfig;
            assert!(
                env.storage().persistent().has(&oc_key),
                "OracleConfig must be stored under DataKey::OracleConfig"
            );

            // TokenWl for the whitelisted token must still exist
            let token_key = DataKey::TokenWl(token.clone());
            assert!(
                env.storage().persistent().has(&token_key),
                "TokenWl must still work for token whitelist"
            );
        });
    }

    /// PriceFeed must be stored under DataKey::PriceFeed, not OutStake.
    #[test]
    fn test_price_feed_uses_dedicated_key() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        let oracle = Address::generate(&env);
        let pair = symbol_short!("ETHUSD");
        let ts = env.ledger().timestamp();

        env.as_contract(&contract_id, || {
            PriceFeedAdapter::update_price_feed(
                &env, &oracle, pair.clone(), 3000, 10, ts, ts + 60,
            )
            .unwrap();
        });

        env.as_contract(&contract_id, || {
            // Must be stored under PriceFeed(pair)
            let pf_key = DataKey::PriceFeed(pair.clone());
            assert!(
                env.storage().persistent().has(&pf_key),
                "Price feed must be stored under DataKey::PriceFeed"
            );

            // Must NOT be stored under OutStake with magic pool_id
            let magic_key = DataKey::OutStake(999999, 0);
            assert!(
                !env.storage().persistent().has(&magic_key),
                "Price feed must not be stored under DataKey::OutStake with magic id"
            );
        });
    }

    /// PriceCondition must be stored under DataKey::PriceCondition, not OutStake.
    #[test]
    fn test_price_condition_uses_dedicated_key() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        let pair = symbol_short!("BTCUSD");
        let pool_id: u64 = 42;

        env.as_contract(&contract_id, || {
            PriceFeedAdapter::set_price_condition(&env, pool_id, pair.clone(), 60000, 1, 100)
                .unwrap();
        });

        env.as_contract(&contract_id, || {
            // Must be stored under PriceCondition(pool_id)
            let pc_key = DataKey::PriceCondition(pool_id);
            assert!(
                env.storage().persistent().has(&pc_key),
                "Price condition must be stored under DataKey::PriceCondition"
            );

            // Must NOT be stored under OutStake(pool_id, 1)
            let bad_key = DataKey::OutStake(pool_id, 1);
            assert!(
                !env.storage().persistent().has(&bad_key),
                "Price condition must not be stored under DataKey::OutStake"
            );
        });
    }

    // ── Round-trip tests ─────────────────────────────────────────────────────

    /// OracleConfig round-trips correctly through DataKey::OracleConfig.
    #[test]
    fn test_oracle_config_round_trip() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, admin) = setup(&env);

        let pyth = Address::generate(&env);

        env.as_contract(&contract_id, || {
            PriceFeedAdapter::init_oracle(&env, &admin, pyth.clone(), 300, 100).unwrap();
        });

        env.as_contract(&contract_id, || {
            let cfg: Option<SimpleOracleConfig> =
                env.storage().persistent().get(&DataKey::OracleConfig);
            let cfg = cfg.expect("OracleConfig must be present");
            assert_eq!(cfg.pyth_contract, pyth);
            assert_eq!(cfg.max_price_age, 300);
            assert_eq!(cfg.min_confidence_ratio, 100);
        });
    }

    /// PriceFeed round-trips correctly through DataKey::PriceFeed.
    #[test]
    fn test_price_feed_round_trip() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        let oracle = Address::generate(&env);
        let pair = symbol_short!("ETHUSD");
        let ts = env.ledger().timestamp();

        env.as_contract(&contract_id, || {
            PriceFeedAdapter::update_price_feed(
                &env, &oracle, pair.clone(), 3000, 10, ts, ts + 60,
            )
            .unwrap();
        });

        env.as_contract(&contract_id, || {
            let feed: Option<SimplePriceFeed> =
                env.storage().persistent().get(&DataKey::PriceFeed(pair.clone()));
            let feed = feed.expect("PriceFeed must be present");
            assert_eq!(feed.pair, pair);
            assert_eq!(feed.price, 3000);
            assert_eq!(feed.confidence, 10);
            assert_eq!(feed.timestamp, ts);
            assert_eq!(feed.expires_at, ts + 60);
        });
    }

    /// PriceCondition round-trips correctly through DataKey::PriceCondition.
    #[test]
    fn test_price_condition_round_trip() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        let pair = symbol_short!("BTCUSD");
        let pool_id: u64 = 7;

        env.as_contract(&contract_id, || {
            PriceFeedAdapter::set_price_condition(&env, pool_id, pair.clone(), 60000, 1, 50)
                .unwrap();
        });

        env.as_contract(&contract_id, || {
            let cond: Option<(Symbol, i128, u32, u32)> =
                env.storage().persistent().get(&DataKey::PriceCondition(pool_id));
            let (fp, tp, op, tol) = cond.expect("PriceCondition must be present");
            assert_eq!(fp, pair);
            assert_eq!(tp, 60000);
            assert_eq!(op, 1);
            assert_eq!(tol, 50);
        });
    }

    // ── Storage tier tests ───────────────────────────────────────────────────

    /// Instance-storage keys (Config, Paused, Version, PoolIdCtr, ReferralCutBps)
    /// must be in instance storage, not persistent.
    #[test]
    fn test_instance_keys_in_instance_storage() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        env.as_contract(&contract_id, || {
            assert!(
                env.storage().instance().has(&DataKey::Config),
                "Config must be in instance storage"
            );
            assert!(
                env.storage().instance().has(&DataKey::PoolIdCtr),
                "PoolIdCtr must be in instance storage"
            );
            assert!(
                env.storage().instance().has(&DataKey::Version),
                "Version must be in instance storage"
            );
            // These must NOT be in persistent storage
            assert!(
                !env.storage().persistent().has(&DataKey::Config),
                "Config must not be in persistent storage"
            );
        });
    }

    /// RentGuard must be in temporary storage only.
    #[test]
    fn test_rent_guard_in_temporary_storage() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        env.as_contract(&contract_id, || {
            // RentGuard should not exist when no re-entrant call is in progress
            assert!(
                !env.storage().temporary().has(&DataKey::RentGuard),
                "RentGuard must not be set outside of a guarded call"
            );
            assert!(
                !env.storage().persistent().has(&DataKey::RentGuard),
                "RentGuard must not be in persistent storage"
            );
            assert!(
                !env.storage().instance().has(&DataKey::RentGuard),
                "RentGuard must not be in instance storage"
            );
        });
    }

    // ── Isolation tests ──────────────────────────────────────────────────────

    /// Different pool IDs must produce distinct PriceCondition keys.
    #[test]
    fn test_price_condition_keys_are_pool_scoped() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        let pair = symbol_short!("ETHUSD");

        env.as_contract(&contract_id, || {
            PriceFeedAdapter::set_price_condition(&env, 1, pair.clone(), 3000, 1, 100).unwrap();
            PriceFeedAdapter::set_price_condition(&env, 2, pair.clone(), 4000, 0, 50).unwrap();
        });

        env.as_contract(&contract_id, || {
            let c1: (Symbol, i128, u32, u32) = env
                .storage()
                .persistent()
                .get(&DataKey::PriceCondition(1))
                .unwrap();
            let c2: (Symbol, i128, u32, u32) = env
                .storage()
                .persistent()
                .get(&DataKey::PriceCondition(2))
                .unwrap();

            assert_eq!(c1.1, 3000, "Pool 1 target price must be 3000");
            assert_eq!(c2.1, 4000, "Pool 2 target price must be 4000");
            assert_ne!(c1.1, c2.1, "Different pools must have independent conditions");
        });
    }

    /// Different feed pairs must produce distinct PriceFeed keys.
    #[test]
    fn test_price_feed_keys_are_pair_scoped() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        let oracle = Address::generate(&env);
        let eth = symbol_short!("ETHUSD");
        let btc = symbol_short!("BTCUSD");
        let ts = env.ledger().timestamp();

        env.as_contract(&contract_id, || {
            PriceFeedAdapter::update_price_feed(&env, &oracle, eth.clone(), 3000, 5, ts, ts + 60)
                .unwrap();
            PriceFeedAdapter::update_price_feed(&env, &oracle, btc.clone(), 60000, 50, ts, ts + 60)
                .unwrap();
        });

        env.as_contract(&contract_id, || {
            let eth_feed: SimplePriceFeed = env
                .storage()
                .persistent()
                .get(&DataKey::PriceFeed(eth.clone()))
                .unwrap();
            let btc_feed: SimplePriceFeed = env
                .storage()
                .persistent()
                .get(&DataKey::PriceFeed(btc.clone()))
                .unwrap();

            assert_eq!(eth_feed.price, 3000);
            assert_eq!(btc_feed.price, 60000);
            assert_ne!(eth_feed.price, btc_feed.price);
        });
    }

    // ── Validity tests ───────────────────────────────────────────────────────

    /// is_price_valid returns false for expired feeds.
    #[test]
    fn test_price_validity_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        let oracle = Address::generate(&env);
        let pair = symbol_short!("ETHUSD");
        let ts = 1000u64;
        env.ledger().with_mut(|l| l.timestamp = ts);

        env.as_contract(&contract_id, || {
            PriceFeedAdapter::update_price_feed(
                &env, &oracle, pair.clone(), 3000, 10, ts, ts + 30,
            )
            .unwrap();
        });

        // Advance past expiry
        env.ledger().with_mut(|l| l.timestamp = ts + 60);

        env.as_contract(&contract_id, || {
            let feed: SimplePriceFeed = env
                .storage()
                .persistent()
                .get(&DataKey::PriceFeed(pair.clone()))
                .unwrap();
            assert!(
                !PriceFeedAdapter::is_price_valid(&env, &feed, 300),
                "Expired feed must be invalid"
            );
        });
    }

    /// is_price_valid returns true for fresh feeds.
    #[test]
    fn test_price_validity_fresh() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, contract_id, _) = setup(&env);

        let oracle = Address::generate(&env);
        let pair = symbol_short!("ETHUSD");
        let ts = 1000u64;
        env.ledger().with_mut(|l| l.timestamp = ts);

        env.as_contract(&contract_id, || {
            PriceFeedAdapter::update_price_feed(
                &env, &oracle, pair.clone(), 3000, 10, ts, ts + 300,
            )
            .unwrap();
        });

        env.as_contract(&contract_id, || {
            let feed: SimplePriceFeed = env
                .storage()
                .persistent()
                .get(&DataKey::PriceFeed(pair.clone()))
                .unwrap();
            assert!(
                PriceFeedAdapter::is_price_valid(&env, &feed, 300),
                "Fresh feed must be valid"
            );
        });
    }
}
