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

#[derive(Clone)]
struct CacheEntry {
    value: PoolWithOdds,
    cached_at: Instant,
}

/// Shared, thread-safe cache of recently-fetched pool details, keyed by pool ID.
#[derive(Clone, Default)]
pub struct PoolCache(Arc<RwLock<HashMap<i64, CacheEntry>>>);

impl PoolCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a cached value for `pool_id` if present and not yet expired.
    pub fn get(&self, pool_id: i64) -> Option<PoolWithOdds> {
        let guard = self.0.read().ok()?;
        let entry = guard.get(&pool_id)?;
        if entry.cached_at.elapsed() < POOL_CACHE_TTL {
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// Store a freshly-fetched value for `pool_id`.
    pub fn set(&self, pool_id: i64, value: PoolWithOdds) {
        if let Ok(mut guard) = self.0.write() {
            guard.insert(
                pool_id,
                CacheEntry {
                    value,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// Invalidate a cached entry (e.g. once a new prediction changes its odds).
    pub fn invalidate(&self, pool_id: i64) {
        if let Ok(mut guard) = self.0.write() {
            guard.remove(&pool_id);
        }
    }
}
