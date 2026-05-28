//! OpenAPI / Swagger documentation.
//!
//! The Swagger UI is served at `/swagger-ui/`.

use axum::Router;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

// ── Documented schemas ────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PoolDoc {
    pub pool_id: i64,
    pub name: String,
    pub category: String,
    pub total_stake: i64,
    pub end_time: i64,
    pub created_at: String,
    pub state: String,
    pub creator: String,
    pub token: String,
    pub result: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct OutcomeOddsDoc {
    pub outcome: i32,
    pub stake: i64,
    pub odds: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PoolWithOddsDoc {
    #[serde(flatten)]
    pub pool: PoolDoc,
    pub odds: Vec<OutcomeOddsDoc>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PoolListResponse {
    pub pools: Vec<PoolDoc>,
    pub limit: i64,
    pub offset: i64,
    pub status: String,
    pub sort_by: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PredictionDoc {
    pub pool_id: i64,
    pub pool_name: String,
    pub pool_result: Option<String>,
    pub outcome: i32,
    pub amount: i64,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PredictionHistoryResponse {
    pub address: String,
    pub predictions: Vec<PredictionDoc>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UserPredictionDoc {
    pub prediction_id: i64,
    pub pool_id: i64,
    pub pool_name: String,
    pub pool_category: String,
    pub pool_state: String,
    pub pool_total_stake: i64,
    pub pool_result: Option<String>,
    pub user_outcome: i32,
    pub user_amount: i64,
    pub is_winning_outcome: Option<bool>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UserPredictionsResponse {
    pub address: String,
    pub predictions: Vec<UserPredictionDoc>,
    pub limit: i64,
    pub offset: i64,
    pub total_predictions: usize,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ProtocolStatsDoc {
    /// Total value locked across all pools (stroops).
    pub total_value_locked: i64,
    pub total_bets: i64,
    pub total_pools: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct FeeInfoDoc {
    /// Protocol (treasury) fee in basis points.
    pub treasury_fee_bps: u32,
    /// Referral share of the protocol fee in basis points.
    pub referral_fee_bps: u32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct LeaderboardEntryVolume {
    pub user_address: String,
    pub total_volume: i64,
    pub prediction_count: i64,
    pub rank: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct LeaderboardEntryWinnings {
    pub user_address: String,
    pub total_winnings: i64,
    pub winning_predictions: i64,
    pub total_predictions: i64,
    pub win_rate: f64,
    pub rank: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct LeaderboardResponse {
    pub leaderboard: Vec<LeaderboardEntryVolume>,
    pub rank_by: String,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ReferralEarningDoc {
    pub pool_id: i64,
    pub pool_name: String,
    pub total_earned: i64,
    pub referral_count: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ReferralEarningsResponse {
    pub referrer: String,
    pub total_earned: i64,
    pub pools: Vec<ReferralEarningDoc>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct DependencyStatus {
    pub db: String,
    pub rpc: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub dependencies: DependencyStatus,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PoolCreatedPayloadDoc {
    pub pool_id: u64,
    pub creator: String,
    pub end_time: u64,
    pub token: String,
    pub category: String,
    pub description: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PredictionPlacedPayloadDoc {
    pub pool_id: u64,
    pub user_address: String,
    pub outcome: i32,
    pub amount: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct IndexerOkResponse {
    pub status: String,
    pub pool_id: u64,
}

// ── OpenAPI spec ──────────────────────────────────────────────────────────────

#[derive(OpenApi)]
#[openapi(
    info(
        title = "PrediFi API",
        version = "1.0.0",
        description = "REST API for the PrediFi prediction-market platform built on Stellar/Soroban.",
        contact(name = "PrediFi Team", url = "https://github.com/Web3Novalabs/predifi"),
        license(name = "MIT")
    ),
    paths(
        api_health,
        api_get_pools,
        api_get_pool_by_id,
        api_get_stats,
        api_get_fees,
        api_get_prices,
        api_get_leaderboard,
        api_get_user_history,
        api_get_user_predictions,
        api_get_referrals,
        api_get_user_referral_earnings,
        api_ingest_pool_created,
        api_ingest_prediction_placed,
    ),
    components(schemas(
        PoolDoc,
        OutcomeOddsDoc,
        PoolWithOddsDoc,
        PoolListResponse,
        PredictionDoc,
        PredictionHistoryResponse,
        UserPredictionDoc,
        UserPredictionsResponse,
        ProtocolStatsDoc,
        FeeInfoDoc,
        LeaderboardEntryVolume,
        LeaderboardEntryWinnings,
        LeaderboardResponse,
        ReferralEarningDoc,
        ReferralEarningsResponse,
        DependencyStatus,
        HealthResponse,
        ErrorResponse,
        PoolCreatedPayloadDoc,
        PredictionPlacedPayloadDoc,
        IndexerOkResponse,
    )),
    tags(
        (name = "health", description = "Service liveness and dependency checks"),
        (name = "pools", description = "Prediction market pools"),
        (name = "stats", description = "Protocol-wide statistics"),
        (name = "leaderboard", description = "User rankings"),
        (name = "predictions", description = "User prediction history"),
        (name = "referrals", description = "Referral earnings"),
        (name = "indexer", description = "Internal event ingestion endpoints"),
    )
)]
pub struct ApiDoc;

// ── Path stubs ────────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[utoipa::path(get, path = "/health", tag = "health",
    responses(
        (status = 200, description = "All dependencies healthy", body = HealthResponse),
        (status = 503, description = "One or more dependencies unreachable", body = HealthResponse),
    )
)]
async fn api_health() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/pools", tag = "pools",
    params(
        ("sort_by" = Option<String>, Query, description = "popular | ending_soon | new (default)"),
        ("category" = Option<String>, Query, description = "Category filter e.g. Sports, Crypto"),
        ("status" = Option<String>, Query, description = "active | closed | settled (default: active)"),
        ("limit" = Option<i64>, Query, description = "Max results (default 20, max 100)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default 0)"),
    ),
    responses(
        (status = 200, description = "Paginated pool list", body = PoolListResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
async fn api_get_pools() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/pools/{pool_id}", tag = "pools",
    params(("pool_id" = i64, Path, description = "On-chain pool identifier")),
    responses(
        (status = 200, description = "Pool details with live odds", body = PoolWithOddsDoc),
        (status = 404, description = "Pool not found", body = ErrorResponse),
    )
)]
async fn api_get_pool_by_id() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/stats", tag = "stats",
    responses(
        (status = 200, description = "Protocol-wide aggregate statistics", body = ProtocolStatsDoc),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
async fn api_get_stats() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/fees", tag = "stats",
    responses(
        (status = 200, description = "Current protocol fee configuration", body = FeeInfoDoc),
    )
)]
async fn api_get_fees() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/prices", tag = "stats",
    responses(
        (status = 200, description = "Latest cached asset prices"),
    )
)]
async fn api_get_prices() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/leaderboard", tag = "leaderboard",
    params(
        ("rank_by" = Option<String>, Query, description = "volume (default) | winnings"),
        ("limit" = Option<i64>, Query, description = "Max results (default 20, max 100)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default 0)"),
    ),
    responses(
        (status = 200, description = "User leaderboard", body = LeaderboardResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
async fn api_get_leaderboard() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/users/{address}/history", tag = "predictions",
    params(
        ("address" = String, Path, description = "Stellar account address (G...)"),
        ("limit" = Option<i64>, Query, description = "Max results (default 20, max 100)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default 0)"),
    ),
    responses(
        (status = 200, description = "Paginated prediction history", body = PredictionHistoryResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
async fn api_get_user_history() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/users/{address}/predictions", tag = "predictions",
    params(
        ("address" = String, Path, description = "Stellar account address (G...)"),
        ("limit" = Option<i64>, Query, description = "Max results (default 20, max 100)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default 0)"),
    ),
    responses(
        (status = 200, description = "Enhanced predictions with pool status", body = UserPredictionsResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
async fn api_get_user_predictions() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/referrals/{address}", tag = "referrals",
    params(("address" = String, Path, description = "Stellar referrer address (G...)")),
    responses(
        (status = 200, description = "Referral summary", body = ReferralEarningsResponse),
        (status = 503, description = "Database not configured", body = ErrorResponse),
    )
)]
async fn api_get_referrals() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/api/v1/users/{address}/referrals", tag = "referrals",
    params(("address" = String, Path, description = "Stellar referrer address (G...)")),
    responses(
        (status = 200, description = "Per-pool referral earnings", body = ReferralEarningsResponse),
        (status = 503, description = "Database not configured", body = ErrorResponse),
    )
)]
async fn api_get_user_referral_earnings() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/api/v1/indexer/pool-created", tag = "indexer",
    request_body = PoolCreatedPayloadDoc,
    responses(
        (status = 200, description = "Pool indexed successfully", body = IndexerOkResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
async fn api_ingest_pool_created() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/api/v1/indexer/prediction-placed", tag = "indexer",
    request_body = PredictionPlacedPayloadDoc,
    responses(
        (status = 200, description = "Prediction indexed successfully", body = IndexerOkResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
async fn api_ingest_prediction_placed() {}

// ── Router ────────────────────────────────────────────────────────────────────

/// Mount Swagger UI at `/swagger-ui` and serve the OpenAPI JSON at `/api-docs/openapi.json`.
pub fn swagger_router() -> Router {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .into()
}
