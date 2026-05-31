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

use redis::{aio::ConnectionManager, AsyncCommands, RedisError};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error, warn};

/// Default TTL for cached pool data (60 seconds)
pub const POOLS_CACHE_TTL: u64 = 60;

/// Default TTL for cached pool details (30 seconds)
pub const POOL_DETAILS_CACHE_TTL: u64 = 30;

/// Default TTL for user predictions (45 seconds)
pub const USER_PREDICTIONS_CACHE_TTL: u64 = 45;

/// Thread-safe Redis cache client with graceful fail-open behaviour.
///
/// All operations silently no-op when Redis is unavailable so the application
/// continues to function without caching rather than returning errors to users.
/// Use [`RedisCache::disabled`] to create an always-no-op instance for tests.
#[derive(Clone)]
pub struct RedisCache {
    manager: Option<ConnectionManager>,
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
                    }
                }
                Err(err) => {
                    warn!("Failed to create Redis connection manager: {}", err);
                    Self { manager: None }
                }
            },
            Err(err) => {
                warn!("Failed to create Redis client: {}", err);
                Self { manager: None }
            }
        }
    }

    /// Create a disabled cache instance (for testing or when Redis is not configured)
    pub fn disabled() -> Self {
        Self { manager: None }
    }

    /// Check if Redis is available
    pub fn is_available(&self) -> bool {
        self.manager.is_some()
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

        match conn.get::<_, String>(key).await {
            Ok(data) => match serde_json::from_str::<T>(&data) {
                Ok(value) => {
                    debug!("Cache hit: {}", key);
                    Some(value)
                }
                Err(err) => {
                    error!("Failed to deserialize cached value for {}: {}", key, err);
                    None
                }
            },
            Err(RedisError::Nil) => {
                debug!("Cache miss: {}", key);
                None
            }
            Err(err) => {
                error!("Redis GET error for {}: {}", key, err);
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
            error!("Redis SET error for {}: {}", key, err);
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
            error!("Redis DEL error for {}: {}", key, err);
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
                error!("Redis KEYS error for pattern {}: {}", pattern, err);
                return;
            }
        };

        if keys.is_empty() {
            return;
        }

        // Delete all matching keys
        if let Err(err) = conn.del::<_, ()>(&keys).await {
            error!("Redis DEL error for pattern {}: {}", pattern, err);
        } else {
            debug!(
                "Invalidated {} cache entries matching: {}",
                keys.len(),
                pattern
            );
        }
    }

    /// Ping Redis to check connection health
    pub async fn ping(&self) -> bool {
        let manager = match self.manager.as_ref() {
            Some(m) => m,
            None => return false,
        };

        let mut conn = manager.clone();
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .is_ok()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
