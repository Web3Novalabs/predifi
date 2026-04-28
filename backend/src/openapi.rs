//! OpenAPI / Swagger documentation (#563).
//!
//! Documents the `Pool` and `Prediction` types and all v1 API endpoints.
//! The Swagger UI is served at `/swagger-ui/`.
//!
//! # Integration
//! Call [`swagger_router`] and merge the returned router into the main app:
//! ```rust,no_run
//! let app = Router::new()
//!     .merge(openapi::swagger_router())
//!     .nest("/api", routes::router(...));
//! ```

use axum::Router;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

// ── Documented types ──────────────────────────────────────────────────────────

/// A prediction market pool.
#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PoolDoc {
    /// Unique pool identifier (auto-incremented on-chain).
    pub pool_id: i64,
    /// Human-readable pool name / question.
    pub name: String,
    /// Market category (e.g. "Crypto", "Sports").
    pub category: String,
    /// Aggregated stake across all outcomes (in stroops).
    pub total_stake: i64,
    /// Unix timestamp (seconds) when the pool closes.
    pub end_time: i64,
    /// ISO-8601 timestamp when the pool was indexed into the DB.
    pub created_at: String,
}

/// A user's prediction on a pool.
#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PredictionDoc {
    /// Pool the prediction belongs to.
    pub pool_id: i64,
    /// Stellar address of the predictor.
    pub user_address: String,
    /// Outcome index the user predicted (0-based).
    pub outcome: i32,
    /// Amount staked in stroops.
    pub amount: i64,
    /// ISO-8601 timestamp when the prediction was indexed.
    pub created_at: String,
}

/// Paginated list of active pools.
#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PoolListResponse {
    pub pools: Vec<PoolDoc>,
    pub limit: i64,
    pub offset: i64,
}

/// Paginated list of a user's predictions.
#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PredictionHistoryResponse {
    pub address: String,
    pub predictions: Vec<PredictionDoc>,
    pub limit: i64,
    pub offset: i64,
}

/// Generic error response.
#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

/// Per-pool referral earnings entry.
#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ReferralEarningDoc {
    pub pool_id: i64,
    pub pool_name: String,
    /// Total amount earned from referrals in this pool (stroops).
    pub total_earned: i64,
    /// Number of referred users who staked in this pool.
    pub referral_count: i64,
}

/// Referral earnings breakdown response.
#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ReferralEarningsResponse {
    pub referrer: String,
    /// Aggregate earnings across all pools (stroops).
    pub total_earned: i64,
    pub pools: Vec<ReferralEarningDoc>,
}

// ── OpenAPI spec definition ───────────────────────────────────────────────────

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
        api_get_pools,
        api_get_pool_by_id,
        api_get_user_history,
        api_get_user_referral_earnings,
        api_health,
    ),
    components(
        schemas(
            PoolDoc,
            PredictionDoc,
            PoolListResponse,
            PredictionHistoryResponse,
            ReferralEarningDoc,
            ReferralEarningsResponse,
            ErrorResponse,
        )
    ),
    tags(
        (name = "pools", description = "Active prediction market pools"),
        (name = "predictions", description = "User prediction history"),
        (name = "referrals", description = "Referral earnings dashboard"),
        (name = "health", description = "Service health endpoints"),
    )
)]
pub struct ApiDoc;

// ── Path stubs for utoipa path macros ─────────────────────────────────────────
// These are documentation-only stubs. The real handlers live in routes/v1.rs.
// utoipa requires #[utoipa::path] attributes on functions; we use lightweight
// stubs here so the OpenAPI spec can be generated without duplicating business
// logic.

/// `GET /api/v1/pools` — list active pools with optional sorting and filtering.
#[utoipa::path(
    get,
    path = "/api/v1/pools",
    tag = "pools",
    params(
        ("sort_by" = Option<String>, Query, description = "Sort order: popular | ending_soon | new (default)"),
        ("category" = Option<String>, Query, description = "Filter by category e.g. Sports, Crypto"),
        ("limit" = Option<i64>, Query, description = "Max results per page (default 20, max 100)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default 0)"),
    ),
    responses(
        (status = 200, description = "List of active pools", body = PoolListResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
async fn api_get_pools() {}

/// `GET /api/v1/pools/{pool_id}` — retrieve a single pool by ID.
#[utoipa::path(
    get,
    path = "/api/v1/pools/{pool_id}",
    tag = "pools",
    params(
        ("pool_id" = i64, Path, description = "On-chain pool identifier"),
    ),
    responses(
        (status = 200, description = "Pool details", body = PoolDoc),
        (status = 404, description = "Pool not found", body = ErrorResponse),
    )
)]
async fn api_get_pool_by_id() {}

/// `GET /api/v1/users/{address}/history` — paginated prediction history for a user.
#[utoipa::path(
    get,
    path = "/api/v1/users/{address}/history",
    tag = "predictions",
    params(
        ("address" = String, Path, description = "Stellar account address (G...)"),
        ("limit" = Option<i64>, Query, description = "Max results per page (default 20, max 100)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default 0)"),
    ),
    responses(
        (status = 200, description = "Prediction history", body = PredictionHistoryResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
async fn api_get_user_history() {}

/// `GET /health` — service liveness check.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy"),
    )
)]
async fn api_health() {}

/// `GET /api/v1/users/{address}/referrals` — per-pool referral earnings for a user.
#[utoipa::path(
    get,
    path = "/api/v1/users/{address}/referrals",
    tag = "referrals",
    params(
        ("address" = String, Path, description = "Stellar referrer address (G...)"),
    ),
    responses(
        (status = 200, description = "Referral earnings breakdown per pool", body = ReferralEarningsResponse),
        (status = 404, description = "No referral records found", body = ErrorResponse),
        (status = 503, description = "Database not configured", body = ErrorResponse),
    )
)]
async fn api_get_user_referral_earnings() {}

// ── Swagger UI router ─────────────────────────────────────────────────────────

/// Build the Swagger UI router and mount it at `/swagger-ui/`.
///
/// Merge this into the main Axum router before serving:
/// ```rust,no_run
/// let app = Router::new().merge(openapi::swagger_router()).nest("/api", ...);
/// ```
pub fn swagger_router() -> Router {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .into()
}
