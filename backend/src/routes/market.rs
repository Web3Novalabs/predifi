use crate::AppState;
use crate::{
    controllers::market_controller,
    models::market::{MarketWithTags, NewMarket},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateMarketRequest {
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub image_url: Option<String>,
    pub event_source_url: Option<String>,
    pub start_time: Option<i64>,
    pub lock_time: Option<i64>,
    pub end_time: Option<i64>,
    pub option1: Option<String>,
    pub option2: Option<String>,
    pub min_bet_amount: Option<String>,
    pub max_bet_amount: Option<String>,
    pub creator_fee: Option<i16>,
    pub is_private: Option<bool>,
    pub creator_address: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CreateMarketResponse {
    pub success: bool,
    pub data: MarketWithTags,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct GetMarketResponse {
    pub success: bool,
    pub data: MarketWithTags,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub message: String,
}

pub async fn create_market_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateMarketRequest>,
) -> Result<(StatusCode, Json<CreateMarketResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Convert string amounts to BigDecimal
    let min_bet_amount = payload
        .min_bet_amount
        .as_ref()
        .and_then(|s| s.parse::<bigdecimal::BigDecimal>().ok());

    let max_bet_amount = payload
        .max_bet_amount
        .as_ref()
        .and_then(|s| s.parse::<bigdecimal::BigDecimal>().ok());

    let new_market = NewMarket {
        name: payload.name,
        description: payload.description,
        category_id: payload.category_id,
        image_url: payload.image_url,
        event_source_url: payload.event_source_url,
        start_time: payload.start_time,
        lock_time: payload.lock_time,
        end_time: payload.end_time,
        option1: payload.option1,
        option2: payload.option2,
        min_bet_amount,
        max_bet_amount,
        creator_fee: payload.creator_fee,
        is_private: payload.is_private,
        creator_address: payload.creator_address,
        created_timestamp: Some(chrono::Utc::now().timestamp()),
        tags: payload.tags,
    };

    match market_controller::create_market_with_tags(state.db.pool(), &new_market).await {
        Ok(market_with_tags) => {
            let response = CreateMarketResponse {
                success: true,
                data: market_with_tags,
                message: "Market created successfully".to_string(),
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            let error_response = ErrorResponse {
                success: false,
                error: format!("Failed to create market: {e}"),
                message: "Market creation failed".to_string(),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn get_market_handler(
    State(state): State<AppState>,
    Path(market_id): Path<i32>,
) -> Result<(StatusCode, Json<GetMarketResponse>), (StatusCode, Json<ErrorResponse>)> {
    match market_controller::get_market_with_tags(state.db.pool(), market_id).await {
        Ok(market_with_tags) => {
            let response = GetMarketResponse {
                success: true,
                data: market_with_tags,
                message: "Market retrieved successfully".to_string(),
            };
            Ok((StatusCode::OK, Json(response)))
        }
        Err(e) => {
            let error_response = ErrorResponse {
                success: false,
                error: format!("Failed to retrieve market: {e}"),
                message: "Market retrieval failed".to_string(),
            };
            Err((StatusCode::NOT_FOUND, Json(error_response)))
        }
    }
}
