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
