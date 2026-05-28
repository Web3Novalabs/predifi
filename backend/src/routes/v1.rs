use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::Config;
use crate::db::{PoolCreatedEvent, PredictionPlacedEvent};
use crate::metrics::SharedMetrics;
use crate::price_cache::PriceCache;
use crate::redis_cache::RedisCache;

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

    if let Some(db) = &state.db {
        if sqlx::query("SELECT 1").execute(db).await.is_err() {
            db_status = "unreachable";
            all_healthy = false;
        }
    } else {
        db_status = "not_configured";
    }

    let mut rpc_status = "ok";
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

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
        Ok(res) if res.status().is_success() => {}
        _ => {
            rpc_status = "unreachable";
            all_healthy = false;
        }
    }

    let body = serde_json::json!({
        "status": if all_healthy { "ok" } else { "error" },
        "version": "v1",
        "dependencies": {
            "db": db_status,
            "rpc": rpc_status
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
    pub config: Config,
    pub cache: PriceCache,
    pub redis: RedisCache,
    /// Optional DB pool — absent in unit tests that don't need a database.
    pub db: Option<sqlx::PgPool>,
    pub metrics: SharedMetrics,
    pub event_bus: crate::ws::EventBus,
}

impl axum::extract::FromRef<AppState> for Config {
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

#[derive(Debug, Deserialize)]
pub struct PoolsQuery {
    /// Sort order: "popular" | "ending_soon" | "new" (default)
    pub sort_by: Option<String>,
    /// Category filter, e.g. "Sports", "Crypto"
    pub category: Option<String>,
    /// Status filter: "active" | "closed" | "settled" (default: "active")
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// `GET /api/v1/pools/:id` — get detailed pool information with real-time odds.
pub async fn get_pool_by_id_handler(
    State(state): State<AppState>,
    Path(pool_id): Path<i64>,
) -> Json<serde_json::Value> {
    let Some(db) = &state.db else {
        return Json(json!({ "error": "database not available" }));
    };

    match crate::db::get_pool_with_odds(db, pool_id).await {
        Ok(Some(pool)) => Json(json!(pool)),
        Ok(None) => Json(json!({ "error": "pool not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn get_stats(State(state): State<AppState>) -> Json<serde_json::Value> {
    let Some(db) = &state.db else {
        return Json(json!({ "error": "database not available" }));
    };

    match crate::db::get_protocol_stats(db).await {
        Ok(stats) => Json(json!(stats)),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
pub async fn get_pools(
    State(state): State<AppState>,
    Query(params): Query<PoolsQuery>,
) -> Json<serde_json::Value> {
    let sort_by = params.sort_by.as_deref().unwrap_or("new");
    let category = params.category.as_deref();
    let status = params.status.as_deref().unwrap_or("active");
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let Some(db) = &state.db else {
        return Json(json!({ "error": "database not available" }));
    };

    // Try to get from Redis cache first
    let cache_key = crate::redis_cache::pools_cache_key(sort_by, category, status, limit, offset);
    
    if let Some(cached_response) = state.redis.get::<serde_json::Value>(&cache_key).await {
        return Json(cached_response);
    }

    // Cache miss - fetch from database
    match crate::db::get_pools_with_filters(db, sort_by, category, status, limit, offset).await {
        Ok(pools) => {
            let response = json!({
                "pools": pools,
                "limit": limit,
                "offset": offset,
                "status": status,
                "category": category,
                "sort_by": sort_by
            });
            
            // Cache the response for 60 seconds
            state.redis.set(&cache_key, &response, crate::redis_cache::POOLS_CACHE_TTL).await;
            
            Json(response)
        },
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

// ── Task 3: User Prediction History API ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// `GET /api/v1/users/:address/history` — paginated prediction history for a user.
pub async fn get_user_history(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let Some(db) = &state.db else {
        return Json(json!({ "error": "database not available" }));
    };

    match crate::db::get_user_prediction_history(db, &address, limit, offset).await {
        Ok(rows) => Json(json!({
            "address": address,
            "predictions": rows,
            "limit": limit,
            "offset": offset,
        })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// `GET /api/v1/users/:address/predictions` — enhanced user predictions with current pool status.
pub async fn get_user_predictions(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let Some(db) = &state.db else {
        return Json(json!({ "error": "database not available" }));
    };

    match crate::db::get_user_predictions(db, &address, limit, offset).await {
        Ok(predictions) => Json(json!({
            "address": address,
            "predictions": predictions,
            "limit": limit,
            "offset": offset,
            "total_predictions": predictions.len(),
        })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Ranking type: "volume" | "winnings" (default: "volume")
    pub rank_by: Option<String>,
}

/// `GET /api/v1/leaderboard` — user rankings by betting volume or winnings.
pub async fn get_leaderboard(
    State(state): State<AppState>,
    Query(params): Query<LeaderboardQuery>,
) -> Json<serde_json::Value> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let rank_by = params.rank_by.as_deref().unwrap_or("volume");

    let Some(db) = &state.db else {
        return Json(json!({ "error": "database not available" }));
    };

    match rank_by {
        "winnings" => match crate::db::get_users_by_winnings(db, limit, offset).await {
            Ok(users) => Json(json!({
                "leaderboard": users,
                "rank_by": "winnings",
                "limit": limit,
                "offset": offset,
            })),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        _ => {
            // Default to volume ranking
            match crate::db::get_users_by_betting_volume(db, limit, offset).await {
                Ok(users) => Json(json!({
                    "leaderboard": users,
                    "rank_by": "volume",
                    "limit": limit,
                    "offset": offset,
                })),
                Err(e) => Json(json!({ "error": e.to_string() })),
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
    pub creator: String,
    pub end_time: u64,
    pub token: String,
    pub category: String,
    /// Pool description / name decoded from the event data.
    pub description: String,
}

/// `POST /api/v1/indexer/pool-created` — ingest a decoded `PoolCreated` event
/// and persist the new pool to the database.
pub async fn ingest_pool_created(
    State(state): State<AppState>,
    Json(payload): Json<PoolCreatedPayload>,
) -> Json<serde_json::Value> {
    let Some(db) = &state.db else {
        return Json(json!({ "error": "database not available" }));
    };

    let event = PoolCreatedEvent {
        pool_id: payload.pool_id,
        creator: payload.creator,
        end_time: payload.end_time,
        token: payload.token,
        category: payload.category,
        description: payload.description,
    };

    match crate::db::insert_pool_from_event(db, &event).await {
        Ok(()) => Json(json!({ "status": "ok", "pool_id": event.pool_id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Request body for the prediction-placed event webhook.
#[derive(Debug, Deserialize)]
pub struct PredictionPlacedPayload {
    pub pool_id: u64,
    pub user_address: String,
    pub outcome: i32,
    pub amount: i64,
}

/// `POST /api/v1/indexer/prediction-placed` — ingest a decoded prediction event and update DB state.
pub async fn ingest_prediction_placed(
    State(state): State<AppState>,
    Json(payload): Json<PredictionPlacedPayload>,
) -> Json<serde_json::Value> {
    let Some(db) = &state.db else {
        return Json(json!({ "error": "database not available" }));
    };

    let event = PredictionPlacedEvent {
        pool_id: payload.pool_id,
        user_address: payload.user_address,
        amount: payload.amount,
        outcome: payload.outcome,
    };

    match crate::db::insert_prediction_from_event(db, &event).await {
        Ok(()) => {
            state.event_bus.send(&json!({
                "type": "prediction_placed",
                "pool_id": event.pool_id,
                "user_address": event.user_address,
                "outcome": event.outcome,
                "amount": event.amount,
            }));
            Json(json!({ "status": "ok", "pool_id": event.pool_id }))
        }
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Build the version 1 API router.
pub fn router(
    config: Config,
    cache: PriceCache,
    redis: RedisCache,
    pool: Option<sqlx::PgPool>,
    metrics: SharedMetrics,
    event_bus: crate::ws::EventBus,
) -> Router {
    let state = AppState {
        config,
        cache,
        redis,
        db: pool,
        metrics,
        event_bus,
    };

    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/pools", get(get_pools))
        .route("/pools/:id", get(get_pool_by_id_handler))
        .route("/stats", get(get_stats))
        .route("/leaderboard", get(get_leaderboard))
        .route("/fees", get(get_fees))
        .route("/prices", get(crate::price_cache::get_prices))
        .route("/referrals/{address}", get(referrals_handler))
        .route("/users/{address}/referrals", get(user_referral_earnings_handler))
        .route("/users/{address}/history", get(get_user_history))
        .route("/users/{address}/predictions", get(get_user_predictions))
        .route("/indexer/pool-created", post(ingest_pool_created))
        .route("/indexer/prediction-placed", post(ingest_prediction_placed))
        .route("/ws", get(crate::ws::ws_handler))
        .with_state(state)
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
    use crate::response::ApiResponse;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    match state.db {
        Some(pool) => {
            let (status, body) =
                crate::referrals::get_referrals(axum::extract::Path(address), State(pool)).await;
            (status, body).into_response()
        }
        None => {
            ApiResponse::<()>::error(StatusCode::SERVICE_UNAVAILABLE, "database not configured")
                .into_response()
        }
    }
}

/// `GET /api/v1/users/:address/referrals` — per-pool referral earnings for a user.
async fn user_referral_earnings_handler(
    axum::extract::Path(address): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> axum::response::Response {
    use crate::response::ApiResponse;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    match state.db {
        Some(pool) => {
            let (status, body) = crate::referrals::get_user_referral_earnings(
                axum::extract::Path(address),
                State(pool),
            )
            .await;
            (status, body).into_response()
        }
        None => {
            ApiResponse::<()>::error(StatusCode::SERVICE_UNAVAILABLE, "database not configured")
                .into_response()
        }
    }
}
