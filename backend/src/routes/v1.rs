use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration as TokioDuration};

use crate::config::Config;
use crate::db::{PoolCreatedEvent, PredictionPlacedEvent};
use crate::metrics::SharedMetrics;
use crate::price_cache::PriceCache;
use crate::redis_cache::RedisCache;
use crate::response::{error_codes, ApiResponse};
use crate::validated_types::{BoundedI64, NonEmptyString, PoolSortBy, PoolStatus, StellarAddress};

/// Struct representing fee information, matching the contract structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeeInfo {
    /// Protocol (treasury) fee in basis points.
    pub treasury_fee_bps: u32,
    /// Referral share of the protocol fee in basis points.
    pub referral_fee_bps: u32,
}

/// `GET /api/v1/health` health-check endpoint.
pub async fn health(State(state): State<AppState>) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use std::time::Duration;

    let mut all_healthy = true;
    let mut db_status = "ok";
    let mut db_error = String::new();

    if let Some(db) = &state.db {
        if let Err(e) = sqlx::query("SELECT 1").execute(db).await {
            db_status = "unreachable";
            db_error = e.to_string();
            all_healthy = false;
        }
    } else {
        db_status = "not_configured";
    }

    let mut rpc_status = "ok";
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(state.config.rpc_health_timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Try RPC health check with retry logic
    let mut rpc_attempts = 0;
    let max_attempts = state.config.rpc_health_retry_count as usize;
    let mut last_error = String::new();

    while rpc_attempts < max_attempts {
        rpc_attempts += 1;

        let rpc_req = client
            .post(&state.config.stellar_rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getHealth"
            }))
            .send()
            .await;

        match rpc_req {
            Ok(res) if res.status().is_success() => {
                // Success - break out of retry loop
                break;
            }
            Ok(res) => {
                last_error = format!("HTTP {} response", res.status());
            }
            Err(e) => {
                last_error = e.to_string();
            }
        }

        // Exponential backoff: 2^(attempt-1) seconds, capped at 5 seconds
        if rpc_attempts < max_attempts {
            let backoff_duration = std::cmp::min(2u64.pow((rpc_attempts - 1) as u32), 5);
            sleep(TokioDuration::from_secs(backoff_duration)).await;
        }
    }

    if rpc_attempts >= max_attempts {
        rpc_status = "unreachable";
        all_healthy = false;
    }

    let mut redis_status = "ok";
    let mut redis_error = String::new();
    if !state.redis.is_available() {
        redis_status = "not_configured";
        all_healthy = false;
    } else if !state.redis.ping().await {
        redis_status = "unreachable";
        redis_error = String::from("Redis ping failed");
        all_healthy = false;
    }

    let mut price_cache_status = "ok";
    if state.cache.snapshot().is_empty() {
        price_cache_status = "not_ready";
        all_healthy = false;
    }

    let body = serde_json::json!({
        "status": if all_healthy { "ok" } else { "error" },
        "version": "v1",
        "dependencies": {
            "db": db_status,
            "rpc": rpc_status,
            "redis": redis_status,
            "price_cache": price_cache_status
        },
        "errors": {
            "db": if db_status == "unreachable" { Some(db_error.clone()) } else { None },
            "rpc": if rpc_status == "unreachable" { Some(last_error.clone()) } else { None },
            "redis": if redis_status == "unreachable" { Some(redis_error.clone()) } else { None },
            "price_cache": if price_cache_status == "not_ready" { Some("price cache is empty".to_string()) } else { None }
        }
    });

    if all_healthy {
        (StatusCode::OK, Json(body)).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
    }
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

// ── Task 2: Active Pools API ──────────────────────────────────────────────────

/// Query parameters for the `GET /api/v1/pools` endpoint.
#[derive(Debug, Deserialize)]
pub struct PoolsQuery {
    /// Sort order: "popular" | "ending_soon" | "new" (default)
    pub sort_by: Option<PoolSortBy>,
    /// Category filter, e.g. "Sports", "Crypto"
    pub category: Option<NonEmptyString>,
    /// Status filter: "active" | "closed" | "settled" (default: "active")
    pub status: Option<PoolStatus>,
    /// Page size, clamped to [1, 100].
    pub limit: Option<BoundedI64<1, 100>>,
    /// Zero-based page offset, minimum 0.
    pub offset: Option<BoundedI64<0, 9223372036854775807>>,
}

#[derive(Debug, Serialize)]
pub struct PoolsResponse {
    pub pools: Vec<crate::db::PoolRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub status: String,
    pub category: Option<String>,
    pub sort_by: String,
}

/// `GET /api/v1/pools/:id` — get detailed pool information with real-time odds.
pub async fn get_pool_by_id_handler(
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

    match crate::db::get_pool_with_odds(db, pool_id).await {
        Ok(Some(pool)) => ApiResponse::success(pool).into_response(),
        Ok(None) => ApiResponse::<()>::error(
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            "pool not found",
        )
        .into_response(),
        Err(e) => ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error_codes::INTERNAL_ERROR,
            e.to_string(),
        )
        .into_response(),
    }
}

/// Query parameters for the `GET /api/v1/stats` endpoint.
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// Category filter, e.g. "Sports", "Crypto"
    pub category: Option<String>,
    /// Pool state filter: "active" | "closed" | "settled"
    pub status: Option<String>,
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

    let cache_key =
        crate::redis_cache::stats_cache_key(params.category.as_deref(), params.status.as_deref());

    if let Some(cached) = state.redis.get::<serde_json::Value>(&cache_key).await {
        return (StatusCode::OK, Json(cached)).into_response();
    }

    match crate::db::get_protocol_stats(db, params.category.as_deref(), params.status.as_deref())
        .await
    {
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
            ApiResponse::success(stats).into_response()
        }
        Err(e) => ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error_codes::INTERNAL_ERROR,
            e.to_string(),
        )
        .into_response(),
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

    // Cache-aside pattern: Step 1 — Check cache
    let cache_key = crate::redis_cache::pools_cache_key(sort_by, category, status, limit, offset);

    if let Some(cached_response) = state.redis.get::<serde_json::Value>(&cache_key).await {
        // Cache hit — return cached data
        return (StatusCode::OK, Json(cached_response)).into_response();
    }

    // Cache miss — Step 2: Fetch from database
    match tokio::try_join!(
        crate::db::get_pools_with_filters(db, sort_by, category, status, limit, offset),
        crate::db::count_pools_with_filters(db, category, status)
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
        Err(e) => ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error_codes::INTERNAL_ERROR,
            e.to_string(),
        )
        .into_response(),
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
    Path(address): Path<String>,
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

    match crate::db::get_user_prediction_history(db, &address, limit, offset).await {
        Ok(rows) => {
            let response = json!({
                "address": address,
                "predictions": rows,
                "limit": limit,
                "offset": offset,
            });
            ApiResponse::success(response).into_response()
        }
        Err(e) => ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error_codes::INTERNAL_ERROR,
            e.to_string(),
        )
        .into_response(),
    }
}

/// `GET /api/v1/users/:address/predictions` — enhanced user predictions with current pool status.
pub async fn get_user_predictions(
    State(state): State<AppState>,
    Path(address): Path<String>,
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

    match crate::db::get_user_predictions(db, &address, limit, offset).await {
        Ok(predictions) => {
            let response = json!({
                "address": address,
                "predictions": predictions,
                "limit": limit,
                "offset": offset,
                "total_predictions": predictions.len(),
            });
            ApiResponse::success(response).into_response()
        }
        Err(e) => ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error_codes::INTERNAL_ERROR,
            e.to_string(),
        )
        .into_response(),
    }
}

/// Query parameters for the `GET /api/v1/leaderboard` endpoint.
#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    /// Maximum number of results to return (capped at 100, default 20).
    pub limit: Option<BoundedI64<1, 100>>,
    /// Zero-based offset for pagination (default 0).
    pub offset: Option<BoundedI64<0, 9223372036854775807>>,
    /// Ranking type: "volume" | "winnings" (default: "volume")
    pub rank_by: Option<String>,
}

/// `GET /api/v1/leaderboard` — user rankings by betting volume or winnings.
pub async fn get_leaderboard(
    State(state): State<AppState>,
    Query(params): Query<LeaderboardQuery>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let limit = params.limit.map(|b| b.get()).unwrap_or(20);
    let offset = params.offset.map(|b| b.get()).unwrap_or(0);
    let rank_by = params.rank_by.as_deref().unwrap_or("volume");

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
                    "limit": limit,
                    "offset": offset,
                });
                ApiResponse::success(response).into_response()
            }
            Err(e) => ApiResponse::<()>::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                error_codes::INTERNAL_ERROR,
                e.to_string(),
            )
            .into_response(),
        },
        _ => {
            // Default to volume ranking
            match crate::db::get_users_by_betting_volume(db, limit, offset).await {
                Ok(users) => {
                    let response = json!({
                        "leaderboard": users,
                        "rank_by": "volume",
                        "limit": limit,
                        "offset": offset,
                    });
                    ApiResponse::success(response).into_response()
                }
                Err(e) => ApiResponse::<()>::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error_codes::INTERNAL_ERROR,
                    e.to_string(),
                )
                .into_response(),
            }
        }
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
    };

    match crate::db::insert_pool_from_event(db, &event).await {
        Ok(()) => {
            state.redis.invalidate_pools_cache().await;
            state.redis.invalidate_stats_cache().await;
            let response = json!({ "status": "ok", "pool_id": event.pool_id });
            ApiResponse::success(response).into_response()
        }
        Err(e) => ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error_codes::INTERNAL_ERROR,
            e.to_string(),
        )
        .into_response(),
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
            let response = json!({ "status": "ok", "pool_id": event.pool_id });
            ApiResponse::success(response).into_response()
        }
        Err(e) => ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error_codes::INTERNAL_ERROR,
            e.to_string(),
        )
        .into_response(),
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
            return ApiResponse::<()>::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                error_codes::INTERNAL_ERROR,
                e.to_string(),
            )
            .into_response();
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
        Err(e) => ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error_codes::INTERNAL_ERROR,
            e.to_string(),
        )
        .into_response(),
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

    let state = AppState {
        config,
        cache,
        redis,
        db: pool,
        metrics,
        event_bus,
    };

    // Light tier — cheap, stateless, polling-friendly endpoints.
    let light = with_rate_limit(
        Router::new()
            .route("/", get(index))
            .route("/health", get(health))
            .route("/fees", get(get_fees))
            .route("/prices", get(crate::price_cache::get_prices))
            .route("/ws", get(crate::ws::ws_handler))
            .with_state(state.clone()),
        RateLimitTier::Light,
    );

    // Read tier — public, database-backed read endpoints.
    let read = with_rate_limit(
        Router::new()
            .route("/pools", get(get_pools))
            .route("/pools/:id", get(get_pool_by_id_handler))
            .route("/stats", get(get_stats))
            .route("/leaderboard", get(get_leaderboard))
            .route("/referrals/{address}", get(referrals_handler))
            .route(
                "/referrals/{address}/estimate",
                get(referral_estimate_handler),
            )
            .route("/markets/:id/predictions", get(get_market_predictions))
            .with_state(state.clone()),
        RateLimitTier::Read,
    );

    // User tier — per-user history and predictions.
    let user = with_rate_limit(
        Router::new()
            .route("/users/{address}/history", get(get_user_history))
            .route("/users/{address}/predictions", get(get_user_predictions))
            .route(
                "/users/{address}/referrals",
                get(user_referral_earnings_handler),
            )
            .with_state(state.clone()),
        RateLimitTier::User,
    );

    // Write tier — indexer ingest (typically internal, strictest limit).
    let write = with_rate_limit(
        Router::new()
            .route("/indexer/pool-created", post(ingest_pool_created))
            .route("/indexer/prediction-placed", post(ingest_prediction_placed))
            .with_state(state),
        RateLimitTier::Write,
    );

    Router::new()
        .merge(light)
        .merge(read)
        .merge(user)
        .merge(write)
}

/// `GET /api/v1/fees` — reads fee config from the shared AppState.
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
    axum::extract::Path(address): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    match state.db {
        Some(pool) => {
            match crate::referrals::get_referrals(axum::extract::Path(address), State(pool)).await {
                Ok((status, body)) => (status, body).into_response(),
                Err(e) => ApiResponse::<()>::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error_codes::INTERNAL_ERROR,
                    e.to_string(),
                )
                .into_response(),
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
    axum::extract::Path(address): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    match state.db {
        Some(pool) => {
            match crate::referrals::estimate_referral_rewards(
                address,
                &pool,
                state.config.treasury_fee_bps,
                state.config.referral_fee_bps,
            )
            .await
            {
                Ok((status, body)) => (status, body).into_response(),
                Err(e) => ApiResponse::<()>::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error_codes::INTERNAL_ERROR,
                    e.to_string(),
                )
                .into_response(),
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
    axum::extract::Path(address): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    match state.db {
        Some(pool) => {
            match crate::referrals::get_user_referral_earnings(
                axum::extract::Path(address),
                State(pool),
            )
            .await
            {
                Ok((status, body)) => (status, body).into_response(),
                Err(e) => ApiResponse::<()>::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error_codes::INTERNAL_ERROR,
                    e.to_string(),
                )
                .into_response(),
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
    use super::*;
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
}
