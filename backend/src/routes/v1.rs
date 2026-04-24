use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::Config;

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

/// `GET /api/v1/fees` returns the current fee configuration.
pub async fn get_fees(State(config): State<Config>) -> Json<FeeInfo> {
    Json(FeeInfo {
        treasury_fee_bps: config.treasury_fee_bps,
        referral_fee_bps: config.referral_fee_bps,
    })
}

/// Build the version 1 API router.
pub fn router(config: Config) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/fees", get(get_fees))
        .with_state(config)
}
