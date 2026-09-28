//! In-memory TTL cache for frequently-accessed pool detail data (#1369).
//!
//! Mirrors the [`crate::price_cache::PriceCache`] pattern: a short-lived,
//! best-effort cache that avoids re-running the two-query pool-detail lookup
//! (`get_pool_by_id` + `get_pool_outcome_stakes`) for pools that are polled
//! repeatedly by the frontend (e.g. an active pool's detail page).
//!
//! ## What is cached
//!
//! One [`crate::db::PoolWithOdds`] value per pool ID — the combined result of
//! a pool's details and its computed outcome odds, as returned by the pool
//! detail endpoint.
//!
//! ## Lifetime
//!
//! Each entry lives for `POOL_CACHE_TTL` (10 seconds) from the moment it is
//! written via [`PoolCache::set`]. [`PoolCache::get`] treats an entry older
//! than the TTL as a miss rather than returning stale data, so a pool that is
//! not otherwise invalidated will naturally refresh from the database at
//! least once every 10 seconds.
//!
//! ## Invalidation
//!
//! Entries are removed early, before the TTL elapses, whenever a request
//! handler mutates a pool in a way that changes its detail response — for
//! example after paying out a creator incentive or updating a pool's tags —
//! by calling [`PoolCache::invalidate`] with that pool's ID. There is no
//! background sweeper: expired entries are simply skipped on the next `get`
//! and overwritten on the next `set`.
//!
//! ## Negative caching
//!
//! A lookup for a pool ID that does not exist in the database is also cached
//! (via [`PoolCache::set_missing`]), under its own, shorter
//! [`PoolCache::negative_ttl`]. This protects the database from unbounded
//! load when a client repeatedly polls a wrong or deleted pool ID: instead of
//! falling through to Postgres on every request, the handler serves 404
//! straight from the cache until the negative entry expires. Once it does,
//! the next lookup queries the database again, so a pool created after a
//! negative entry was cached becomes visible as soon as that entry expires.
//!
//! ## Relationship to the Redis cache
//!
//! This is a separate, complementary layer from [`crate::redis_cache`]:
//!
//! - **Storage**: this cache lives in process memory (a `HashMap` behind an
//!   `RwLock`); [`crate::redis_cache::RedisCache`] stores serialized JSON in
//!   an external Redis instance.
//! - **Scope**: this cache is per-process and lost on restart or when running
//!   multiple backend instances (no shared invalidation across instances);
//!   the Redis cache is shared across every backend instance and process
//!   restart.
//! - **What's cached**: this module caches only single pool-detail lookups;
//!   `redis_cache` also covers pool list queries, protocol stats, and user
//!   prediction lists, each with their own TTL.
//! - **Failure mode**: this cache cannot fail independently of the process it
//!   runs in; `redis_cache` is built to fail open (silently skip caching) if
//!   the Redis connection is unavailable.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crate::db::PoolWithOdds;

/// How long a cached pool-detail entry remains valid before being treated as
/// stale and re-fetched from the database.
const POOL_CACHE_TTL: Duration = Duration::from_secs(10);

/// Default TTL for negative (pool-not-found) cache entries when a
/// [`PoolCache`] is created via [`PoolCache::new`]. Deliberately shorter than
/// [`POOL_CACHE_TTL`] so a pool ID that starts existing (or that was queried
/// with a typo that gets corrected) becomes visible again quickly, while
/// still absorbing a burst of repeated lookups for the same bad ID.
const DEFAULT_POOL_CACHE_NEGATIVE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
enum CacheEntry {
    /// A successfully fetched pool, valid for [`POOL_CACHE_TTL`].
    Found { value: PoolWithOdds, cached_at: Instant },
    /// A confirmed "no such pool" result, valid for the cache's configured
    /// negative TTL (see [`PoolCache::negative_ttl`]).
    NotFound { cached_at: Instant },
}

/// Outcome of a [`PoolCache::get`] lookup.
#[derive(Debug, Clone)]
pub enum CacheLookup {
    /// A cached, unexpired pool detail value.
    Found(PoolWithOdds),
    /// The pool ID is cached as confirmed nonexistent (negative cache hit);
    /// callers should respond 404 without touching the database.
    NotFound,
    /// No usable entry (never cached, or the entry expired); callers must
    /// query the database and then call [`PoolCache::set`] or
    /// [`PoolCache::set_missing`] with the result.
    Miss,
}

struct Inner {
    entries: RwLock<HashMap<i64, CacheEntry>>,
    /// TTL applied to negative (not-found) entries. Configurable via
    /// [`PoolCache::with_negative_ttl`]; defaults to
    /// [`DEFAULT_POOL_CACHE_NEGATIVE_TTL`].
    negative_ttl: Duration,
}

/// Shared, thread-safe cache of recently-fetched pool details (and confirmed
/// pool-not-found results), keyed by pool ID.
#[derive(Clone)]
pub struct PoolCache(Arc<Inner>);

impl Default for PoolCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolCache {
    /// Create a cache using the default negative-cache TTL
    /// ([`DEFAULT_POOL_CACHE_NEGATIVE_TTL`]).
    pub fn new() -> Self {
        Self::with_negative_ttl(DEFAULT_POOL_CACHE_NEGATIVE_TTL)
    }

    /// Create a cache with an explicit negative-cache TTL, e.g. sourced from
    /// `PREDIFI_POOL_NEGATIVE_CACHE_TTL_SECS` (see
    /// [`crate::config::Config::pool_negative_cache_ttl_secs`]).
    pub fn with_negative_ttl(negative_ttl: Duration) -> Self {
        Self(Arc::new(Inner {
            entries: RwLock::new(HashMap::new()),
            negative_ttl,
        }))
    }

    /// The TTL applied to negative (not-found) entries in this cache.
    pub fn negative_ttl(&self) -> Duration {
        self.0.negative_ttl
    }

    /// Look up `pool_id`, distinguishing a positive hit, a negative
    /// (confirmed not-found) hit, and a plain cache miss.
    pub fn get(&self, pool_id: i64) -> CacheLookup {
        let Ok(guard) = self.0.entries.read() else {
            return CacheLookup::Miss;
        };
        match guard.get(&pool_id) {
            Some(CacheEntry::Found { value, cached_at }) if cached_at.elapsed() < POOL_CACHE_TTL => {
                CacheLookup::Found(value.clone())
            }
            Some(CacheEntry::NotFound { cached_at }) if cached_at.elapsed() < self.0.negative_ttl => {
                CacheLookup::NotFound
            }
            _ => CacheLookup::Miss,
        }
    }

    /// Store a freshly-fetched value for `pool_id`.
    pub fn set(&self, pool_id: i64, value: PoolWithOdds) {
        if let Ok(mut guard) = self.0.entries.write() {
            guard.insert(
                pool_id,
                CacheEntry::Found {
                    value,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// Record that `pool_id` does not exist in the database, so repeated
    /// lookups for it are served from cache (as a 404) for
    /// [`PoolCache::negative_ttl`] instead of hitting Postgres every time.
    pub fn set_missing(&self, pool_id: i64) {
        if let Ok(mut guard) = self.0.entries.write() {
            guard.insert(
                pool_id,
                CacheEntry::NotFound {
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// Invalidate a cached entry (e.g. once a new prediction changes its odds,
    /// or a pool is created after its ID was negatively cached).
    pub fn invalidate(&self, pool_id: i64) {
        if let Ok(mut guard) = self.0.entries.write() {
            guard.remove(&pool_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{OutcomeOdds, PoolWithOdds};
    use chrono::Utc;

    fn sample_pool(pool_id: i64, name: &str) -> PoolWithOdds {
        PoolWithOdds {
            pool_id,
            name: name.to_string(),
            category: "Sports".to_string(),
            total_stake: 1000,
            end_time: Utc::now(),
            created_at: Utc::now(),
            state: "active".to_string(),
            creator: "GABC".to_string(),
            token: "XLM".to_string(),
            result: None,
            odds: vec![OutcomeOdds {
                outcome: 1,
                stake: 500,
                odds: 1.5,
            }],
        }
    }

    #[test]
    fn fresh_cache_is_empty() {
        let cache = PoolCache::new();
        assert!(matches!(cache.get(1), CacheLookup::Miss));
    }

    #[test]
    fn insert_then_get_returns_value() {
        let cache = PoolCache::new();
        let pool = sample_pool(42, "Test Pool");
        cache.set(42, pool.clone());
        match cache.get(42) {
            CacheLookup::Found(retrieved) => {
                assert_eq!(retrieved.pool_id, 42);
                assert_eq!(retrieved.name, "Test Pool");
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn get_missing_key_returns_none() {
        let cache = PoolCache::new();
        cache.set(1, sample_pool(1, "Pool One"));
        assert!(matches!(cache.get(2), CacheLookup::Miss));
    }

    #[test]
    fn overwrite_replaces_value() {
        let cache = PoolCache::new();
        cache.set(1, sample_pool(1, "Original"));
        cache.set(1, sample_pool(1, "Replaced"));
        match cache.get(1) {
            CacheLookup::Found(retrieved) => assert_eq!(retrieved.name, "Replaced"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn set_missing_then_get_returns_not_found() {
        let cache = PoolCache::new();
        cache.set_missing(99);
        assert!(matches!(cache.get(99), CacheLookup::NotFound));
    }

    #[test]
    fn negative_ttl_expires_independently_of_positive_ttl() {
        // A near-zero negative TTL means the entry is immediately stale.
        let cache = PoolCache::with_negative_ttl(Duration::from_millis(0));
        cache.set_missing(7);
        std::thread::sleep(Duration::from_millis(5));
        assert!(matches!(cache.get(7), CacheLookup::Miss));
    }

    #[test]
    fn invalidate_clears_a_negative_entry() {
        let cache = PoolCache::new();
        cache.set_missing(5);
        assert!(matches!(cache.get(5), CacheLookup::NotFound));
        cache.invalidate(5);
        assert!(matches!(cache.get(5), CacheLookup::Miss));
    }

    #[test]
    fn default_negative_ttl_is_lower_than_positive_ttl() {
        let cache = PoolCache::new();
        assert!(cache.negative_ttl() < POOL_CACHE_TTL);
    }
}
