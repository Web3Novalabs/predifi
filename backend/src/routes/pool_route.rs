use crate::controllers::pool_controller::*;
use crate::db::database::AppState;
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

async fn active_pools_handler(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse {
    let limit = pagination.limit.unwrap_or(20);
    let offset = pagination.offset.unwrap_or(0);
    match get_active_pools(state.db.pool(), limit, offset).await {
        Ok(pools) => (StatusCode::OK, axum::Json(pools)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch active pools",
        )
            .into_response(),
    }
}

async fn locked_pools_handler(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse {
    let limit = pagination.limit.unwrap_or(20);
    let offset = pagination.offset.unwrap_or(0);
    match get_locked_pools(state.db.pool(), limit, offset).await {
        Ok(pools) => (StatusCode::OK, axum::Json(pools)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch locked pools",
        )
            .into_response(),
    }
}

async fn settled_pools_handler(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse {
    let limit = pagination.limit.unwrap_or(20);
    let offset = pagination.offset.unwrap_or(0);
    match get_settled_pools(state.db.pool(), limit, offset).await {
        Ok(pools) => (StatusCode::OK, axum::Json(pools)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch settled pools",
        )
            .into_response(),
    }
}

async fn closed_pools_handler(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse {
    let limit = pagination.limit.unwrap_or(20);
    let offset = pagination.offset.unwrap_or(0);
    match get_closed_pools(state.db.pool(), limit, offset).await {
        Ok(pools) => (StatusCode::OK, axum::Json(pools)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch closed pools",
        )
            .into_response(),
    }
}

pub fn pool_routes() -> Router<AppState> {
    Router::new()
        .route("/pools/active", get(active_pools_handler))
        .route("/pools/locked", get(locked_pools_handler))
        .route("/pools/settled", get(settled_pools_handler))
        .route("/pools/closed", get(closed_pools_handler))
}
