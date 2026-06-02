//! Integration tests for Redis cache expiration using testcontainers-rs.
//!
//! Each test spins up a throwaway Redis container via the shared fixture
//! module, creates a [`RedisCache`] connected to it, and verifies that cached
//! items actually expire after the configured TTL.
//!
//! # Resource lifecycle
//!
//! `setup_redis()` returns both the [`RedisCache`] **and** the container handle.
//! Tests must bind the container to a named variable (not `_`) so that it
//! stays alive for the entire test body.  At the end of each test the
//! container is dropped, which stops the Docker container and releases the
//! ephemeral port.

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::test_support;

    /// A value stored with a 1-second TTL should be retrievable immediately
    /// but become inaccessible after the TTL has elapsed.
    #[tokio::test]
    async fn cached_item_expires_after_short_ttl() {
        let (cache, container) = test_support::setup_redis().await;

        assert!(cache.is_available(), "Redis should be available");

        // Set a value with a 2-second TTL
        cache.set("test_expire_key", &"hello-world", 2).await;

        // Immediately retrievable
        let result: Option<String> = cache.get("test_expire_key").await;
        assert_eq!(
            result.as_deref(),
            Some("hello-world"),
            "cached value should be retrievable immediately after set"
        );

        // Wait for the TTL to expire (2 sec + small buffer)
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Should now be gone
        let result: Option<String> = cache.get("test_expire_key").await;
        assert!(
            result.is_none(),
            "cached value should have expired after TTL elapsed"
        );

        drop(container);
    }

    /// Multiple cached items with different TTLs should expire independently.
    #[tokio::test]
    async fn items_with_different_ttls_expire_independently() {
        let (cache, container) = test_support::setup_redis().await;

        assert!(cache.is_available(), "Redis should be available");

        // Set two keys with different TTLs: 1 second and 5 seconds
        cache.set("expire_short", &"short-lived", 1).await;
        cache.set("expire_long", &"long-lived", 5).await;

        // Both retrievable immediately
        let short: Option<String> = cache.get("expire_short").await;
        let long: Option<String> = cache.get("expire_long").await;
        assert_eq!(short.as_deref(), Some("short-lived"));
        assert_eq!(long.as_deref(), Some("long-lived"));

        // Wait for the short TTL to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Short-lived key should be gone, long-lived key should persist
        let short: Option<String> = cache.get("expire_short").await;
        let long: Option<String> = cache.get("expire_long").await;
        assert!(
            short.is_none(),
            "short-lived key should have expired after 1s TTL elapsed"
        );
        assert_eq!(
            long.as_deref(),
            Some("long-lived"),
            "long-lived key should still be present before its TTL expires"
        );

        // Now wait for the long TTL to expire too
        tokio::time::sleep(Duration::from_secs(4)).await;

        let long: Option<String> = cache.get("expire_long").await;
        assert!(
            long.is_none(),
            "long-lived key should also have expired after its TTL elapsed"
        );

        drop(container);
    }

    /// A zero TTL should cause the item to expire immediately.
    #[tokio::test]
    async fn zero_ttl_expires_immediately() {
        let (cache, container) = test_support::setup_redis().await;

        assert!(cache.is_available(), "Redis should be available");

        // Setting with TTL = 0 (Redis SETEX treats 0 TTL as immediate expiry)
        cache.set("test_zero_ttl", &"gone-soon", 0).await;

        // Give Redis a moment to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        let result: Option<String> = cache.get("test_zero_ttl").await;
        assert!(
            result.is_none(),
            "value set with TTL=0 should not be retrievable"
        );

        drop(container);
    }

    /// Verifies that a value can be retrieved immediately after being cached
    /// and does NOT prematurely expire before its TTL.
    #[tokio::test]
    async fn cached_item_persists_within_ttl_window() {
        let (cache, container) = test_support::setup_redis().await;

        assert!(cache.is_available(), "Redis should be available");

        cache.set("test_persist_key", &"still-here", 10).await;

        // Immediately retrievable
        let result: Option<String> = cache.get("test_persist_key").await;
        assert_eq!(result.as_deref(), Some("still-here"));

        // Should still be retrievable after a few seconds (well within 10s TTL)
        tokio::time::sleep(Duration::from_secs(3)).await;

        let result: Option<String> = cache.get("test_persist_key").await;
        assert_eq!(
            result.as_deref(),
            Some("still-here"),
            "value should persist within its TTL window"
        );

        drop(container);
    }

    /// Verify that complex serializable types (not just strings) are properly
    /// cached and expire correctly.
    #[tokio::test]
    async fn complex_types_expire_correctly() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct MarketData {
            pool_id: i64,
            price: f64,
            volume: i64,
        }

        let (cache, container) = test_support::setup_redis().await;

        assert!(cache.is_available(), "Redis should be available");

        let data = MarketData {
            pool_id: 42,
            price: 1.5,
            volume: 100_000,
        };

        cache.set("test_complex_key", &data, 2).await;

        // Immediately retrievable
        let result: Option<MarketData> = cache.get("test_complex_key").await;
        assert_eq!(result.as_ref(), Some(&data));

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_secs(3)).await;

        let result: Option<MarketData> = cache.get("test_complex_key").await;
        assert!(
            result.is_none(),
            "complex type should also expire after TTL elapsed"
        );

        drop(container);
    }
}