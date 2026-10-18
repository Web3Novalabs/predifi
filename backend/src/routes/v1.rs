use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::config::Config;
use crate::db::PoolCreatedEvent;
use crate::db::PredictionPlacedEvent;
use crate::metrics::SharedMetrics;
use crate::pool_cache::PoolCache;
use crate::price_cache::PriceCache;
use crate::redis_cache::RedisCache;
use crate::response::{error_codes, ApiResponse};
use crate::validated_types::{BoundedI64, NonEmptyString, PoolSortBy, PoolStatus, StellarAddress};

/// Standardized HTTP 500 response.
///
/// Logs the underlying cause server-side (via `AppError::Internal`) and
/// returns the generic `ApiResponse` error envelope so internal error
/// details (DB messages, low-level errors) never leak to clients.
fn internal_error_response(err: impl std::fmt::Display) -> axum::response::Response {
    use axum::response::IntoResponse;
    crate::errors::AppError::Internal(err.to_string()).into_response()
}

/// Struct representing fee information, matching the contract structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeeInfo {
    /// Protocol (treasury) fee in basis points.
    pub treasury_fee_bps: u32,
    /// Referral share of the protocol fee in basis points.
    pub referral_fee_bps: u32,
}

/// `GET /api/v1/health` component-level health check (see [`crate::health`]).
pub async fn health(State(state): State<AppState>) -> axum::response::Response {
    crate::health::check_all(&state.config, state.db.as_ref(), &state.redis, &state.cache)
        .await
        .into_response_with_version("v1")
}

/// `GET /api/v1` version discovery endpoint.
async fn index() -> Json<serde_json::Value> {
    Json(json!({
        "name": "predifi-backend",
        "version": "v1"
    }))
}

/// Shared state for all v1 routes.
#[derive(Clone)]
pub struct AppState {
    /// Validated runtime configuration (fees, URLs, timeouts, etc.).
    pub config: Arc<Config>,
    /// In-memory oracle price cache refreshed every 60 seconds.
    pub cache: PriceCache,
    /// Short-TTL cache for pool-detail lookups (#1369).
    pub pool_cache: PoolCache,
    /// Redis cache client for hot-path response caching.
    pub redis: RedisCache,
    /// Optional DB pool — absent in unit tests that don't need a database.
    pub db: Option<sqlx::PgPool>,
    /// Shared Prometheus metrics registry.
    pub metrics: SharedMetrics,
    /// Broadcast channel for pushing live prediction events to WebSocket clients.
    pub event_bus: crate::ws::EventBus,
}

impl axum::extract::FromRef<AppState> for Arc<Config> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl axum::extract::FromRef<AppState> for PriceCache {
    fn from_ref(state: &AppState) -> Self {
        state.cache.clone()
    }
}

impl axum::extract::FromRef<AppState> for SharedMetrics {
    fn from_ref(state: &AppState) -> Self {
        state.metrics.clone()
    }
}

impl axum::extract::FromRef<AppState> for Option<sqlx::PgPool> {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl axum::extract::FromRef<AppState> for crate::ws::EventBus {
    fn from_ref(state: &AppState) -> Self {
        state.event_bus.clone()
    }
}

// ── Admin: runtime log-level control ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SetLogLevelRequest {
    /// New log filter: a level (`"debug"`) or `RUST_LOG`-style per-module
    /// directives (`"info,predifi_backend::ws=debug,sqlx=warn"`).
    pub level: String,
}

#[derive(Debug, Serialize)]
pub struct SetLogLevelResponse {
    pub status: String,
    pub level: String,
}

/// `PATCH /api/v1/admin/log-level` — change the active log filter at runtime.
///
/// Accepts per-module directives, so one noisy module can be turned up in
/// production without flooding the logs from everything else.
///
/// Restricted to callers that present a valid admin API key in the
/// `x-admin-api-key` header.  The key must match the `PREDIFI_ADMIN_API_KEY`
/// environment variable.  Returns 401 when the header is absent or does not
/// match.
///
/// The new level is applied immediately and remains in effect until the next
/// change or a process restart (which reverts to the configured default).
pub async fn set_log_level(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<SetLogLevelRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let admin_key = std::env::var("PREDIFI_ADMIN_API_KEY").ok();

    let provided = headers
        .get("x-admin-api-key")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let authenticated = match (&admin_key, provided) {
        (Some(expected), Some(provided)) if expected == &provided => true,
        _ => false,
    };

    if !authenticated {
        return ApiResponse::<()>::error(
            StatusCode::UNAUTHORIZED,
            error_codes::UNAUTHORIZED,
            "invalid or missing admin credentials",
        )
        .into_response();
    }

    let level = body.level.trim().to_string();
    if level.is_empty() {
        return ApiResponse::<()>::error(
            StatusCode::BAD_REQUEST,
            error_codes::INVALID_INPUT,
            "level must not be empty",
        )
        .into_response();
    }

    match crate::telemetry::try_reload_log_level(&level) {
        Ok(()) => {
            tracing::info!(log_level = %level, "log level updated via admin endpoint");
            ApiResponse::success(SetLogLevelResponse {
                status: "ok".to_string(),
                level,
            })
            .into_response()
        }
        Err(e) => ApiResponse::<()>::error(StatusCode::BAD_REQUEST, error_codes::INVALID_INPUT, e)
            .into_response(),
    }
}

// ── Task 2: Active Pools API ──────────────────────────────────────────────────

/// Default page size for `GET /api/v1/pools` when `limit` is omitted.
const POOLS_DEFAULT_PAGE_SIZE: i64 = 20;
/// Maximum page size for `GET /api/v1/pools`. A requested `limit` above this
/// is clamped down to it rather than rejected with 400, so a client that
/// over-requests still gets a bounded, servable page (see #1734).
const POOLS_MAX_PAGE_SIZE: i64 = 100;

/// Query parameters for the `GET /api/v1/pools` endpoint.
#[derive(Debug, Deserialize)]
pub struct PoolsQuery {
    /// Sort order: "popular" | "ending_soon" | "new" (default)
    pub sort_by: Option<PoolSortBy>,
    /// Category filter, e.g. "Sports", "Crypto"
    pub category: Option<NonEmptyString>,
    /// Status filter: "active" | "closed" | "settled" (default: "active")
    pub status: Option<PoolStatus>,
    /// Comma-separated tag filter, e.g. "btc,price-prediction". A pool
    /// matches if any of its tags overlap this list.
    pub tags: Option<String>,
    /// Full-text search query across pool name, category, and tags.
    /// Uses PostgreSQL `plainto_tsquery` + ILIKE fallback.
    pub q: Option<String>,
    /// Page size. Missing defaults to [`POOLS_DEFAULT_PAGE_SIZE`]; a value
    /// outside `[1, POOLS_MAX_PAGE_SIZE]` is clamped into range in the
    /// handler rather than rejected — unlike most other paginated endpoints
    /// in this file, which use [`BoundedI64`] and reject out-of-range values.
    pub limit: Option<i64>,
    /// Zero-based page offset, minimum 0.
    pub offset: Option<BoundedI64<0, 9223372036854775807>>,
}

/// Resolve the effective page size for `GET /api/v1/pools`: missing defaults
/// to [`POOLS_DEFAULT_PAGE_SIZE`], and anything outside `[1,
/// POOLS_MAX_PAGE_SIZE]` is clamped into range (never rejected). See #1734.
fn resolve_pools_page_size(limit: Option<i64>) -> i64 {
    limit
        .unwrap_or(POOLS_DEFAULT_PAGE_SIZE)
        .clamp(1, POOLS_MAX_PAGE_SIZE)
}

#[derive(Debug, Serialize)]
pub struct PoolsResponse {
    pub pools: Vec<crate::tags::PoolListingRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub status: String,
    pub category: Option<String>,
    pub sort_by: String,
    /// Echoed full-text search query, when present.
    pub q: Option<String>,
}

/// `GET /api/v1/pools/:id` — get detailed pool information with real-time odds.
pub async fn get_pool_by_id_handler(
    State(state): State<AppState>,
    Path(pool_id): Path<i64>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use crate::pool_cache::CacheLookup;
    match state.pool_cache.get(pool_id) {
        CacheLookup::Found(cached) => return ApiResponse::success(cached).into_response(),
        CacheLookup::NotFound => {
            // Negative cache hit: a previous lookup already confirmed this
            // pool doesn't exist, so serve 404 without touching the database.
            return ApiResponse::<()>::error(
                StatusCode::NOT_FOUND,
                error_codes::NOT_FOUND,
                "pool not found",
            )
            .into_response();
        }
        CacheLookup::Miss => {}
    }

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::db::get_pool_with_odds(db, pool_id).await {
        Ok(Some(pool)) => {
            state.pool_cache.set(pool_id, pool.clone());
            ApiResponse::success(pool).into_response()
        }
        Ok(None) => {
            state.pool_cache.set_missing(pool_id);
            ApiResponse::<()>::error(
                StatusCode::NOT_FOUND,
                error_codes::NOT_FOUND,
                "pool not found",
            )
            .into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

/// Query parameters for the `GET /api/v1/stats` endpoint.
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// Category filter, e.g. "Sports", "Crypto"
    pub category: Option<NonEmptyString>,
    /// Pool state filter: "active" | "closed" | "settled"
    pub status: Option<PoolStatus>,
}

/// `GET /api/v1/stats` — protocol-wide aggregate statistics.
///
/// Implements the **cache-aside pattern**:
/// 1. Check Redis for a cached result keyed by `category` and `status`.
/// 2. On cache hit: return the cached value.
/// 3. On cache miss: fetch from the database and cache for
///    [`STATS_CACHE_TTL`](crate::redis_cache::STATS_CACHE_TTL) seconds.
pub async fn get_stats(
    State(state): State<AppState>,
    Query(params): Query<StatsQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    let category = params.category.as_ref().map(|s| s.as_str());
    let status = params.status.map(|s| s.as_str());

    let cache_key = crate::redis_cache::stats_cache_key(category, status);

    if let Some(cached) = state.redis.get::<serde_json::Value>(&cache_key).await {
        return ApiResponse::success(cached).into_response();
    }

    match crate::db::get_protocol_stats(db, category, status).await {
        Ok(stats) => {
            let json_response = serde_json::json!(&stats);
            state
                .redis
                .set(
                    &cache_key,
                    &json_response,
                    crate::redis_cache::STATS_CACHE_TTL,
                )
                .await;
            // Pre-existing bug fixed in passing (found while wiring up the
            // same cache-aside pattern for #1938): this used to return
            // `ApiResponse::success(stats)` here — `{"status":"success",
            // "data": <stats>}` — while the cache-hit branch above returns
            // the un-enveloped `stats` object directly. A client polling
            // this endpoint would see its response shape silently change
            // depending on whether the TTL had expired. Returning the same
            // raw shape that's cached (matching `get_pools`'s cache-aside
            // path) makes a hit and a miss identical on the wire.
            (StatusCode::OK, Json(json_response)).into_response()
        }
        Err(e) => internal_error_response(e),
    }
}
/// `GET /api/v1/pools` — paginated list of pools with optional filters.
///
/// Checks the Redis cache first; falls back to the database on a cache miss
/// and stores the result for [`POOLS_CACHE_TTL`](crate::redis_cache::POOLS_CACHE_TTL) seconds.
/// `GET /api/v1/pools` — paginated list of pools with optional filters.
///
/// Implements the **cache-aside pattern**:
///
/// 1. Check Redis cache for the queried parameters (sort_by, category, status, limit, offset)
/// 2. If cache hit: Return the cached response
/// 3. If cache miss: Fetch from database and cache the result for POOLS_CACHE_TTL (60s)
///
/// This reduces database load for frequently accessed pool lists while keeping
/// data relatively fresh. When a new pool is created, the entire cache is
/// invalidated (see `ingest_pool_created`) to prevent stale results.
///
/// Cache is keyed by all query parameters, so different sort/filter combinations
/// are cached independently. For example, `/pools?sort=popular` and `/pools?sort=new`
/// maintain separate cache entries.
pub async fn get_pools(
    State(state): State<AppState>,
    Query(params): Query<PoolsQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let sort_by = params.sort_by.map(|s| s.as_str()).unwrap_or("new");
    let category = params.category.as_ref().map(|s| s.as_str());
    let status = params.status.map(|s| s.as_str()).unwrap_or("active");
    // Clamp rather than reject: a client asking for more than the max page
    // size still gets a bounded, servable page instead of a 400 (#1734).
    let limit = resolve_pools_page_size(params.limit);
    let offset = params.offset.map(|b| b.get()).unwrap_or(0);
    let tags: Option<Vec<String>> = params
        .tags
        .as_ref()
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty());
    // Full-text search query — trim and convert empty to None.
    let q: Option<String> = params
        .q
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    // Cache-aside pattern: Step 1 — Check cache
    let mut cache_key =
        crate::redis_cache::pools_cache_key(sort_by, category, status, limit, offset);
    if let Some(tags) = &tags {
        let sanitized: Vec<String> = tags
            .iter()
            .map(|t| crate::redis_cache::sanitize_key_component(t))
            .filter(|t| !t.is_empty())
            .collect();
        if !sanitized.is_empty() {
            cache_key = format!("{cache_key}:tags:{}", sanitized.join(","));
        }
    }
    if let Some(q_val) = &q {
        cache_key = format!(
            "{cache_key}:q:{}",
            crate::redis_cache::sanitize_key_component(q_val)
        );
    }

    if let Some(cached_response) = state.redis.get::<serde_json::Value>(&cache_key).await {
        // Cache hit — return cached data
        return (StatusCode::OK, Json(cached_response)).into_response();
    }

    // Cache miss — Step 2: Fetch from database
    match tokio::try_join!(
        crate::tags::list_pools(
            db,
            sort_by,
            category,
            tags.as_deref(),
            status,
            q.as_deref(),
            limit,
            offset
        ),
        crate::tags::count_pools(db, category, tags.as_deref(), status, q.as_deref())
    ) {
        Ok((pools, total)) => {
            if status == "active" {
                state.metrics.active_pools.set(total as f64);
            }

            let response = PoolsResponse {
                pools,
                total,
                limit,
                offset,
                status: status.to_string(),
                category: category.map(|s| s.to_string()),
                sort_by: sort_by.to_string(),
                q: q.clone(),
            };

            let json_response = json!(&response);

            // Step 3: Cache the response for 60 seconds
            state
                .redis
                .set(
                    &cache_key,
                    &json_response,
                    crate::redis_cache::POOLS_CACHE_TTL,
                )
                .await;

            // Return the response
            (StatusCode::OK, Json(json_response)).into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

// ── Task 3: User Prediction History API ──────────────────────────────────────

/// Generic pagination query parameters used by history and prediction endpoints.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    /// Maximum number of results to return (capped at 100, default 20).
    pub limit: Option<BoundedI64<1, 100>>,
    /// Zero-based offset for pagination (default 0).
    pub offset: Option<BoundedI64<0, 9223372036854775807>>,
}

/// `GET /api/v1/users/:address/history` — paginated prediction history for a user.
pub async fn get_user_history(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
    Query(params): Query<PaginationQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let limit = params.limit.map(|b| b.get()).unwrap_or(20);
    let offset = params.offset.map(|b| b.get()).unwrap_or(0);

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::db::get_user_prediction_history(db, address.as_str(), limit, offset).await {
        Ok(rows) => {
            let response = json!({
                "address": address,
                "predictions": rows,
                "limit": limit,
                "offset": offset,
            });
            ApiResponse::success(response).into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

/// `GET /api/v1/users/:address/predictions` — enhanced user predictions with current pool status.
///
/// This is one of the most frequently polled endpoints (every user dashboard
/// load) and was previously hitting the database on every request despite
/// `user_predictions_cache_key`/`USER_PREDICTIONS_CACHE_TTL` already existing
/// in `redis_cache.rs` for exactly this purpose (see #1938). Implements the
/// same cache-aside pattern as `get_pools`/`get_stats`:
/// 1. Check Redis for a cached page keyed by address, limit, and offset.
/// 2. On cache hit: return the cached value.
/// 3. On cache miss: fetch from the database and cache for
///    [`USER_PREDICTIONS_CACHE_TTL`](crate::redis_cache::USER_PREDICTIONS_CACHE_TTL) seconds.
///
/// A new prediction for this address invalidates only this user's cached
/// pages (see `RedisCache::invalidate_user_predictions_cache`), so the TTL
/// mainly protects against repeated polling rather than being the sole
/// source of freshness.
pub async fn get_user_predictions(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
    Query(params): Query<PaginationQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let limit = params.limit.map(|b| b.get()).unwrap_or(20);
    let offset = params.offset.map(|b| b.get()).unwrap_or(0);

    let cache_key = crate::redis_cache::user_predictions_cache_key(address.as_str(), limit, offset);
    if let Some(cached) = state.redis.get::<serde_json::Value>(&cache_key).await {
        return (StatusCode::OK, Json(cached)).into_response();
    }

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::db::get_user_predictions(db, address.as_str(), limit, offset).await {
        Ok(predictions) => {
            let response = json!({
                "address": address,
                "predictions": predictions,
                "limit": limit,
                "offset": offset,
                "total_predictions": predictions.len(),
            });
            state
                .redis
                .set(
                    &cache_key,
                    &response,
                    crate::redis_cache::USER_PREDICTIONS_CACHE_TTL,
                )
                .await;
            // Return the same un-enveloped shape cached above, so a cache
            // hit and a cache miss are byte-for-byte identical on the wire
            // (matches the `get_pools` cache-aside path; see the note in
            // `get_stats` above for why this matters).
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

/// Query parameters for the `GET /api/v1/leaderboard` endpoint.
#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    /// Maximum number of results to return (capped at 100, default 20).
    pub limit: Option<BoundedI64<1, 100>>,
    /// Zero-based offset for pagination (default 0).
    pub offset: Option<BoundedI64<0, 9223372036854775807>>,
    /// Ranking type: "volume" | "winnings" | "win_rate" | "streak" (default: "volume")
    pub rank_by: Option<NonEmptyString>,
    /// Time window: "week" | "month" | "all" (default: "all"). Ignored for `rank_by=winnings`.
    pub period: Option<NonEmptyString>,
}

/// `GET /api/v1/leaderboard` — user rankings by betting volume, winnings, win
/// rate, or current win streak, optionally scoped to a time window
/// ("week" | "month" | "all").
pub async fn get_leaderboard(
    State(state): State<AppState>,
    Query(params): Query<LeaderboardQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let limit = params.limit.map(|b| b.get()).unwrap_or(20);
    let offset = params.offset.map(|b| b.get()).unwrap_or(0);
    let rank_by = params
        .rank_by
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("volume");
    let period = params.period.as_ref().map(|s| s.as_str()).unwrap_or("all");

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match rank_by {
        "winnings" => match crate::db::get_users_by_winnings(db, limit, offset).await {
            Ok(users) => {
                let response = json!({
                    "leaderboard": users,
                    "rank_by": "winnings",
                    "period": "all",
                    "limit": limit,
                    "offset": offset,
                });
                ApiResponse::success(response).into_response()
            }
            Err(e) => internal_error_response(e),
        },
        "win_rate" | "streak" => {
            match crate::db::get_leaderboard_extended(db, rank_by, period, None, limit, offset)
                .await
            {
                Ok(entries) => {
                    let response = json!({
                        "leaderboard": entries,
                        "rank_by": rank_by,
                        "period": period,
                        "limit": limit,
                        "offset": offset,
                    });
                    ApiResponse::success(response).into_response()
                }
                Err(e) => internal_error_response(e),
            }
        }
        _ if period != "all" => {
            // Volume ranking scoped to a time window.
            match crate::db::get_leaderboard_extended(db, "volume", period, None, limit, offset)
                .await
            {
                Ok(entries) => {
                    let response = json!({
                        "leaderboard": entries,
                        "rank_by": "volume",
                        "period": period,
                        "limit": limit,
                        "offset": offset,
                    });
                    ApiResponse::success(response).into_response()
                }
                Err(e) => internal_error_response(e),
            }
        }
        _ => {
            // Default: all-time volume ranking (unchanged legacy behavior).
            match crate::db::get_users_by_betting_volume(db, limit, offset).await {
                Ok(users) => {
                    let response = json!({
                        "leaderboard": users,
                        "rank_by": "volume",
                        "period": "all",
                        "limit": limit,
                        "offset": offset,
                    });
                    ApiResponse::success(response).into_response()
                }
                Err(e) => internal_error_response(e),
            }
        }
    }
}

/// Query parameters for the `GET /api/v1/pools/:id/leaderboard` endpoint.
#[derive(Debug, Deserialize)]
pub struct PoolLeaderboardQuery {
    /// Maximum number of results to return (capped at 100, default 20).
    pub limit: Option<BoundedI64<1, 100>>,
    /// Zero-based offset for pagination (default 0).
    pub offset: Option<BoundedI64<0, 9223372036854775807>>,
    /// Ranking type: "volume" | "win_rate" | "streak" (default: "volume")
    pub rank_by: Option<NonEmptyString>,
}

/// `GET /api/v1/pools/:id/leaderboard` — top stakers for a single pool.
pub async fn get_pool_leaderboard(
    State(state): State<AppState>,
    Path(pool_id): Path<i64>,
    Query(params): Query<PoolLeaderboardQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let limit = params.limit.map(|b| b.get()).unwrap_or(20);
    let offset = params.offset.map(|b| b.get()).unwrap_or(0);
    let rank_by = params
        .rank_by
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("volume");

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::db::get_leaderboard_extended(db, rank_by, "all", Some(pool_id), limit, offset)
        .await
    {
        Ok(entries) => {
            let response = json!({
                "pool_id": pool_id,
                "leaderboard": entries,
                "rank_by": rank_by,
                "limit": limit,
                "offset": offset,
            });
            ApiResponse::success(response).into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

// ── Task 4: Pool Creation Indexer ─────────────────────────────────────────────

/// Request body for the pool-created event webhook / indexer endpoint.
///
/// The event listener decodes the XDR `pool_created` event from the Stellar
/// Horizon event stream and POSTs the decoded fields here.
#[derive(Debug, Deserialize)]
pub struct PoolCreatedPayload {
    pub pool_id: u64,
    pub creator: StellarAddress,
    pub end_time: u64,
    pub token: NonEmptyString,
    pub category: NonEmptyString,
    /// Pool description / name decoded from the event data.
    pub description: NonEmptyString,
    /// Number of distinct outcome options (#1932). Defaults to `2` (binary)
    /// so older callers that predate multi-outcome pools still deserialize.
    #[serde(default = "default_options_count")]
    pub options_count: u32,
    /// Human-readable label for each outcome index (#1932), e.g.
    /// `["Yes", "No"]`. Defaults to empty for the same backward-compat reason.
    #[serde(default)]
    pub outcome_descriptions: Vec<String>,
}

fn default_options_count() -> u32 {
    2
}

/// `POST /api/v1/indexer/pool-created` — ingest a decoded `PoolCreated` event
/// and persist the new pool to the database.
pub async fn ingest_pool_created(
    State(state): State<AppState>,
    Json(payload): Json<PoolCreatedPayload>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    let event = PoolCreatedEvent {
        pool_id: payload.pool_id,
        creator: payload.creator.into(),
        end_time: payload.end_time,
        token: payload.token.into(),
        category: payload.category.into(),
        description: payload.description.into(),
        options_count: payload.options_count as i32,
        outcome_descriptions: payload.outcome_descriptions,
    };

    match crate::db::insert_pool_from_event(db, &event).await {
        Ok(()) => {
            state.redis.invalidate_pools_cache().await;
            state.redis.invalidate_stats_cache().await;
            let response = json!({ "status": "ok", "pool_id": event.pool_id });
            ApiResponse::success(response).into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

/// Request body for the prediction-placed event webhook.
#[derive(Debug, Deserialize)]
pub struct PredictionPlacedPayload {
    pub pool_id: u64,
    pub user_address: StellarAddress,
    pub outcome: i32,
    pub amount: i64,
}

/// `POST /api/v1/indexer/prediction-placed` — ingest a decoded prediction event and update DB state.
pub async fn ingest_prediction_placed(
    State(state): State<AppState>,
    Json(payload): Json<PredictionPlacedPayload>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    let event = PredictionPlacedEvent {
        pool_id: payload.pool_id,
        user_address: payload.user_address.into(),
        amount: payload.amount,
        outcome: payload.outcome,
    };

    match crate::db::insert_prediction_from_event_with_pool(db, &event).await {
        Ok(()) => {
            state.event_bus.send(&json!({
                "type": "prediction_placed",
                "pool_id": event.pool_id,
                "user_address": event.user_address,
                "outcome": event.outcome,
                "amount": event.amount,
            }));
            state.redis.invalidate_stats_cache().await;
            state
                .redis
                .invalidate_user_predictions_cache(&event.user_address)
                .await;
            let response = json!({ "status": "ok", "pool_id": event.pool_id });
            ApiResponse::success(response).into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

// ── Market Predictions (cursor-based) ────────────────────────────────────────

/// Query parameters for `GET /api/v1/markets/:id/predictions`.
#[derive(Debug, Deserialize)]
pub struct MarketPredictionsQuery {
    /// Opaque cursor: the `id` of the last item on the previous page.
    /// Omit (or set to null) to start from the most recent prediction.
    pub after: Option<i64>,
    /// Page size (1–100, default 20).
    pub limit: Option<i64>,
}

/// `GET /api/v1/markets/:id/predictions` — cursor-paginated list of predictions
/// for a specific market (pool).
///
/// ## Pagination
/// The response includes `next_cursor`: pass it as `?after=<cursor>` on the
/// next request to fetch the following page. A `null` `next_cursor` means you
/// have reached the last page.
///
/// ## 404 behaviour
/// If the pool does not exist the handler returns 404 so callers can
/// distinguish "market not found" from "market has no predictions".
pub async fn get_market_predictions(
    State(state): State<AppState>,
    Path(market_id): Path<i64>,
    Query(params): Query<MarketPredictionsQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    // Clamp limit: 1..=100, default 20.
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let after = params.after;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    // Verify the market exists first so we can return 404 instead of an empty list.
    match crate::db::get_pool_by_id(db, market_id).await {
        Ok(None) => {
            return ApiResponse::<()>::error(
                StatusCode::NOT_FOUND,
                error_codes::NOT_FOUND,
                "market not found",
            )
            .into_response();
        }
        Err(e) => {
            return internal_error_response(e);
        }
        Ok(Some(_)) => {}
    }

    match tokio::try_join!(
        crate::db::get_market_predictions(db, market_id, after, limit),
        crate::db::count_market_predictions(db, market_id),
    ) {
        Ok((mut rows, total)) => {
            // The query fetches limit+1 rows; if we got the extra one there is
            // another page available.
            let has_next = rows.len() as i64 > limit;
            if has_next {
                rows.truncate(limit as usize);
            }
            let next_cursor: Option<i64> = if has_next {
                rows.last().map(|r| r.id)
            } else {
                None
            };

            let response = serde_json::json!({
                "market_id": market_id,
                "predictions": rows,
                "total": total,
                "limit": limit,
                "next_cursor": next_cursor,
            });
            ApiResponse::success(response).into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

// ── Pool creator incentive system (#1366) ─────────────────────────────────────

/// `GET /api/v1/creators/:address/stats` — reputation/quality metrics for a pool creator.
///
/// Returns the standard error envelope with 404 if the creator has no stats
/// row (distinct from a genuine query failure, which is a 500).
pub async fn get_creator_stats_handler(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::db::get_creator_stats(db, address.as_str()).await {
        Ok(Some(stats)) => ApiResponse::success(stats).into_response(),
        Ok(None) => ApiResponse::<()>::error(
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            "creator not found",
        )
        .into_response(),
        Err(e) => internal_error_response(e),
    }
}

/// `POST /api/v1/pools/:id/pay-creator-incentive` — pay out a pool creator's
/// incentive share once the pool has reached its participation threshold.
/// Idempotent: a second call after payout is a no-op.
pub async fn pay_creator_incentive_handler(
    State(state): State<AppState>,
    Path(pool_id): Path<i64>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::db::pay_creator_incentive(db, pool_id, state.config.treasury_fee_bps).await {
        Ok(Some(amount)) => {
            state.pool_cache.invalidate(pool_id);
            (
                StatusCode::OK,
                Json(json!({ "status": "paid", "pool_id": pool_id, "amount": amount })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::OK,
            Json(json!({ "status": "not_eligible", "pool_id": pool_id })),
        )
            .into_response(),
        Err(e) => crate::errors::AppError::from(e).into_response(),
    }
}

// ── Pool templates for recurring markets (#1368) ──────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreatePoolTemplateRequest {
    pub creator: StellarAddress,
    pub name: String,
    pub category: String,
    pub description: String,
    pub token: String,
    pub duration_seconds: i64,
    pub recurrence_interval_seconds: i64,
}

/// `POST /api/v1/pool-templates` — save a reusable pool configuration.
pub async fn create_pool_template_handler(
    State(state): State<AppState>,
    Json(body): Json<CreatePoolTemplateRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::db::create_pool_template(
        db,
        body.creator.as_str(),
        &body.name,
        &body.category,
        &body.description,
        &body.token,
        body.duration_seconds,
        body.recurrence_interval_seconds,
    )
    .await
    {
        Ok(template) => (StatusCode::OK, Json(json!(template))).into_response(),
        Err(e) => crate::errors::AppError::from(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListPoolTemplatesQuery {
    pub creator: StellarAddress,
}

/// `GET /api/v1/pool-templates?creator=...` — list a creator's saved templates.
pub async fn list_pool_templates_handler(
    State(state): State<AppState>,
    Query(params): Query<ListPoolTemplatesQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::db::list_pool_templates(db, params.creator.as_str()).await {
        Ok(templates) => (
            StatusCode::OK,
            Json(json!({ "creator": params.creator, "templates": templates })),
        )
            .into_response(),
        Err(e) => crate::errors::AppError::from(e).into_response(),
    }
}

// ── User profile ───────────────────────────────────────────────────────────────

/// `GET /api/v1/users/:address/profile` — aggregated profile: win/loss stats,
/// claim status per pool, and a daily activity series for performance charts.
pub async fn get_user_profile_handler(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::profile::get_full_profile(db, address.as_str()).await {
        Ok(profile) => ApiResponse::success(profile).into_response(),
        Err(e) => internal_error_response(e),
    }
}

// ── Pool tags ────────────────────────────────────────────────────────────────

/// `GET /api/v1/tags` — distinct tags currently in use, for filter-UI dropdowns.
pub async fn list_tags_handler(State(state): State<AppState>) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::tags::list_distinct_tags(db).await {
        Ok(tags) => ApiResponse::success(json!({ "tags": tags })).into_response(),
        Err(e) => internal_error_response(e),
    }
}

/// Request body for `PATCH /api/v1/pools/:id/tags`.
#[derive(Debug, Deserialize)]
pub struct UpdatePoolTagsRequest {
    /// Must match the pool's on-chain creator — only the creator may retag a pool.
    pub creator: StellarAddress,
    pub tags: Vec<String>,
}

/// `PATCH /api/v1/pools/:id/tags` — creator-only: replace a pool's tag set.
pub async fn update_pool_tags_handler(
    State(state): State<AppState>,
    Path(pool_id): Path<i64>,
    Json(body): Json<UpdatePoolTagsRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::tags::update_pool_tags(db, pool_id, body.creator.as_str(), &body.tags).await {
        Ok(true) => {
            state.pool_cache.invalidate(pool_id);
            state.redis.invalidate_pools_cache().await;
            ApiResponse::success(json!({ "pool_id": pool_id, "tags": body.tags })).into_response()
        }
        Ok(false) => ApiResponse::<()>::error(
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            "pool not found, or caller is not the creator",
        )
        .into_response(),
        Err(e) => internal_error_response(e),
    }
}

// ── Claims (winnings / refunds) ───────────────────────────────────────────────

/// Request body for the claim-event webhook / indexer endpoint.
#[derive(Debug, Deserialize)]
pub struct ClaimPayload {
    pub pool_id: u64,
    pub user_address: StellarAddress,
    /// Amount paid out by `claim_winnings` / `claim_refund` (0 for losers).
    pub amount_paid: i64,
}

/// `POST /api/v1/indexer/claim` — ingest a decoded `WinningsClaimed` /
/// `RefundClaimed` contract event and mark the user's predictions as claimed.
pub async fn ingest_claim_handler(
    State(state): State<AppState>,
    Json(payload): Json<ClaimPayload>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::profile::mark_predictions_claimed(
        db,
        payload.pool_id as i64,
        payload.user_address.as_str(),
        payload.amount_paid,
    )
    .await
    {
        Ok(rows) => {
            ApiResponse::success(json!({ "status": "ok", "rows_updated": rows })).into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

// ── Notifications ──────────────────────────────────────────────────────────────

/// Query parameters for `GET /api/v1/notifications/:address`.
#[derive(Debug, Deserialize)]
pub struct NotificationsQuery {
    pub unread_only: Option<bool>,
    pub limit: Option<BoundedI64<1, 100>>,
    pub offset: Option<BoundedI64<0, 9223372036854775807>>,
}

/// `GET /api/v1/notifications/:address` — list a user's notifications, newest first.
pub async fn list_notifications_handler(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
    Query(params): Query<NotificationsQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    let unread_only = params.unread_only.unwrap_or(false);
    let limit = params.limit.map(|b| b.get()).unwrap_or(20);
    let offset = params.offset.map(|b| b.get()).unwrap_or(0);

    match tokio::try_join!(
        crate::notifications::list_notifications(db, address.as_str(), unread_only, limit, offset),
        crate::notifications::count_unread(db, address.as_str())
    ) {
        Ok((notifications, unread_count)) => ApiResponse::success(json!({
            "address": address,
            "notifications": notifications,
            "unread_count": unread_count,
            "limit": limit,
            "offset": offset,
        }))
        .into_response(),
        Err(e) => internal_error_response(e),
    }
}

/// Request body for `POST /api/v1/notifications/:address/read`.
#[derive(Debug, Deserialize)]
pub struct MarkNotificationsReadRequest {
    /// Specific notification IDs to mark read. Omit (or leave empty) to mark all as read.
    #[serde(default)]
    pub ids: Vec<i64>,
}

/// `POST /api/v1/notifications/:address/read` — mark notifications as read.
pub async fn mark_notifications_read_handler(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
    Json(body): Json<MarkNotificationsReadRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    let ids = if body.ids.is_empty() {
        None
    } else {
        Some(body.ids.as_slice())
    };

    match crate::notifications::mark_read(db, address.as_str(), ids).await {
        Ok(updated) => ApiResponse::success(json!({ "updated": updated })).into_response(),
        Err(e) => internal_error_response(e),
    }
}

/// `GET /api/v1/users/:address/interests` — categories/tags a user follows
/// for "new pool matching your interests" alerts.
pub async fn get_user_interests_handler(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::notifications::get_user_interests(db, address.as_str()).await {
        Ok(interests) => {
            ApiResponse::success(json!({ "address": address, "interests": interests }))
                .into_response()
        }
        Err(e) => internal_error_response(e),
    }
}

/// `PUT /api/v1/users/:address/interests` — replace a user's interest list wholesale.
pub async fn set_user_interests_handler(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
    Json(body): Json<crate::notifications::SetInterestsRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::notifications::set_user_interests(db, address.as_str(), &body.interests).await {
        Ok(()) => ApiResponse::success(json!({ "address": address, "interests": body.interests }))
            .into_response(),
        Err(e) => internal_error_response(e),
    }
}

/// Build the version 1 API router.
pub fn router(
    config: Arc<Config>,
    cache: PriceCache,
    redis: RedisCache,
    pool: Option<sqlx::PgPool>,
    metrics: SharedMetrics,
    event_bus: crate::ws::EventBus,
) -> Router {
    use crate::rate_limit::{with_rate_limit, RateLimitTier};

    let pool_cache = PoolCache::with_negative_ttl(std::time::Duration::from_secs(
        config.pool_negative_cache_ttl_secs,
    ));

    let state = AppState {
        config,
        cache,
        pool_cache,
        redis,
        db: pool,
        metrics,
        event_bus,
    };

    // Admin tier — authenticated administrative operations (no public rate limit).
    let admin = Router::new()
        .route("/admin/log-level", patch(set_log_level))
        .with_state(state.clone());

    // Light tier — cheap, stateless, polling-friendly endpoints.
    let light = with_rate_limit(
        Router::new()
            .route("/", get(index))
            .route("/health", get(health))
            .route("/fees", get(get_fees))
            .route("/prices", get(crate::price_cache::get_prices))
            .with_state(state.clone()),
        RateLimitTier::Light,
    );

    // Token tier — refresh rotation and WebSocket handshake (10 req / 60 s).
    let token = with_rate_limit(
        Router::new()
            .route("/auth/refresh", post(refresh_tokens_handler))
            .route("/ws", get(crate::ws::ws_handler))
            .with_state(state.clone()),
        RateLimitTier::Token,
    );

    // Read tier — public, database-backed read endpoints.
    let read = with_rate_limit(
        Router::new()
            .route("/pools", get(get_pools))
            .route("/pools/:id", get(get_pool_by_id_handler))
            .route("/pools/:id/leaderboard", get(get_pool_leaderboard))
            .route("/stats", get(get_stats))
            .route("/leaderboard", get(get_leaderboard))
            .route("/tags", get(list_tags_handler))
            .route("/referrals/:address", get(referrals_handler))
            .route(
                "/referrals/:address/estimate",
                get(referral_estimate_handler),
            )
            .route("/markets/:id/predictions", get(get_market_predictions))
            .with_state(state.clone()),
        RateLimitTier::Read,
    );

    // User tier — per-user history and predictions.
    let user = with_rate_limit(
        Router::new()
            .route("/users/:address/history", get(get_user_history))
            .route("/users/:address/predictions", get(get_user_predictions))
            .route("/users/:address/profile", get(get_user_profile_handler))
            .route(
                "/users/:address/referrals",
                get(user_referral_earnings_handler),
            )
            .route(
                "/users/:address/interests",
                get(get_user_interests_handler).put(set_user_interests_handler),
            )
            .route(
                "/users/:address/email",
                get(get_user_email_handler).put(upsert_user_email_handler),
            )
            .route("/notifications/:address", get(list_notifications_handler))
            .route(
                "/notifications/:address/read",
                post(mark_notifications_read_handler),
            )
            .with_state(state.clone()),
        RateLimitTier::User,
    );

    // Write tier — indexer ingest (typically internal, strictest limit).
    let write = with_rate_limit(
        Router::new()
            .route("/indexer/pool-created", post(ingest_pool_created))
            .route("/indexer/prediction-placed", post(ingest_prediction_placed))
            .route("/indexer/claim", post(ingest_claim_handler))
            .route("/pools/:id/tags", patch(update_pool_tags_handler))
            .with_state(state.clone()),
        RateLimitTier::Write,
    );

    // Routes without a rate-limit tier of their own. Anything already served by
    // `light`/`read`/`user`/`write`/`token` above must NOT be repeated here: `merge`
    // panics on an overlapping method route, which would take down the whole
    // server at startup.
    Router::new()
        .route("/creators/:address/stats", get(get_creator_stats_handler))
        .route(
            "/pools/:id/pay-creator-incentive",
            post(pay_creator_incentive_handler),
        )
        .route(
            "/pool-templates",
            get(list_pool_templates_handler).post(create_pool_template_handler),
        )
        .with_state(state)
        .merge(light)
        .merge(token)
        .merge(read)
        .merge(user)
        .merge(write)
        .merge(admin)
}

/// `POST /api/v1/auth/refresh` — rotate a refresh token and issue a new pair.
///
/// Rate-limited at the Token tier (10 req / 60 s per IP). Fails closed when
/// Redis is unavailable so a stolen refresh token cannot be replayed after
/// rotation without a durable revocation record.
async fn refresh_tokens_handler(
    State(state): State<AppState>,
    Json(body): Json<RefreshTokenRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use std::time::{SystemTime, UNIX_EPOCH};

    if body.refresh_token.is_empty() {
        return ApiResponse::<()>::error(
            StatusCode::BAD_REQUEST,
            error_codes::INVALID_INPUT,
            "refresh_token is required",
        )
        .into_response();
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match crate::jwt_security::rotate_refresh_token(
        &state.redis,
        &state.config.secret_key,
        &body.refresh_token,
        state.config.jwt_key_version,
        now,
    )
    .await
    {
        Ok(pair) => {
            let response = RefreshTokenResponse {
                access_token: pair.access_token,
                refresh_token: pair.refresh_token,
                token_type: "Bearer".to_string(),
                expires_in: crate::jwt_security::access_token_expires_in(),
            };
            ApiResponse::success(response).into_response()
        }
        Err(crate::jwt::JwtVerifyError::TokenStoreUnavailable) => ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::SERVICE_UNAVAILABLE,
            "token store unavailable",
        )
        .into_response(),
        Err(error) => ApiResponse::<()>::error(
            StatusCode::UNAUTHORIZED,
            error_codes::UNAUTHORIZED,
            error.to_string(),
        )
        .into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct RefreshTokenRequest {
    refresh_token: String,
}

#[derive(Debug, Serialize)]
struct RefreshTokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expires_in: u64,
}
async fn get_fees(State(state): State<AppState>) -> Json<FeeInfo> {
    Json(FeeInfo {
        treasury_fee_bps: state.config.treasury_fee_bps,
        referral_fee_bps: state.config.referral_fee_bps,
    })
}

/// `GET /api/v1/referrals/:address` — delegates to the referrals module.
///
/// Requires a live DB pool; returns 503 if the pool is not configured.
async fn referrals_handler(
    axum::extract::Path(address): axum::extract::Path<StellarAddress>,
    State(state): State<AppState>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let address_str = address.as_str().to_string();

    match state.db {
        Some(pool) => {
            match crate::referrals::get_referrals(axum::extract::Path(address_str), State(pool))
                .await
            {
                Ok((status, body)) => (status, body).into_response(),
                Err(e) => internal_error_response(e),
            }
        }
        None => ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not configured",
        )
        .into_response(),
    }
}

/// `GET /api/v1/referrals/:address/estimate` — estimated referral reward.
///
/// Requires a live DB pool; returns 503 if the pool is not configured.
async fn referral_estimate_handler(
    axum::extract::Path(address): axum::extract::Path<StellarAddress>,
    State(state): State<AppState>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let address_str = address.as_str().to_string();

    match state.db {
        Some(pool) => {
            match crate::referrals::estimate_referral_rewards(
                address_str,
                &pool,
                state.config.treasury_fee_bps,
                state.config.referral_fee_bps,
            )
            .await
            {
                Ok((status, body)) => (status, body).into_response(),
                Err(e) => internal_error_response(e),
            }
        }
        None => ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not configured",
        )
        .into_response(),
    }
}

/// `GET /api/v1/users/:address/referrals` — per-pool referral earnings for a user.
async fn user_referral_earnings_handler(
    axum::extract::Path(address): axum::extract::Path<StellarAddress>,
    State(state): State<AppState>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let address_str = address.as_str().to_string();

    match state.db {
        Some(pool) => {
            match crate::referrals::get_user_referral_earnings(
                axum::extract::Path(address_str),
                State(pool),
            )
            .await
            {
                Ok((status, body)) => (status, body).into_response(),
                Err(e) => internal_error_response(e),
            }
        }
        None => ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not configured",
        )
        .into_response(),
    }
}

#[cfg(test)]
mod cache_aside_tests {
    use crate::redis_cache::{pools_cache_key, POOLS_CACHE_TTL};

    /// Test the cache-aside pattern: cache miss, database fetch, and cache population
    #[tokio::test]
    async fn test_get_pools_cache_aside_pattern() {
        let cache = crate::redis_cache::RedisCache::disabled();
        let cache_key = pools_cache_key("new", None, "active", 20, 0);

        // Step 1: Cache is empty (miss)
        let cached: Option<serde_json::Value> = cache.get(&cache_key).await;
        assert!(cached.is_none());

        // Step 2: Simulate database result
        let test_response = serde_json::json!({
            "pools": [],
            "total": 0,
            "limit": 20,
            "offset": 0,
            "status": "active",
            "category": serde_json::Value::Null,
            "sort_by": "new"
        });

        // Step 3: Cache the result (as done in get_pools handler)
        cache.set(&cache_key, &test_response, POOLS_CACHE_TTL).await;

        // Step 4: Verify cache no-op with disabled cache (expected behavior)
        let cached: Option<serde_json::Value> = cache.get(&cache_key).await;
        assert!(cached.is_none());
    }

    /// Test cache key generation for different query parameters
    #[test]
    fn test_cache_key_uniqueness_for_pool_queries() {
        // Same base query, different sort_by -> different keys
        let key_new = pools_cache_key("new", None, "active", 20, 0);
        let key_popular = pools_cache_key("popular", None, "active", 20, 0);
        let key_ending = pools_cache_key("ending_soon", None, "active", 20, 0);

        assert_ne!(key_new, key_popular);
        assert_ne!(key_new, key_ending);
        assert_ne!(key_popular, key_ending);

        // Same base query, different category -> different keys
        let key_no_category = pools_cache_key("new", None, "active", 20, 0);
        let key_sports = pools_cache_key("new", Some("sports"), "active", 20, 0);
        let key_finance = pools_cache_key("new", Some("finance"), "active", 20, 0);

        assert_ne!(key_no_category, key_sports);
        assert_ne!(key_sports, key_finance);

        // Same base query, different status -> different keys
        let key_active = pools_cache_key("new", None, "active", 20, 0);
        let key_closed = pools_cache_key("new", None, "closed", 20, 0);
        let key_settled = pools_cache_key("new", None, "settled", 20, 0);

        assert_ne!(key_active, key_closed);
        assert_ne!(key_closed, key_settled);

        // Same base query, different pagination -> different keys
        let key_page1 = pools_cache_key("new", None, "active", 20, 0);
        let key_page2 = pools_cache_key("new", None, "active", 20, 20);
        let key_limit50 = pools_cache_key("new", None, "active", 50, 0);

        assert_ne!(key_page1, key_page2);
        assert_ne!(key_page1, key_limit50);
    }

    /// Test cache invalidation pattern
    #[tokio::test]
    async fn test_cache_invalidation_on_new_pool() {
        let cache = crate::redis_cache::RedisCache::disabled();

        // Simulate setting multiple pool cache entries
        let keys = vec![
            pools_cache_key("new", None, "active", 20, 0),
            pools_cache_key("popular", None, "active", 20, 0),
            pools_cache_key("new", Some("sports"), "active", 20, 0),
        ];

        for key in &keys {
            let data = serde_json::json!({"test": "data"});
            cache.set(key, &data, POOLS_CACHE_TTL).await;
        }

        // Invalidate all pool caches (as done when new pool is created)
        cache.invalidate_pools_cache().await;

        // All keys should be invalidated (with disabled cache, get always returns None anyway)
        for key in &keys {
            let cached: Option<serde_json::Value> = cache.get(key).await;
            assert!(cached.is_none());
        }
    }

    /// Test cache behavior with different query parameter combinations
    #[test]
    fn test_cache_key_encoding_with_all_parameters() {
        // Test with all parameters specified
        let full_key = pools_cache_key("popular", Some("sports"), "closed", 50, 100);
        assert_eq!(full_key, "pools:popular:sports:closed:50:100");

        // Test with None category encodes as "all"
        let no_category = pools_cache_key("new", None, "active", 20, 0);
        assert_eq!(no_category, "pools:new:all:active:20:0");

        // Test various sort_by values
        let sort_new = pools_cache_key("new", None, "active", 20, 0);
        let sort_popular = pools_cache_key("popular", None, "active", 20, 0);
        let sort_ending = pools_cache_key("ending_soon", None, "active", 20, 0);

        assert_eq!(sort_new, "pools:new:all:active:20:0");
        assert_eq!(sort_popular, "pools:popular:all:active:20:0");
        assert_eq!(sort_ending, "pools:ending_soon:all:active:20:0");
    }

    /// Test stats cache-aside pattern: miss, populate, verify no-op on disabled cache.
    #[tokio::test]
    async fn test_stats_cache_aside_pattern() {
        use crate::redis_cache::{stats_cache_key, RedisCache, STATS_CACHE_TTL};

        let cache = RedisCache::disabled();
        let key = stats_cache_key(None, None);

        // Cache miss
        let cached: Option<serde_json::Value> = cache.get(&key).await;
        assert!(cached.is_none());

        // Populate (no-ops on disabled cache)
        let data = serde_json::json!({
            "total_value_locked": 1000,
            "total_bets": 50,
            "total_pools": 10
        });
        cache.set(&key, &data, STATS_CACHE_TTL).await;

        // Still None — disabled cache always returns None
        let cached: Option<serde_json::Value> = cache.get(&key).await;
        assert!(cached.is_none());
    }

    /// Test that stats cache keys are unique per filter combination.
    #[test]
    fn test_stats_cache_key_uniqueness() {
        use crate::redis_cache::stats_cache_key;

        let k1 = stats_cache_key(None, None);
        let k2 = stats_cache_key(Some("sports"), None);
        let k3 = stats_cache_key(None, Some("active"));
        let k4 = stats_cache_key(Some("crypto"), Some("closed"));

        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
        assert_ne!(k1, k4);
        assert_ne!(k2, k3);
        assert_ne!(k2, k4);
    }

    /// Test stats cache invalidation no-ops on a disabled cache.
    #[tokio::test]
    async fn test_stats_cache_invalidation_on_new_data() {
        use crate::redis_cache::RedisCache;

        let cache = RedisCache::disabled();
        // Should not panic
        cache.invalidate_stats_cache().await;
    }

    /// Test the `get_user_predictions` cache-aside pattern added for #1938:
    /// miss, populate, and invalidation all behave the same way as the
    /// `stats`/`pools` cache-aside patterns above.
    #[tokio::test]
    async fn test_user_predictions_cache_aside_pattern() {
        use crate::redis_cache::{
            user_predictions_cache_key, RedisCache, USER_PREDICTIONS_CACHE_TTL,
        };

        let cache = RedisCache::disabled();
        let key = user_predictions_cache_key("GABC123", 20, 0);

        // Cache miss
        let cached: Option<serde_json::Value> = cache.get(&key).await;
        assert!(cached.is_none());

        // Populate (no-ops on disabled cache, same as the other endpoints)
        let data = serde_json::json!({
            "address": "GABC123",
            "predictions": [],
            "limit": 20,
            "offset": 0,
            "total_predictions": 0
        });
        cache.set(&key, &data, USER_PREDICTIONS_CACHE_TTL).await;

        let cached: Option<serde_json::Value> = cache.get(&key).await;
        assert!(cached.is_none());

        // Invalidation must not panic either.
        cache.invalidate_user_predictions_cache("GABC123").await;
    }

    /// Different (address, limit, offset) triples must produce distinct
    /// cache keys so one user's cached page can't be served to another.
    #[test]
    fn test_user_predictions_cache_key_uniqueness() {
        use crate::redis_cache::user_predictions_cache_key;

        let k1 = user_predictions_cache_key("GABC123", 20, 0);
        let k2 = user_predictions_cache_key("GXYZ789", 20, 0);
        let k3 = user_predictions_cache_key("GABC123", 10, 0);
        let k4 = user_predictions_cache_key("GABC123", 20, 20);

        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
        assert_ne!(k1, k4);
    }
}

/// Tests for `GET /api/v1/pools` page-size resolution (#1734): missing
/// `limit` uses the documented default, and an out-of-range `limit` is
/// clamped into bounds rather than rejected with 400.
#[cfg(test)]
mod pools_pagination_tests {
    use super::{resolve_pools_page_size, POOLS_DEFAULT_PAGE_SIZE, POOLS_MAX_PAGE_SIZE};

    #[test]
    fn missing_limit_uses_documented_default() {
        assert_eq!(resolve_pools_page_size(None), POOLS_DEFAULT_PAGE_SIZE);
    }

    #[test]
    fn limit_within_bounds_is_unchanged() {
        assert_eq!(resolve_pools_page_size(Some(1)), 1);
        assert_eq!(resolve_pools_page_size(Some(50)), 50);
        assert_eq!(
            resolve_pools_page_size(Some(POOLS_MAX_PAGE_SIZE)),
            POOLS_MAX_PAGE_SIZE
        );
    }

    #[test]
    fn limit_above_max_is_clamped_not_rejected() {
        assert_eq!(
            resolve_pools_page_size(Some(POOLS_MAX_PAGE_SIZE + 1)),
            POOLS_MAX_PAGE_SIZE
        );
        assert_eq!(
            resolve_pools_page_size(Some(1_000_000)),
            POOLS_MAX_PAGE_SIZE
        );
    }

    #[test]
    fn non_positive_limit_is_clamped_up_to_one() {
        assert_eq!(resolve_pools_page_size(Some(0)), 1);
        assert_eq!(resolve_pools_page_size(Some(-5)), 1);
    }
}

// ── User email registration (#1929) ──────────────────────────────────────────

/// `GET /api/v1/users/:address/email` — retrieve the registered email and
/// opt-in state for a user.
pub async fn get_user_email_handler(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    match crate::notifications::get_user_email(db, address.as_str()).await {
        Ok(Some((email, enabled))) => ApiResponse::success(json!({
            "address": address,
            "email": email,
            "email_enabled": enabled,
        }))
        .into_response(),
        Ok(None) => ApiResponse::<()>::error(
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            "no email registered for this address",
        )
        .into_response(),
        Err(e) => internal_error_response(e),
    }
}

/// `PUT /api/v1/users/:address/email` — register or update a user's email address.
///
/// Body: `{ "email": "user@example.com", "email_enabled": true }`
pub async fn upsert_user_email_handler(
    State(state): State<AppState>,
    Path(address): Path<StellarAddress>,
    Json(body): Json<crate::notifications::RegisterEmailRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let Some(db) = &state.db else {
        return ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::DATABASE_UNAVAILABLE,
            "database not available",
        )
        .into_response();
    };

    let email = body.email.trim();
    if email.is_empty() || !email.contains('@') {
        return ApiResponse::<()>::error(
            StatusCode::BAD_REQUEST,
            error_codes::INVALID_INPUT,
            "a valid email address is required",
        )
        .into_response();
    }

    let enabled = body.email_enabled.unwrap_or(true);

    match crate::notifications::upsert_user_email(db, address.as_str(), email, enabled).await {
        Ok(()) => ApiResponse::success(json!({
            "address": address,
            "email": email,
            "email_enabled": enabled,
        }))
        .into_response(),
        Err(e) => internal_error_response(e),
    }
}
