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
//! CREATE INDEX IF NOT EXISTS idx_referrals_user_address ON referrals (user_address);
//! CREATE INDEX IF NOT EXISTS idx_referrals_referrer_pool ON referrals (referrer, pool_id);
//! CREATE INDEX IF NOT EXISTS idx_referrals_pool_id ON referrals (pool_id);
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

use crate::errors::AppError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use sqlx::PgPool;

use crate::db::ReferralEarningRow;
use crate::response::ApiResponse;
use crate::response::error_codes;

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
) -> Result<(StatusCode, Json<ApiResponse<ReferralStats>>), AppError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        total_volume: i64,
        unique_users: i64,
    }

    let result = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            COALESCE(total_volume, 0)::BIGINT AS total_volume,
            unique_users
        FROM referrer_stats
        WHERE referrer = $1
        "#,
    )
    .bind(&address)
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(row)) if row.unique_users == 0 => Ok(ApiResponse::error(
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            format!("no referrals found for {address}"),
        )),
        Ok(Some(row)) => Ok(ApiResponse::success(ReferralStats {
            referrer: address,
            total_volume: row.total_volume,
            unique_users: row.unique_users,
        })),
        Ok(None) => Ok(ApiResponse::error(
            StatusCode::NOT_FOUND,
            format!("no referrals found for {address}"),
        )),
        Err(err) => {
            tracing::error!(error = %err, "referrals query failed");
            Err(AppError::from(err))
        }
    }
}

/// Basis-point denominator (100% = 10_000 bps).
const BPS_DENOMINATOR: i128 = 10_000;

/// Estimate the referral reward a referrer would earn on a given volume.
///
/// A referrer earns the referral share of the protocol (treasury) fee charged
/// on the staked volume they brought in:
///
/// `reward = volume * (treasury_fee_bps / 10_000) * (referral_fee_bps / 10_000)`
///
/// Computed with `i128` intermediates to avoid overflow and floored to whole
/// token units. Non-positive volumes yield `0`.
pub fn estimate_referral_reward(
    referral_volume: i64,
    treasury_fee_bps: u32,
    referral_fee_bps: u32,
) -> i64 {
    if referral_volume <= 0 {
        return 0;
    }
    let treasury_fee = referral_volume as i128 * treasury_fee_bps as i128 / BPS_DENOMINATOR;
    let reward = treasury_fee * referral_fee_bps as i128 / BPS_DENOMINATOR;
    reward.clamp(0, i64::MAX as i128) as i64
}

/// Response body for `GET /api/v1/referrals/:address/estimate`.
#[derive(Debug, Serialize)]
pub struct ReferralRewardEstimate {
    pub referrer: String,
    /// Total referred volume (sum of `amount`) for this referrer.
    pub total_volume: i64,
    pub treasury_fee_bps: u32,
    pub referral_fee_bps: u32,
    /// Estimated reward derived from the referred volume and fee rates.
    pub estimated_reward: i64,
}

/// `GET /api/v1/referrals/:address/estimate`
///
/// Estimates the referral reward for a referrer based on their total referred
/// volume and the configured protocol fee rates.
pub async fn estimate_referral_rewards(
    address: String,
    pool: &PgPool,
    treasury_fee_bps: u32,
    referral_fee_bps: u32,
) -> Result<(StatusCode, Json<ApiResponse<ReferralRewardEstimate>>), AppError> {
    let total_volume = sqlx::query_scalar::<_, i64>(
        r#"SELECT COALESCE(total_volume, 0)::BIGINT FROM referrer_stats WHERE referrer = $1"#,
    )
    .bind(&address)
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);

    let estimated_reward = estimate_referral_reward(total_volume, treasury_fee_bps, referral_fee_bps);

    Ok(ApiResponse::success(ReferralRewardEstimate {
        referrer: address,
        total_volume,
        treasury_fee_bps,
        referral_fee_bps,
        estimated_reward,
    }))
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
) -> Result<(StatusCode, Json<ApiResponse<ReferralEarningsResponse>>), AppError> {
    match crate::db::get_referral_earnings(&pool, &address).await {
        Ok(rows) if rows.is_empty() => Ok(ApiResponse::error(
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            format!("no referral earnings found for {address}"),
        )),
        Ok(rows) => {
            let total_earned = rows.iter().map(|r| r.total_earned).sum();
            Ok(ApiResponse::success(ReferralEarningsResponse {
                referrer: address,
                total_earned,
                pools: rows,
            }))
        }
        Err(err) => {
            tracing::error!(error = %err, "referral earnings query failed");
            Err(AppError::from(err))
        }
    }
}

#[cfg(test)]
mod estimation_tests {
    use super::estimate_referral_reward;

    #[test]
    fn estimates_reward_from_volume_and_fee_rates() {
        // 1_000_000 volume, 2% treasury fee, 30% referral share => 6_000.
        assert_eq!(estimate_referral_reward(1_000_000, 200, 3_000), 6_000);
    }

    #[test]
    fn non_positive_volume_yields_zero() {
        assert_eq!(estimate_referral_reward(0, 200, 3_000), 0);
        assert_eq!(estimate_referral_reward(-100, 200, 3_000), 0);
    }

    #[test]
    fn zero_fee_rates_yield_zero() {
        assert_eq!(estimate_referral_reward(1_000_000, 0, 3_000), 0);
        assert_eq!(estimate_referral_reward(1_000_000, 200, 0), 0);
    }

    #[test]
    fn large_volume_does_not_overflow() {
        // i64::MAX volume must not panic and must stay within i64 range.
        let reward = estimate_referral_reward(i64::MAX, 10_000, 10_000);
        assert_eq!(reward, i64::MAX);
    }
}
