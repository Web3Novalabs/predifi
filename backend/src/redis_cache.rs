//! Redis caching layer for hot data
//!
//! Implements issue #714: Redis Caching for Hot Data
//!
//! This module provides a caching layer for frequently accessed API responses
//! to reduce database load. Uses Redis with configurable TTL values.
//!
//! Features:
//! - Automatic cache invalidation after TTL
//! - Graceful fallback when Redis is unavailable
//! - JSON serialization for complex types
//! - Connection pooling via redis ConnectionManager

use redis::{aio::ConnectionManager, AsyncCommands, ErrorKind};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, error, warn};

/// Default TTL for cached pool data (60 seconds)
pub const POOLS_CACHE_TTL: u64 = 60;

/// Default TTL for cached pool details (30 seconds)
pub const POOL_DETAILS_CACHE_TTL: u64 = 30;

/// Default TTL for user predictions (45 seconds)
pub const USER_PREDICTIONS_CACHE_TTL: u64 = 45;

/// Default TTL for cached protocol stats (30 seconds)
pub const STATS_CACHE_TTL: u64 = 30;

/// Redis key pattern matching all cached pool list queries.
pub const POOLS_CACHE_PATTERN: &str = "pools:*";

/// Redis key pattern matching all cached stats queries.
pub const STATS_CACHE_PATTERN: &str = "stats:*";

/// Thread-safe Redis cache client with graceful fail-open behaviour.
///
/// All operations silently no-op when Redis is unavailable so the application
/// continues to function without caching rather than returning errors to users.
/// Use [`RedisCache::disabled`] to create an always-no-op instance for tests.
#[derive(Clone)]
pub struct RedisCache {
    manager: Option<ConnectionManager>,
    /// When set, health probes treat Redis as available without a live connection.
    simulate_available: bool,
}

impl RedisCache {
    /// Create a new Redis cache client
    ///
    /// If Redis connection fails, returns a cache instance that will
    /// gracefully skip caching operations (fail-open behavior).
    pub async fn new(redis_url: &str) -> Self {
        match redis::Client::open(redis_url) {
            Ok(client) => match ConnectionManager::new(client).await {
                Ok(manager) => {
                    debug!("Redis cache initialized successfully");
                    Self {
                        manager: Some(manager),
                        simulate_available: false,
                    }
                }
                Err(err) => {
                    warn!("Failed to create Redis connection manager: {}", err);
                    Self {
                        manager: None,
                        simulate_available: false,
                    }
                }
            },
            Err(err) => {
                warn!("Failed to create Redis client: {}", err);
                Self {
                    manager: None,
                    simulate_available: false,
                }
            }
        }
    }

    /// Create a disabled cache instance (for testing or when Redis is not configured)
    pub fn disabled() -> Self {
        Self {
            manager: None,
            simulate_available: false,
        }
    }

    /// Create a no-op cache that satisfies health/readiness probes in unit tests.
    #[cfg(test)]
    pub fn simulate_available() -> Self {
        Self {
            manager: None,
            simulate_available: true,
        }
    }

    /// Check if Redis is available
    pub fn is_available(&self) -> bool {
        self.manager.is_some() || self.simulate_available
    }

    /// Get a value from cache
    ///
    /// Returns None if:
    /// - Redis is unavailable
    /// - Key doesn't exist
    /// - Deserialization fails
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let manager = self.manager.as_ref()?;
        let mut conn = manager.clone();

        match conn.get::<_, Option<String>>(key).await {
            Ok(Some(data)) => match serde_json::from_str::<T>(&data) {
                Ok(value) => {
                    debug!("Cache hit: {}", key);
                    Some(value)
                }
                Err(err) => {
                    error!("Failed to deserialize cached value for {}: {}", key, err);
                    None
                }
            },
            Ok(None) => {
                debug!("Cache miss: {}", key);
                None
            }
            Err(err) => {
                if is_connection_error(err.kind()) {
                    warn!("Redis connection dropout on GET {}: {}", key, err);
                } else {
                    error!("Redis GET error for {}: {}", key, err);
                }
                None
            }
        }
    }

    /// Set a value in cache with TTL
    ///
    /// Silently fails if Redis is unavailable (fail-open behavior)
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) {
        let manager = match self.manager.as_ref() {
            Some(m) => m,
            None => return,
        };

        let mut conn = manager.clone();

        let data = match serde_json::to_string(value) {
            Ok(d) => d,
            Err(err) => {
                error!("Failed to serialize value for {}: {}", key, err);
                return;
            }
        };

        if let Err(err) = conn.set_ex::<_, _, ()>(key, data, ttl_secs).await {
            if is_connection_error(err.kind()) {
                warn!("Redis connection dropout on SET {}: {}", key, err);
            } else {
                error!("Redis SET error for {}: {}", key, err);
            }
        } else {
            debug!("Cached: {} (TTL: {}s)", key, ttl_secs);
        }
    }

    /// Delete a key from cache
    ///
    /// Used for cache invalidation when data changes
    pub async fn delete(&self, key: &str) {
        let manager = match self.manager.as_ref() {
            Some(m) => m,
            None => return,
        };

        let mut conn = manager.clone();

        if let Err(err) = conn.del::<_, ()>(key).await {
            if is_connection_error(err.kind()) {
                warn!("Redis connection dropout on DEL {}: {}", key, err);
            } else {
                error!("Redis DEL error for {}: {}", key, err);
            }
        } else {
            debug!("Invalidated cache: {}", key);
        }
    }

    /// Delete multiple keys matching a pattern
    ///
    /// Useful for invalidating related cache entries
    pub async fn delete_pattern(&self, pattern: &str) {
        let manager = match self.manager.as_ref() {
            Some(m) => m,
            None => return,
        };

        let mut conn = manager.clone();

        // Get all keys matching pattern
        let keys: Vec<String> = match conn.keys(pattern).await {
            Ok(k) => k,
            Err(err) => {
                if is_connection_error(err.kind()) {
                    warn!("Redis connection dropout on KEYS {}: {}", pattern, err);
                } else {
                    error!("Redis KEYS error for pattern {}: {}", pattern, err);
                }
                return;
            }
        };

        if keys.is_empty() {
            return;
        }

        // Delete all matching keys
        if let Err(err) = conn.del::<_, ()>(&keys).await {
            if is_connection_error(err.kind()) {
                warn!(
                    "Redis connection dropout on DEL pattern {}: {}",
                    pattern, err
                );
            } else {
                error!("Redis DEL error for pattern {}: {}", pattern, err);
            }
        } else {
            debug!(
                "Invalidated {} cache entries matching: {}",
                keys.len(),
                pattern
            );
        }
    }

    /// Invalidate all cached pool list entries.
    ///
    /// Call after a new pool is created so clients see fresh data on the next
    /// `GET /api/v1/pools` request.
    pub async fn invalidate_pools_cache(&self) {
        self.delete_pattern(POOLS_CACHE_PATTERN).await;
    }

    /// Invalidate all cached stats entries.
    ///
    /// Call after a pool is created or a prediction is placed so the next
    /// `GET /api/v1/stats` request reflects the updated aggregates.
    pub async fn invalidate_stats_cache(&self) {
        self.delete_pattern(STATS_CACHE_PATTERN).await;
    }

    /// Check if a cache entry exists without deserializing it
    ///
    /// Useful for cache-aside pattern to determine if we need to populate the cache
    pub async fn exists(&self, key: &str) -> bool {
        let manager = match self.manager.as_ref() {
            Some(m) => m,
            None => return false,
        };

        let mut conn = manager.clone();
        match conn.exists::<_, bool>(key).await {
            Ok(exists) => exists,
            Err(err) => {
                if is_connection_error(err.kind()) {
                    debug!("Redis connection dropout on EXISTS {}: {}", key, err);
                } else {
                    debug!("Redis EXISTS error for {}: {}", key, err);
                }
                false
            }
        }
    }

    /// Ping Redis to check connection health
    pub async fn ping(&self) -> bool {
        if self.simulate_available {
            return true;
        }

        let manager = match self.manager.as_ref() {
            Some(m) => m,
            None => return false,
        };

        let mut conn = manager.clone();
        let r: Result<String, _> = redis::cmd("PING").query_async(&mut conn).await;
        r.is_ok()
    }
}

/// Returns `true` for errors caused by a transient connection dropout.
///
/// The `ConnectionManager` automatically reconnects after a dropout, so
/// these errors are expected during the brief window before reconnection and
/// should be logged at `warn` rather than `error`.
fn is_connection_error(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::IoError | ErrorKind::BusyLoadingError)
}

/// Generate a cache key for a pools list query.
///
/// The key encodes all query parameters so that different filter/sort/page
/// combinations are stored independently in Redis.
pub fn pools_cache_key(
    sort_by: &str,
    category: Option<&str>,
    status: &str,
    limit: i64,
    offset: i64,
) -> String {
    match category {
        Some(cat) => format!("pools:{}:{}:{}:{}:{}", sort_by, cat, status, limit, offset),
        None => format!("pools:{}:all:{}:{}:{}", sort_by, status, limit, offset),
    }
}

/// Generate a cache key for a single pool's detail page.
pub fn pool_details_cache_key(pool_id: i64) -> String {
    format!("pool:{}:details", pool_id)
}

/// Generate a cache key for a user's paginated predictions list.
pub fn user_predictions_cache_key(address: &str, limit: i64, offset: i64) -> String {
    format!("user:{}:predictions:{}:{}", address, limit, offset)
}

/// Generate a cache key for the protocol stats endpoint.
///
/// The key encodes the optional `category` and `status` filter parameters so
/// that different filter combinations are cached independently.
pub fn stats_cache_key(category: Option<&str>, status: Option<&str>) -> String {
    let cat = category.unwrap_or("all");
    let st = status.unwrap_or("all");
    format!("stats:{}:{}", cat, st)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Key-generation tests ──────────────────────────────────────────────────

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(
            pools_cache_key("popular", Some("sports"), "active", 10, 0),
            "pools:popular:sports:active:10:0"
        );
        assert_eq!(
            pools_cache_key("new", None, "active", 20, 10),
            "pools:new:all:active:20:10"
        );
        assert_eq!(pool_details_cache_key(123), "pool:123:details");
        assert_eq!(
            user_predictions_cache_key("GABC123", 10, 0),
            "user:GABC123:predictions:10:0"
        );
    }

    // ── Disabled-cache / cache-miss tests ────────────────────────────────────

    /// A disabled cache must never claim to be available.
    #[tokio::test]
    async fn test_disabled_cache() {
        let cache = RedisCache::disabled();
        assert!(!cache.is_available());
        assert!(!cache.ping().await);

        // Should not panic on operations
        cache.set("test", &"value", 60).await;
        let result: Option<String> = cache.get("test").await;
        assert!(result.is_none());
        cache.delete("test").await;
    }

    /// GET on any key of a disabled cache is a cache miss — returns `None`.
    #[tokio::test]
    async fn cache_miss_on_disabled_cache_returns_none_for_any_key() {
        let cache = RedisCache::disabled();

        let result: Option<String> = cache.get("pools:popular:all:active:10:0").await;
        assert!(
            result.is_none(),
            "disabled cache must always miss on GET (pools key)"
        );

        let result: Option<serde_json::Value> = cache.get("pool:42:details").await;
        assert!(
            result.is_none(),
            "disabled cache must always miss on GET (pool-details key)"
        );

        let result: Option<Vec<u32>> = cache.get("user:GABC:predictions:10:0").await;
        assert!(
            result.is_none(),
            "disabled cache must always miss on GET (user predictions key)"
        );
    }

    /// Calling `set` followed by `get` on a disabled cache still returns `None`
    /// because no backing store exists.
    #[tokio::test]
    async fn cache_miss_after_set_on_disabled_cache() {
        let cache = RedisCache::disabled();

        cache.set("some-key", &42u32, 60).await;
        let result: Option<u32> = cache.get("some-key").await;
        assert!(
            result.is_none(),
            "GET after SET on a disabled cache must still be a cache miss"
        );
    }

    /// `delete` on a disabled cache must be a no-op (must not panic).
    #[tokio::test]
    async fn cache_miss_delete_on_disabled_cache_is_noop() {
        let cache = RedisCache::disabled();
        // Neither of these must panic
        cache.delete("nonexistent-key").await;
        cache.delete_pattern("pools:*").await;
    }

    /// Verifies that `is_available()` returns `false` for a disabled cache,
    /// confirming callers can guard cache operations correctly.
    #[test]
    fn disabled_cache_is_not_available() {
        let cache = RedisCache::disabled();
        assert!(
            !cache.is_available(),
            "disabled cache must report is_available() == false"
        );
    }

    /// `ping` on a disabled cache must return `false` (connection refused / absent).
    #[tokio::test]
    async fn disabled_cache_ping_returns_false() {
        let cache = RedisCache::disabled();
        assert!(
            !cache.ping().await,
            "ping on a disabled cache must return false"
        );
    }

    /// Cache misses must not be confused with empty-collection hits.
    /// `GET` for a key that was never set must return `None`, not `Some(vec![])`.
    #[tokio::test]
    async fn cache_miss_is_none_not_empty_collection() {
        let cache = RedisCache::disabled();
        let result: Option<Vec<String>> = cache.get("pools:new:all:active:10:0").await;
        assert!(
            result.is_none(),
            "a cache miss must be None, not Some(empty collection)"
        );
    }

    /// Cache miss on a key with a previously-set TTL (simulated via disabled
    /// cache) must also be `None`.
    #[tokio::test]
    async fn cache_miss_after_ttl_simulated_via_disabled_cache() {
        let cache = RedisCache::disabled();

        // Simulate: set with 1s TTL, then read back immediately (disabled = always miss)
        cache.set("ttl-key", &"data", 1).await;
        let result: Option<String> = cache.get("ttl-key").await;
        assert!(
            result.is_none(),
            "disabled cache never stores data so always misses even within TTL"
        );
    }

    // ── Cache-key uniqueness tests ────────────────────────────────────────────

    /// Different pagination parameters must produce distinct cache keys,
    /// ensuring pages are stored independently.
    #[test]
    fn pools_cache_keys_differ_by_offset() {
        let key_page1 = pools_cache_key("new", None, "active", 10, 0);
        let key_page2 = pools_cache_key("new", None, "active", 10, 10);
        assert_ne!(
            key_page1, key_page2,
            "different offsets must produce different cache keys"
        );
    }

    /// Different pool IDs must produce distinct cache keys.
    #[test]
    fn pool_details_keys_differ_by_id() {
        let key_a = pool_details_cache_key(1);
        let key_b = pool_details_cache_key(2);
        assert_ne!(
            key_a, key_b,
            "different pool IDs must produce different keys"
        );
    }

    /// Different user addresses must produce distinct cache keys.
    #[test]
    fn user_predictions_keys_differ_by_address() {
        let key_a = user_predictions_cache_key("ADDR_A", 10, 0);
        let key_b = user_predictions_cache_key("ADDR_B", 10, 0);
        assert_ne!(
            key_a, key_b,
            "different user addresses must produce different keys"
        );
    }

    /// The same key parameters must always produce the same key (deterministic).
    #[test]
    fn cache_key_generation_is_deterministic() {
        for _ in 0..5 {
            assert_eq!(
                pools_cache_key("popular", Some("crypto"), "active", 20, 40),
                "pools:popular:crypto:active:20:40"
            );
        }
    }

    #[test]
    fn test_stats_cache_key_generation() {
        assert_eq!(stats_cache_key(None, None), "stats:all:all");
        assert_eq!(stats_cache_key(Some("sports"), None), "stats:sports:all");
        assert_eq!(stats_cache_key(None, Some("active")), "stats:all:active");
        assert_eq!(
            stats_cache_key(Some("crypto"), Some("closed")),
            "stats:crypto:closed"
        );
    }

    #[test]
    fn test_stats_cache_key_uniqueness() {
        let key_all = stats_cache_key(None, None);
        let key_cat = stats_cache_key(Some("sports"), None);
        let key_st = stats_cache_key(None, Some("active"));
        let key_both = stats_cache_key(Some("sports"), Some("active"));

        assert_ne!(key_all, key_cat);
        assert_ne!(key_all, key_st);
        assert_ne!(key_all, key_both);
        assert_ne!(key_cat, key_both);
    }

    #[test]
    fn test_stats_cache_pattern_constant() {
        assert_eq!(STATS_CACHE_PATTERN, "stats:*");
    }

    #[tokio::test]
    async fn test_stats_cache_invalidation() {
        let cache = RedisCache::disabled();
        // Should not panic; no-ops on a disabled cache
        cache.invalidate_stats_cache().await;
    }

    #[test]
    fn test_stats_cache_ttl() {
        assert_eq!(STATS_CACHE_TTL, 30);
    }
}
