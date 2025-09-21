use crate::controllers::validator_controller::*;
use crate::db::database::AppState;
use crate::error::AppResult;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

pub fn validator_routes() -> Router<AppState> {
    Router::new()
        .route("/validator/:address", get(get_validator_handler))
        .route("/validators", get(get_validators_handler))
        .route(
            "/validator/:address/status",
            get(get_validator_status_handler),
        )
}

async fn get_validator_handler(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> AppResult<Json<crate::models::validator::Validator>> {
    let validator = get_validator(state.db.pool(), &address).await?;
    Ok(Json(validator))
}

async fn get_validators_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::models::validator::Validator>>> {
    let validators = get_validators(state.db.pool()).await?;
    Ok(Json(validators))
}

async fn get_validator_status_handler(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> AppResult<Json<bool>> {
    let status = get_validator_status(state.db.pool(), &address).await?;
    Ok(Json(status))
}
