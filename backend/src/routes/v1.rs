use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::config::Config;
use crate::price_cache::PriceCache;
use crate::db::{self, PoolCreatedEvent};

/// Struct representing fee information, matching the contract structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeeInfo {
    /// Protocol (treasury) fee in basis points.
    pub treasury_fee_bps: u32,
    /// Referral share of the protocol fee in basis points.
    pub referral_fee_bps: u32,
}


/// `GET /api/v1/health` health-check endpoint.
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "version": "v1" }))
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
    /// Optional DB pool — absent in unit tests that don't need a database.
    pub db: Option<sqlx::PgPool>,
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

impl axum::extract::FromRef<AppState> for Option<sqlx::PgPool> {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

// ── Task 2: Active Pools API ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PoolsQuery {
    /// Sort order: "popular" | "ending_soon" | "new" (default)
    pub sort_by: Option<String>,
    /// Category filter, e.g. "Sports", "Crypto"
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// `GET /api/v1/pools` — list active pools with sorting and category filter.
pub async fn get_pools(
    State(state): State<AppState>,
    Query(params): Query<PoolsQuery>,
) -> Json<serde_json::Value> {
    let sort_by = params.sort_by.as_deref().unwrap_or("new");
    let category = params.category.as_deref();
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let Some(db) = &state.db else {
        return Json(json!({ "error": "database not available" }));
    };

    match crate::db::get_active_pools(db, sort_by, category, limit, offset).await {
        Ok(pools) => Json(json!({ "pools": pools, "limit": limit, "offset": offset })),
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

/// Build the version 1 API router.
pub fn router(config: Config, cache: PriceCache, db: Option<sqlx::PgPool>) -> Router {
    let state = AppState { config, cache, db };

    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/fees", get(get_fees))
        .route("/prices", get(crate::price_cache::get_prices))
        .route("/referrals/{address}", get(referrals_handler))
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
    use axum::response::IntoResponse;
    use axum::http::StatusCode;
    use crate::response::ApiResponse;

    match state.db {
        Some(pool) => {
            let (status, body) =
                crate::referrals::get_referrals(axum::extract::Path(address), State(pool)).await;
            (status, body).into_response()
        }
        None => ApiResponse::<()>::error(
            StatusCode::SERVICE_UNAVAILABLE,
            "database not configured",
        )
        .into_response(),
    }
}
