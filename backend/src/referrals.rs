//! # Referral Tracking API
//!
//! Exposes `GET /api/v1/referrals/:address` for the influencer dashboard.
//!
//! ## Expected table schema
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS referrals (
//!     id            BIGSERIAL PRIMARY KEY,
//!     referrer      TEXT      NOT NULL,
//!     user_address  TEXT      NOT NULL,
//!     pool_id       BIGINT    NOT NULL,
//!     amount        BIGINT    NOT NULL DEFAULT 0,
//!     created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
//! );
//! CREATE INDEX IF NOT EXISTS idx_referrals_referrer ON referrals (referrer);
//! ```
//!
//! ## Response
//!
//! ```json
//! {
//!   "status": "success",
//!   "data": {
//!     "referrer": "GABC...",
//!     "total_volume": 12500,
//!     "unique_users": 7
//!   }
//! }
//! ```

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use sqlx::PgPool;

use crate::db::ReferralEarningRow;
use crate::response::ApiResponse;

/// Summary statistics for a single referrer address.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReferralStats {
    /// The queried referrer address.
    #[sqlx(skip)]
    pub referrer: String,
    /// Sum of `amount` across all rows where `referrer = :address`.
    pub total_volume: i64,
    /// Count of distinct `user_address` values referred by this address.
    pub unique_users: i64,
}

/// `GET /api/v1/referrals/:address`
///
/// Returns aggregated referral statistics for the given referrer address.
/// Responds with 404 if the address has no referral records.
pub async fn get_referrals(
    Path(address): Path<String>,
    State(pool): State<PgPool>,
) -> (StatusCode, Json<ApiResponse<ReferralStats>>) {
    #[derive(sqlx::FromRow)]
    struct Row {
        total_volume: i64,
        unique_users: i64,
    }

    let result = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            COALESCE(SUM(amount), 0)::BIGINT   AS total_volume,
            COUNT(DISTINCT user_address)::BIGINT AS unique_users
        FROM referrals
        WHERE referrer = $1
        "#,
    )
    .bind(&address)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(row) if row.unique_users == 0 => ApiResponse::error(
            StatusCode::NOT_FOUND,
            format!("no referrals found for {address}"),
        ),
        Ok(row) => ApiResponse::success(ReferralStats {
            referrer: address,
            total_volume: row.total_volume,
            unique_users: row.unique_users,
        }),
        Err(err) => {
            tracing::error!(error = %err, "referrals query failed");
            ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, "database error")
        }
    }
}

/// Response body for `GET /api/v1/users/:address/referrals`.
#[derive(Debug, Serialize)]
pub struct ReferralEarningsResponse {
    pub referrer: String,
    pub total_earned: i64,
    pub pools: Vec<ReferralEarningRow>,
}

/// `GET /api/v1/users/:address/referrals`
///
/// Returns per-pool referral earnings for the given referrer address.
/// Responds with 404 if the address has no referral records.
pub async fn get_user_referral_earnings(
    Path(address): Path<String>,
    State(pool): State<PgPool>,
) -> (StatusCode, Json<ApiResponse<ReferralEarningsResponse>>) {
    match crate::db::get_referral_earnings(&pool, &address).await {
        Ok(rows) if rows.is_empty() => ApiResponse::error(
            StatusCode::NOT_FOUND,
            format!("no referral earnings found for {address}"),
        ),
        Ok(rows) => {
            let total_earned = rows.iter().map(|r| r.total_earned).sum();
            ApiResponse::success(ReferralEarningsResponse {
                referrer: address,
                total_earned,
                pools: rows,
            })
        }
        Err(err) => {
            tracing::error!(error = %err, "referral earnings query failed");
            ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, "database error")
        }
    }
}
