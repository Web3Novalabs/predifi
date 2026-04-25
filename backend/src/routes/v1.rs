use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::Config;
use crate::price_cache::PriceCache;

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
    pub pool: Option<sqlx::PgPool>,
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
        state.pool.clone()
    }
}

/// Build the version 1 API router.
pub fn router(config: Config, cache: PriceCache, pool: Option<sqlx::PgPool>) -> Router {
    let state = AppState { config, cache, pool };

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

    match state.pool {
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
