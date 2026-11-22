//! Tests for out-of-order price feed rejection (#2018) and typed errors instead
//! of panics in the oracle resolution paths (#2019).

#![cfg(test)]

use crate::test::{setup, ROLE_ADMIN};
use crate::{DataKey, PoolConfig, PredifiContractClient, PredifiError};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger as _},
    Address, Env, String, Symbol,
};

/// Role id checked by `oracle_resolve`.
const ROLE_ORACLE: u32 = 3;

/// Ledger time used by the tests below; feed timestamps must be strictly before it.
const NOW: u64 = 1_000;

type StoredFeed = (i128, i128, u64, u64);

fn read_feed(env: &Env, client: &PredifiContractClient<'_>, feed_pair: &Symbol) -> StoredFeed {
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get::<DataKey, StoredFeed>(&DataKey::PriceFeed(feed_pair.clone()))
            .unwrap()
    })
}

fn create_pool(
    env: &Env,
    client: &PredifiContractClient<'_>,
    token: &Address,
    creator: &Address,
) -> u64 {
    client.create_pool(
        creator,
        &5_000u64,
        token,
        &2u32,
        &symbol_short!("Finance"),
        &PoolConfig {
            start_time: NOW,
            description: String::from_str(env, "Oracle typed errors"),
            metadata_url: String::from_str(env, "ipfs://oracle-typed-errors"),
            min_stake: 1i128,
            max_stake: 0i128,
            max_total_stake: 0,
            min_total_stake: 1,
            initial_liquidity: 0i128,
            required_resolutions: 1u32,
            private: false,
            whitelist_key: None,
            outcome_descriptions: soroban_sdk::vec![
                env,
                String::from_str(env, "No"),
                String::from_str(env, "Yes"),
            ],
        },
    )
}

// ── #2018: reject out-of-order price feed updates ──────────────────────────────

/// Whitelists an oracle and stores the feed `(1_000, 1, 950, 10_000)` for `ETHUSD`.
fn arrange_stored_feed(env: &Env) -> (PredifiContractClient<'_>, Address, Symbol) {
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = NOW);

    let (ac_client, client, _token, _, _, _, _operator, _creator) = setup(env);
    let admin = Address::generate(env);
    let oracle = Address::generate(env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.add_oracle(&admin, &oracle);

    let feed_pair = Symbol::new(env, "ETHUSD");
    client.update_price_feed(&oracle, &feed_pair, &1_000i128, &1i128, &950u64, &10_000u64);

    (client, oracle, feed_pair)
}

#[test]
fn test_update_price_feed_accepts_newer_timestamp() {
    let env = Env::default();
    let (client, oracle, feed_pair) = arrange_stored_feed(&env);

    let result =
        client.try_update_price_feed(&oracle, &feed_pair, &1_010i128, &2i128, &960u64, &10_000u64);

    assert!(result.is_ok());
    assert_eq!(read_feed(&env, &client, &feed_pair), (1_010, 2, 960, 10_000));
}

#[test]
fn test_update_price_feed_rejects_equal_timestamp() {
    let env = Env::default();
    let (client, oracle, feed_pair) = arrange_stored_feed(&env);

    let result =
        client.try_update_price_feed(&oracle, &feed_pair, &1_010i128, &2i128, &950u64, &10_000u64);

    assert_eq!(result, Err(Ok(PredifiError::InvalidData)));
    assert_eq!(read_feed(&env, &client, &feed_pair), (1_000, 1, 950, 10_000));
}

#[test]
fn test_update_price_feed_rejects_older_timestamp_and_keeps_stored_price() {
    let env = Env::default();
    let (client, oracle, feed_pair) = arrange_stored_feed(&env);

    // A delayed or replayed submission carrying an older timestamp.
    let result =
        client.try_update_price_feed(&oracle, &feed_pair, &1_010i128, &2i128, &940u64, &10_000u64);

    assert_eq!(result, Err(Ok(PredifiError::InvalidData)));
    // The newer stored price is unchanged afterwards.
    assert_eq!(read_feed(&env, &client, &feed_pair), (1_000, 1, 950, 10_000));
}

#[test]
fn test_update_price_feed_first_update_for_a_pair_has_nothing_to_order_against() {
    let env = Env::default();
    let (client, oracle, _eth) = arrange_stored_feed(&env);
    let btc = Symbol::new(&env, "BTCUSD");

    // Even a very old timestamp is fine when no entry exists for this pair yet.
    let result =
        client.try_update_price_feed(&oracle, &btc, &60_000i128, &1i128, &1u64, &10_000u64);

    assert!(result.is_ok());
    assert_eq!(read_feed(&env, &client, &btc), (60_000, 1, 1, 10_000));
}

#[test]
fn test_update_price_feed_ordering_is_tracked_per_feed_pair() {
    let env = Env::default();
    let (client, oracle, _eth) = arrange_stored_feed(&env);
    let btc = Symbol::new(&env, "BTCUSD");

    client.update_price_feed(&oracle, &btc, &60_000i128, &1i128, &990u64, &10_000u64);

    // ETHUSD's stored timestamp (950) does not constrain BTCUSD: 980 is older than
    // BTCUSD's own 990 so it is rejected, while 991 is newer so it is accepted.
    let older =
        client.try_update_price_feed(&oracle, &btc, &60_100i128, &1i128, &980u64, &10_000u64);
    let newer =
        client.try_update_price_feed(&oracle, &btc, &60_100i128, &1i128, &991u64, &10_000u64);

    assert_eq!(older, Err(Ok(PredifiError::InvalidData)));
    assert!(newer.is_ok());
}

#[test]
fn test_update_price_feed_deviation_check_still_applies_to_newer_updates() {
    let env = Env::default();
    let (client, oracle, feed_pair) = arrange_stored_feed(&env);

    // Newer timestamp but more than 5x the previous price: still rejected.
    let too_high =
        client.try_update_price_feed(&oracle, &feed_pair, &5_001i128, &1i128, &960u64, &10_000u64);
    // Newer timestamp but below a fifth of the previous price: still rejected.
    let too_low =
        client.try_update_price_feed(&oracle, &feed_pair, &199i128, &1i128, &960u64, &10_000u64);

    assert_eq!(too_high, Err(Ok(PredifiError::InvalidData)));
    assert_eq!(too_low, Err(Ok(PredifiError::InvalidData)));
    assert_eq!(read_feed(&env, &client, &feed_pair), (1_000, 1, 950, 10_000));

    // Exactly 5x the previous price is within bounds and accepted.
    let at_bound =
        client.try_update_price_feed(&oracle, &feed_pair, &5_000i128, &1i128, &960u64, &10_000u64);
    assert!(at_bound.is_ok());
}

// ── #2019: typed errors instead of panics ──────────────────────────────────────

#[test]
fn test_oracle_resolve_unknown_pool_returns_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = NOW);

    let (ac_client, client, _token, _, _, _, _operator, _creator) = setup(&env);
    let oracle = Address::generate(&env);
    ac_client.grant_role(&oracle, &ROLE_ORACLE);

    let result =
        client.try_oracle_resolve(&oracle, &9_999u64, &1u32, &String::from_str(&env, "proof"));

    assert_eq!(result, Err(Ok(PredifiError::PoolNotFound)));
}

#[test]
fn test_resolve_pool_from_price_unknown_feed_pair_returns_price_feed_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = NOW);

    let (_ac_client, client, token, _, _, _, operator, creator) = setup(&env);
    let pool_id = create_pool(&env, &client, &token, &creator);

    // The condition points at a feed pair that no oracle has ever updated.
    let unknown_feed = Symbol::new(&env, "NOFEED");
    client.set_price_condition(&operator, &pool_id, &unknown_feed, &1_000i128, &1u32, &100u32);

    env.ledger().with_mut(|li| li.timestamp = 8_600);
    let result = client.try_resolve_pool_from_price(&pool_id);

    assert_eq!(result, Err(Ok(PredifiError::PriceFeedNotFound)));
}

#[test]
fn test_resolve_pool_from_price_unknown_pool_returns_price_condition_not_set() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = NOW);

    let (_ac_client, client, _token, _, _, _, _operator, _creator) = setup(&env);

    let result = client.try_resolve_pool_from_price(&9_999u64);

    assert_eq!(result, Err(Ok(PredifiError::PriceConditionNotSet)));
}

#[test]
fn test_resolve_pool_from_price_missing_pool_record_returns_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = NOW);

    let (ac_client, client, token, _, _, _, operator, creator) = setup(&env);
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    ac_client.grant_role(&admin, &ROLE_ADMIN);
    client.add_oracle(&admin, &oracle);
    client.init_oracle(&admin, &Address::generate(&env), &10_000u64, &100u32);

    let pool_id = create_pool(&env, &client, &token, &creator);
    let feed_pair = Symbol::new(&env, "ETHUSD");
    client.set_price_condition(&operator, &pool_id, &feed_pair, &1_000i128, &1u32, &100u32);
    client.update_price_feed(&oracle, &feed_pair, &1_005i128, &1i128, &999u64, &10_000u64);

    // Everything the resolver checks first (condition, feed, freshness) is valid;
    // only the pool record is gone.
    env.as_contract(&client.address, || {
        env.storage().persistent().remove(&DataKey::Pool(pool_id));
    });
    env.ledger().with_mut(|li| li.timestamp = 8_600);

    let result = client.try_resolve_pool_from_price(&pool_id);

    assert_eq!(result, Err(Ok(PredifiError::PoolNotFound)));
}
