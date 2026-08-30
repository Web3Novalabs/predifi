//! In-memory TTL cache for frequently-accessed pool detail data (#1369).
//!
//! Mirrors the [`crate::price_cache::PriceCache`] pattern: a short-lived,
//! best-effort cache that avoids re-running the two-query pool-detail lookup
//! (`get_pool_by_id` + `get_pool_outcome_stakes`) for pools that are polled
//! repeatedly by the frontend (e.g. an active pool's detail page).

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
        assert!(cache.get(1).is_none());
    }

    #[test]
    fn insert_then_get_returns_value() {
        let cache = PoolCache::new();
        let pool = sample_pool(42, "Test Pool");
        cache.set(42, pool.clone());
        let retrieved = cache.get(42).expect("cached value should be present");
        assert_eq!(retrieved.pool_id, 42);
        assert_eq!(retrieved.name, "Test Pool");
    }

    #[test]
    fn get_missing_key_returns_none() {
        let cache = PoolCache::new();
        cache.set(1, sample_pool(1, "Pool One"));
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn overwrite_replaces_value() {
        let cache = PoolCache::new();
        cache.set(1, sample_pool(1, "Original"));
        cache.set(1, sample_pool(1, "Replaced"));
        let retrieved = cache.get(1).expect("cached value should be present");
        assert_eq!(retrieved.name, "Replaced");
    }
}
