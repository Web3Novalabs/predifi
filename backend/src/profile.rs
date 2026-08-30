//! User profile aggregation: prediction history, win/loss ratio, earnings,
//! active positions, and per-pool claim status.
//!
//! Kept as an independent module (rather than growing `db.rs`) so profile
//! queries can be developed and reasoned about without touching the existing
//! pool/prediction indexing code paths.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};

/// Aggregate prediction stats for a user across every pool they've staked in.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileStats {
    /// Total number of predictions placed by the user.
    pub total_predictions: i64,
    /// Number of settled predictions won by the user.
    pub wins: i64,
    /// Number of settled predictions lost by the user.
    pub losses: i64,
    /// Number of predictions that have not settled yet.
    pub pending: i64,
    /// Win rate as a percentage of *settled* predictions (0.0 when none are settled yet).
    pub win_rate: f64,
    /// Total amount staked across all predictions.
    pub total_staked: i64,
    /// Total amount paid out for claimed winning predictions.
    pub total_earnings: i64,
    /// Number of distinct pools with an active position.
    pub active_positions: i64,
}

#[derive(FromRow)]
struct ProfileStatsRow {
    total_predictions: i64,
    wins: i64,
    losses: i64,
    pending: i64,
    total_staked: i64,
    total_earnings: i64,
    active_positions: i64,
}

/// Fetch aggregate prediction stats (wins/losses/earnings/active positions) for `address`.
pub async fn get_profile_stats(pool: &PgPool, address: &str) -> Result<ProfileStats, sqlx::Error> {
    let row = sqlx::query_as::<_, ProfileStatsRow>(
        r#"
        SELECT
            COUNT(*)::BIGINT AS total_predictions,
            COUNT(*) FILTER (
                WHERE pl.state = 'settled' AND pl.result IS NOT NULL
                  AND pl.result ~ '^\d+$' AND pl.result::int = p.outcome
            )::BIGINT AS wins,
            COUNT(*) FILTER (
                WHERE pl.state = 'settled' AND pl.result IS NOT NULL
                  AND pl.result ~ '^\d+$' AND pl.result::int <> p.outcome
            )::BIGINT AS losses,
            COUNT(*) FILTER (WHERE pl.state <> 'settled')::BIGINT AS pending,
            COALESCE(SUM(p.amount), 0)::BIGINT AS total_staked,
            COALESCE(SUM(p.claimed_amount) FILTER (WHERE p.claimed), 0)::BIGINT AS total_earnings,
            COUNT(DISTINCT p.pool_id) FILTER (WHERE pl.state = 'active')::BIGINT AS active_positions
        FROM predictions p
        JOIN pools pl ON pl.pool_id = p.pool_id
        WHERE p.user_address = $1
        "#,
    )
    .bind(address)
    .fetch_one(pool)
    .await?;

    let settled = row.wins + row.losses;
    let win_rate = if settled > 0 {
        (row.wins as f64 / settled as f64) * 100.0
    } else {
        0.0
    };

    Ok(ProfileStats {
        total_predictions: row.total_predictions,
        wins: row.wins,
        losses: row.losses,
        pending: row.pending,
        win_rate,
        total_staked: row.total_staked,
        total_earnings: row.total_earnings,
        active_positions: row.active_positions,
    })
}

/// Per-prediction claim status, joined with the owning pool's resolution/claim-window state.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ClaimStatusRow {
    /// Database identifier of the prediction.
    pub prediction_id: i64,
    /// Identifier of the pool containing the prediction.
    pub pool_id: i64,
    /// Display name of the pool.
    pub pool_name: String,
    /// Selected outcome index.
    pub outcome: i32,
    /// Amount staked on the prediction.
    pub amount: i64,
    /// Current lifecycle state of the pool.
    pub pool_state: String,
    /// Resolved pool result, when available.
    pub pool_result: Option<String>,
    /// Whether the prediction won, or `None` before resolution.
    pub is_winner: Option<bool>,
    /// Whether the prediction has been claimed.
    pub claimed: bool,
    /// Amount paid for the claim.
    pub claimed_amount: i64,
    /// End of the pool's claim window, when resolved.
    pub claim_window_expires_at: Option<DateTime<Utc>>,
    /// Whether the claim window has expired.
    pub claim_expired: bool,
}

/// Fetch claim status for every prediction `address` has made, newest first.
pub async fn get_user_claim_status(
    pool: &PgPool,
    address: &str,
) -> Result<Vec<ClaimStatusRow>, sqlx::Error> {
    sqlx::query_as::<_, ClaimStatusRow>(
        r#"
        SELECT
            p.id AS prediction_id,
            p.pool_id,
            pl.name AS pool_name,
            p.outcome,
            p.amount,
            pl.state AS pool_state,
            pl.result AS pool_result,
            CASE
                WHEN pl.result IS NOT NULL AND pl.result ~ '^\d+$'
                THEN pl.result::int = p.outcome
                ELSE NULL
            END AS is_winner,
            p.claimed,
            p.claimed_amount::BIGINT AS claimed_amount,
            (pl.resolved_at + (pl.claim_window_seconds || ' seconds')::interval) AS claim_window_expires_at,
            COALESCE(
                pl.resolved_at IS NOT NULL
                    AND pl.resolved_at + (pl.claim_window_seconds || ' seconds')::interval < NOW(),
                FALSE
            ) AS claim_expired
        FROM predictions p
        JOIN pools pl ON pl.pool_id = p.pool_id
        WHERE p.user_address = $1
        ORDER BY p.created_at DESC
        "#,
    )
    .bind(address)
    .fetch_all(pool)
    .await
}

/// One day's worth of activity, used to render cumulative performance charts.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PerformancePoint {
    /// Start of the UTC day represented by this point.
    pub day: DateTime<Utc>,
    /// Amount staked during the day.
    pub staked: i64,
    /// Amount earned during the day.
    pub earnings: i64,
    /// Number of predictions placed during the day.
    pub predictions: i64,
}

/// Daily staking/earnings activity for `address`, oldest first — the raw
/// series the frontend accumulates into cumulative performance charts.
pub async fn get_performance_over_time(
    pool: &PgPool,
    address: &str,
) -> Result<Vec<PerformancePoint>, sqlx::Error> {
    sqlx::query_as::<_, PerformancePoint>(
        r#"
        SELECT
            date_trunc('day', p.created_at) AS day,
            COALESCE(SUM(p.amount), 0)::BIGINT AS staked,
            COALESCE(SUM(p.claimed_amount) FILTER (WHERE p.claimed), 0)::BIGINT AS earnings,
            COUNT(*)::BIGINT AS predictions
        FROM predictions p
        WHERE p.user_address = $1
        GROUP BY 1
        ORDER BY 1
        "#,
    )
    .bind(address)
    .fetch_all(pool)
    .await
}

/// Full profile payload: aggregate stats, per-pool claim status, and the
/// daily activity series used to draw performance charts.
#[derive(Debug, Serialize)]
pub struct UserProfile {
    /// Stellar address whose profile was requested.
    pub address: String,
    /// Aggregate prediction statistics.
    pub stats: ProfileStats,
    /// Per-prediction claim status records.
    pub claims: Vec<ClaimStatusRow>,
    /// Daily activity points ordered oldest first.
    pub performance: Vec<PerformancePoint>,
}

/// Assemble the full profile payload for `GET /api/v1/users/:address/profile`.
pub async fn get_full_profile(pool: &PgPool, address: &str) -> Result<UserProfile, sqlx::Error> {
    let (stats, claims, performance) = tokio::try_join!(
        get_profile_stats(pool, address),
        get_user_claim_status(pool, address),
        get_performance_over_time(pool, address),
    )?;

    Ok(UserProfile {
        address: address.to_string(),
        stats,
        claims,
        performance,
    })
}

/// Mark every prediction row for `(pool_id, user_address)` as claimed.
///
/// The contract tracks claims per `(user, pool)`, not per individual stake
/// row, so all matching rows are flagged `claimed = true`; the paid-out
/// amount is recorded once (on the most recent row) so aggregate sums in
/// [`get_profile_stats`] aren't inflated by duplicate rows.
pub async fn mark_predictions_claimed(
    pool: &PgPool,
    pool_id: i64,
    user_address: &str,
    amount_paid: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        WITH ranked AS (
            SELECT id, ROW_NUMBER() OVER (ORDER BY created_at DESC) AS rn
            FROM predictions
            WHERE pool_id = $2 AND user_address = $3
        )
        UPDATE predictions p
        SET claimed = TRUE,
            claimed_amount = CASE WHEN ranked.rn = 1 THEN $1 ELSE 0 END,
            claimed_at = NOW()
        FROM ranked
        WHERE p.id = ranked.id
        "#,
    )
    .bind(amount_paid)
    .bind(pool_id)
    .bind(user_address)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_rate_is_zero_with_no_settled_predictions() {
        let row = ProfileStatsRow {
            total_predictions: 3,
            wins: 0,
            losses: 0,
            pending: 3,
            total_staked: 300,
            total_earnings: 0,
            active_positions: 3,
        };
        let settled = row.wins + row.losses;
        let win_rate = if settled > 0 {
            (row.wins as f64 / settled as f64) * 100.0
        } else {
            0.0
        };
        assert_eq!(win_rate, 0.0);
    }

    #[test]
    fn win_rate_reflects_settled_predictions_only() {
        let wins = 3i64;
        let losses = 1i64;
        let win_rate = (wins as f64 / (wins + losses) as f64) * 100.0;
        assert_eq!(win_rate, 75.0);
    }
}
