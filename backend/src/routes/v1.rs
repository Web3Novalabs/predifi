use axum::{routing::get, Json, Router};
use serde_json::json;

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

/// Build the version 1 API router.
pub fn router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
}
