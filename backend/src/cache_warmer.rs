//! Proactive Redis cache warming for the most popular pool-list queries.
//!
//! Part of issue #1938 (Redis cache hit ratio improvement). Without this,
//! the `pools:*` cache entries populated by `GET /api/v1/pools` only exist
//! after the *first* visitor pays the full database round trip, and every
//! time [`crate::redis_cache::POOLS_CACHE_TTL`] elapses the next visitor
//! pays it again. Landing-page traffic overwhelmingly requests the default,
//! first page of *active* pools under one of a handful of sort orders, so
//! proactively refreshing just those entries on a fixed interval keeps them
//! permanently warm and removes that cold-cache penalty for the vast
//! majority of real traffic.
//!
//! This intentionally does **not** attempt to warm every possible
//! category/tag/search combination — the long tail is still populated
//! lazily by the existing cache-aside path in `get_pools`.

use sqlx::PgPool;
use tracing::{debug, warn};

use crate::redis_cache::{pools_cache_key, RedisCache, POOLS_CACHE_TTL};
use crate::routes::v1::PoolsResponse;

/// Page size used when warming — matches the frontend's default page size
/// and `GET /api/v1/pools`'s own default (`POOLS_DEFAULT_PAGE_SIZE`).
const WARM_PAGE_SIZE: i64 = 20;

/// `(sort_by, status)` combinations warmed on every tick.
///
/// These are the default landing-page queries — the first page of active
/// pools under each sort order a client can pick from the UI — with no
/// category, tag, or search filter, since those are long-tail and cheaper
/// to leave to the lazy cache-aside path.
const WARM_QUERIES: &[(&str, &str)] = &[
    ("popular", "active"),
    ("new", "active"),
    ("ending_soon", "active"),
];

/// Run [`warm_popular_pools`] once immediately, then every `interval`.
///
/// Intended to be spawned as a background worker alongside the price-cache
/// fetcher and notification sweep (see `server::run_with_signal`), and
/// drained the same way on shutdown.
pub async fn run_warm_loop(pool: PgPool, redis: RedisCache, interval: std::time::Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        warm_popular_pools(&pool, &redis).await;
    }
}

/// Refresh the Redis cache entries for every query in [`WARM_QUERIES`].
///
/// No-ops cheaply when Redis is unavailable (the underlying `set` calls are
/// already fail-open) and logs a warning, rather than failing the tick, if a
/// single query's database lookup errors — one slow/broken sort order should
/// not stop the others from warming.
pub async fn warm_popular_pools(pool: &PgPool, redis: &RedisCache) {
    if !redis.is_available() {
        return;
    }

    for &(sort_by, status) in WARM_QUERIES {
        let cache_key = pools_cache_key(sort_by, None, status, WARM_PAGE_SIZE, 0);

        let result = tokio::try_join!(
            crate::tags::list_pools(pool, sort_by, None, None, status, None, WARM_PAGE_SIZE, 0),
            crate::tags::count_pools(pool, None, None, status, None)
        );

        match result {
            Ok((pools, total)) => {
                let response = PoolsResponse {
                    pools,
                    total,
                    limit: WARM_PAGE_SIZE,
                    offset: 0,
                    status: status.to_string(),
                    category: None,
                    sort_by: sort_by.to_string(),
                    q: None,
                };
                redis
                    .set(&cache_key, &serde_json::json!(&response), POOLS_CACHE_TTL)
                    .await;
                debug!(sort_by, status, "warmed pools cache entry");
            }
            Err(error) => {
                warn!(sort_by, status, %error, "failed to warm pools cache entry");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every warmed query must target `status = "active"` — warming a
    /// closed/settled query would waste a database round trip on data that
    /// landing-page traffic never actually requests by default.
    #[test]
    fn warm_queries_are_all_active_status() {
        for &(_, status) in WARM_QUERIES {
            assert_eq!(status, "active");
        }
    }

    /// The warmed cache key for each sort order must exactly match the key
    /// `get_pools` would compute for the same default (no category/tag/q,
    /// first page) request, or warming would populate a key nobody reads.
    #[test]
    fn warm_cache_keys_match_default_get_pools_key() {
        for &(sort_by, status) in WARM_QUERIES {
            let warmed = pools_cache_key(sort_by, None, status, WARM_PAGE_SIZE, 0);
            let request_default = pools_cache_key(sort_by, None, status, 20, 0);
            assert_eq!(warmed, request_default);
        }
    }
}
