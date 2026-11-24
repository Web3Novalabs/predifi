//! Proactive Redis cache warming for the hottest read paths.
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
//!
//! Issue #2205 extends the warmer beyond the pool lists to the other hot,
//! aggregate-heavy reads that are hit on every landing / dashboard view:
//! protocol stats, the default leaderboards, the tag list, and the
//! per-pool leaderboards of the currently most popular pools. Each warm
//! attempt is counted in `app_redis_cache_warm_total{cache_name,result}`.

use sqlx::PgPool;
use tracing::{debug, warn};

use crate::redis_cache::{
    leaderboard_cache_key, pool_leaderboard_cache_key, pools_cache_key, stats_cache_key,
    tags_cache_key, RedisCache, LEADERBOARD_CACHE_TTL, POOLS_CACHE_TTL,
    POOL_LEADERBOARD_CACHE_TTL, STATS_CACHE_TTL, TAGS_CACHE_TTL,
};
use crate::routes::v1::PoolsResponse;

/// Page size used when warming — matches the frontend's default page size
/// and `GET /api/v1/pools`'s own default (`POOLS_DEFAULT_PAGE_SIZE`).
const WARM_PAGE_SIZE: i64 = 20;

/// Page size used for warmed leaderboards — matches the default `limit` of
/// `GET /api/v1/leaderboard`.
const WARM_LEADERBOARD_LIMIT: i64 = 20;

/// How many of the most-staked active pools get their own leaderboard warmed.
const WARM_TOP_POOL_COUNT: usize = 5;

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

/// `(rank_by, period)` combinations of the global leaderboard warmed on every
/// tick: every ranking in its default all-time view, plus the weekly and
/// monthly volume views the leaderboard page offers.
const WARM_LEADERBOARDS: &[(&str, &str)] = &[
    ("volume", "all"),
    ("winnings", "all"),
    ("win_rate", "all"),
    ("streak", "all"),
    ("volume", "week"),
    ("volume", "month"),
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

/// Refresh the Redis cache entries for every query in [`WARM_QUERIES`], then
/// the stats, tag, and leaderboard entries (#2205).
///
/// No-ops cheaply when Redis is unavailable (the underlying `set` calls are
/// already fail-open) and logs a warning, rather than failing the tick, if a
/// single query's database lookup errors — one slow/broken entry should
/// not stop the others from warming.
pub async fn warm_popular_pools(pool: &PgPool, redis: &RedisCache) {
    if !redis.is_available() {
        return;
    }

    let mut popular_pool_ids: Vec<i64> = Vec::new();

    for &(sort_by, status) in WARM_QUERIES {
        let cache_key = pools_cache_key(sort_by, None, status, WARM_PAGE_SIZE, 0);

        let result = tokio::try_join!(
            crate::tags::list_pools(pool, sort_by, None, None, status, None, WARM_PAGE_SIZE, 0),
            crate::tags::count_pools(pool, None, None, status, None)
        );

        match result {
            Ok((pools, total)) => {
                if sort_by == "popular" {
                    popular_pool_ids = pools
                        .iter()
                        .take(WARM_TOP_POOL_COUNT)
                        .map(|p| p.pool_id)
                        .collect();
                }
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
                redis.record_warm("pools", "ok");
                debug!(sort_by, status, "warmed pools cache entry");
            }
            Err(error) => {
                redis.record_warm("pools", "error");
                warn!(sort_by, status, %error, "failed to warm pools cache entry");
            }
        }
    }

    warm_stats(pool, redis).await;
    warm_tags(pool, redis).await;
    warm_leaderboards(pool, redis).await;
    warm_pool_leaderboards(pool, redis, &popular_pool_ids).await;
}

/// Warm the unfiltered protocol-stats entry (`GET /api/v1/stats`).
async fn warm_stats(pool: &PgPool, redis: &RedisCache) {
    match crate::db::get_protocol_stats(pool, None, None).await {
        Ok(stats) => {
            redis
                .set(
                    &stats_cache_key(None, None),
                    &serde_json::json!(&stats),
                    STATS_CACHE_TTL,
                )
                .await;
            redis.record_warm("stats", "ok");
        }
        Err(error) => {
            redis.record_warm("stats", "error");
            warn!(%error, "failed to warm stats cache entry");
        }
    }
}

/// Warm the distinct-tag list (`GET /api/v1/tags`).
async fn warm_tags(pool: &PgPool, redis: &RedisCache) {
    match crate::tags::list_distinct_tags(pool).await {
        Ok(tags) => {
            redis
                .set(
                    &tags_cache_key(),
                    &serde_json::json!({ "tags": tags }),
                    TAGS_CACHE_TTL,
                )
                .await;
            redis.record_warm("tags", "ok");
        }
        Err(error) => {
            redis.record_warm("tags", "error");
            warn!(%error, "failed to warm tags cache entry");
        }
    }
}

/// Warm every combination in [`WARM_LEADERBOARDS`] (first page only).
async fn warm_leaderboards(pool: &PgPool, redis: &RedisCache) {
    for &(rank_by, period) in WARM_LEADERBOARDS {
        match crate::routes::v1::build_leaderboard_payload(
            pool,
            rank_by,
            period,
            WARM_LEADERBOARD_LIMIT,
            0,
        )
        .await
        {
            Ok(payload) => {
                redis
                    .set(
                        &leaderboard_cache_key(rank_by, period, WARM_LEADERBOARD_LIMIT, 0),
                        &payload,
                        LEADERBOARD_CACHE_TTL,
                    )
                    .await;
                redis.record_warm("leaderboard", "ok");
            }
            Err(error) => {
                redis.record_warm("leaderboard", "error");
                warn!(rank_by, period, %error, "failed to warm leaderboard cache entry");
            }
        }
    }
}

/// Warm the default (volume) leaderboard of each currently popular pool, so
/// the pool detail page's leaderboard widget is a cache hit for the pools
/// that receive the most traffic.
async fn warm_pool_leaderboards(pool: &PgPool, redis: &RedisCache, pool_ids: &[i64]) {
    for &pool_id in pool_ids {
        match crate::db::get_leaderboard_extended(
            pool,
            "volume",
            "all",
            Some(pool_id),
            WARM_LEADERBOARD_LIMIT,
            0,
        )
        .await
        {
            Ok(entries) => {
                let payload = serde_json::json!({
                    "pool_id": pool_id,
                    "leaderboard": entries,
                    "rank_by": "volume",
                    "limit": WARM_LEADERBOARD_LIMIT,
                    "offset": 0,
                });
                redis
                    .set(
                        &pool_leaderboard_cache_key(pool_id, "volume", WARM_LEADERBOARD_LIMIT, 0),
                        &payload,
                        POOL_LEADERBOARD_CACHE_TTL,
                    )
                    .await;
                redis.record_warm("poolboard", "ok");
            }
            Err(error) => {
                redis.record_warm("poolboard", "error");
                warn!(pool_id, %error, "failed to warm pool leaderboard cache entry");
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

    /// Warmed leaderboard keys must match the key `get_leaderboard` computes
    /// for the default first-page request of the same ranking/window.
    #[test]
    fn warm_leaderboard_keys_match_default_request_key() {
        for &(rank_by, period) in WARM_LEADERBOARDS {
            assert_eq!(
                leaderboard_cache_key(rank_by, period, WARM_LEADERBOARD_LIMIT, 0),
                leaderboard_cache_key(rank_by, period, 20, 0)
            );
        }
    }
}
