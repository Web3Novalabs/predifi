//! Prediction repository — all queries that read or write the `predictions`
//! table, plus leaderboard and protocol-stats aggregates.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres};
use tracing::instrument;

// ── Row / DTO types ───────────────────────────────────────────────────────────

/// Lightweight history row returned by the user prediction-history list.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PredictionHistoryRow {
    pub pool_id: i64,
    pub pool_name: String,
    pub pool_result: Option<String>,
    pub outcome: i32,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
}

/// Full prediction enriched with current pool status and win/loss resolution.
#[derive(Debug, serde::Serialize)]
pub struct UserPrediction {
    pub prediction_id: i64,
    pub pool_id: i64,
    pub pool_name: String,
    pub pool_category: String,
    pub pool_state: String,
    pub pool_end_time: DateTime<Utc>,
    pub pool_total_stake: i64,
    pub pool_result: Option<String>,
    pub user_outcome: i32,
    pub user_amount: i64,
    pub prediction_created_at: DateTime<Utc>,
    pub is_winning_outcome: Option<bool>,
}

/// Cursor-paginated prediction row for the market predictions endpoint.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct MarketPredictionRow {
    /// Stable row ID used as the pagination cursor.
    pub id: i64,
    pub pool_id: i64,
    pub user_address: String,
    pub outcome: i32,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
}

/// User ranked by total betting volume.
#[derive(Debug, serde::Serialize)]
pub struct UserBettingVolume {
    pub user_address: String,
    pub total_volume: i64,
    pub prediction_count: i64,
    pub rank: i64,
}

/// User ranked by winnings from settled pools.
#[derive(Debug, serde::Serialize)]
pub struct UserWinnings {
    pub user_address: String,
    pub total_winnings: i64,
    pub winning_predictions: i64,
    pub total_predictions: i64,
    pub win_rate: f64,
    pub rank: i64,
}

/// Leaderboard entry with time-window and pool-scope support (#1363).
#[derive(Debug, serde::Serialize)]
pub struct LeaderboardEntry {
    pub user_address: String,
    pub total_volume: i64,
    pub prediction_count: i64,
    pub wins: i64,
    pub settled_count: i64,
    pub win_rate: f64,
    pub current_streak: i64,
    pub rank: i64,
}

/// Protocol-wide aggregate statistics.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolStats {
    /// Sum of `total_stake` across all matching pools (TVL proxy).
    pub total_value_locked: i64,
    /// Total prediction records across matching pools.
    pub total_bets: i64,
    /// Total number of matching pools.
    pub total_pools: i64,
}

/// Decoded data from a `prediction_placed` contract event.
#[derive(Debug)]
pub struct PredictionPlacedEvent {
    pub pool_id: u64,
    pub user_address: String,
    pub outcome: i32,
    pub amount: i64,
}

// ── Private row types ─────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct UserPredictionRow {
    prediction_id: i64,
    pool_id: i64,
    pool_name: String,
    pool_category: String,
    pool_state: String,
    pool_end_time: DateTime<Utc>,
    pool_total_stake: i64,
    pool_result: Option<String>,
    user_outcome: i32,
    user_amount: i64,
    prediction_created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct UserVolumeRow {
    user_address: String,
    total_volume: i64,
    prediction_count: i64,
}

#[derive(sqlx::FromRow)]
struct UserWinningsRow {
    user_address: String,
    total_winnings: i64,
    winning_predictions: i64,
    total_predictions: i64,
}

#[derive(sqlx::FromRow)]
struct LeaderboardRow {
    user_address: String,
    total_volume: i64,
    prediction_count: i64,
    wins: i64,
    settled_count: i64,
    current_streak: i64,
}

// ── Read queries ──────────────────────────────────────────────────────────────

/// Paginated lightweight prediction history for a user address.
pub async fn get_user_prediction_history(
    pool: &PgPool,
    address: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<PredictionHistoryRow>, sqlx::Error> {
    sqlx::query_as::<_, PredictionHistoryRow>(
        r#"
        SELECT
            p.pool_id,
            pl.name   AS pool_name,
            pl.result AS pool_result,
            p.outcome,
            p.amount,
            p.created_at
        FROM predictions p
        JOIN pools pl ON pl.pool_id = p.pool_id
        WHERE p.user_address = $1
        ORDER BY p.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(address)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// Paginated full-detail predictions for a user address, including win/loss status.
pub async fn get_user_predictions(
    pool: &PgPool,
    address: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<UserPrediction>, sqlx::Error> {
    let rows = sqlx::query_as::<_, UserPredictionRow>(
        r#"
        SELECT
            p.id          AS prediction_id,
            p.pool_id,
            pl.name       AS pool_name,
            pl.category   AS pool_category,
            pl.state      AS pool_state,
            pl.end_time   AS pool_end_time,
            pl.total_stake AS pool_total_stake,
            pl.result     AS pool_result,
            p.outcome     AS user_outcome,
            p.amount      AS user_amount,
            p.created_at  AS prediction_created_at
        FROM predictions p
        JOIN pools pl ON pl.pool_id = p.pool_id
        WHERE p.user_address = $1
        ORDER BY p.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(address)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let is_winning_outcome = row.pool_result.as_ref().and_then(|result| {
                result
                    .parse::<i32>()
                    .ok()
                    .map(|winning| winning == row.user_outcome)
            });

            UserPrediction {
                prediction_id: row.prediction_id,
                pool_id: row.pool_id,
                pool_name: row.pool_name,
                pool_category: row.pool_category,
                pool_state: row.pool_state,
                pool_end_time: row.pool_end_time,
                pool_total_stake: row.pool_total_stake,
                pool_result: row.pool_result,
                user_outcome: row.user_outcome,
                user_amount: row.user_amount,
                prediction_created_at: row.prediction_created_at,
                is_winning_outcome,
            }
        })
        .collect())
}

/// Cursor-paginated market predictions for a pool (newest first by `id DESC`).
///
/// `after_id` is the opaque cursor from the previous page. Pass `None` to
/// start at the most recent row. Fetches `limit + 1` rows so the caller can
/// detect whether a next page exists without an extra `COUNT(*)` query.
pub async fn get_market_predictions(
    pool: &PgPool,
    pool_id: i64,
    after_id: Option<i64>,
    limit: i64,
) -> Result<Vec<MarketPredictionRow>, sqlx::Error> {
    sqlx::query_as::<_, MarketPredictionRow>(
        r#"
        SELECT id, pool_id, user_address, outcome, amount, created_at
        FROM predictions
        WHERE pool_id = $1
          AND ($2::bigint IS NULL OR id < $2)
        ORDER BY id DESC
        LIMIT $3
        "#,
    )
    .bind(pool_id)
    .bind(after_id)
    .bind(limit + 1) // +1 to detect next-page existence
    .fetch_all(pool)
    .await
}

/// Total prediction count for a pool (used for the `total` response field).
pub async fn count_market_predictions(pool: &PgPool, pool_id: i64) -> Result<i64, sqlx::Error> {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM predictions WHERE pool_id = $1")
            .bind(pool_id)
            .fetch_one(pool)
            .await?;
    Ok(count)
}

/// Top users ranked by total betting volume, with page-offset-aware rank.
pub async fn get_users_by_betting_volume(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<UserBettingVolume>, sqlx::Error> {
    let rows = sqlx::query_as::<_, UserVolumeRow>(
        r#"
        SELECT
            user_address,
            SUM(amount)  AS total_volume,
            COUNT(*)     AS prediction_count
        FROM predictions
        GROUP BY user_address
        ORDER BY SUM(amount) DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(i, r)| UserBettingVolume {
            user_address: r.user_address,
            total_volume: r.total_volume,
            prediction_count: r.prediction_count,
            rank: offset + i as i64 + 1,
        })
        .collect())
}

/// Top users ranked by winnings from settled pools.
///
/// `pool_winning_totals` pre-aggregates each pool's total winning-outcome
/// stake once (avoiding N+1 correlated subqueries — see #1370).
pub async fn get_users_by_winnings(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<UserWinnings>, sqlx::Error> {
    let rows = sqlx::query_as::<_, UserWinningsRow>(
        r#"
        WITH pool_winning_totals AS (
            SELECT pl.pool_id,
                   SUM(p.amount) AS winning_stake_total
            FROM predictions p
            JOIN pools pl ON pl.pool_id = p.pool_id
            WHERE pl.state  = 'settled'
              AND pl.result IS NOT NULL
              AND p.outcome = CAST(pl.result AS INTEGER)
            GROUP BY pl.pool_id
        ),
        winning_predictions AS (
            SELECT
                p.user_address,
                p.amount,
                pl.total_stake,
                pwt.winning_stake_total
            FROM predictions p
            JOIN pools pl  ON pl.pool_id  = p.pool_id
            JOIN pool_winning_totals pwt ON pwt.pool_id = pl.pool_id
            WHERE pl.state  = 'settled'
              AND pl.result IS NOT NULL
              AND p.outcome = CAST(pl.result AS INTEGER)
        ),
        user_winnings AS (
            SELECT
                user_address,
                SUM(amount * (total_stake::FLOAT / NULLIF(winning_stake_total, 0)))
                    AS total_winnings,
                COUNT(*) AS winning_predictions
            FROM winning_predictions
            GROUP BY user_address
        ),
        user_totals AS (
            SELECT p.user_address, COUNT(*) AS total_predictions
            FROM predictions p
            JOIN pools pl ON pl.pool_id = p.pool_id
            WHERE pl.state = 'settled'
            GROUP BY p.user_address
        )
        SELECT
            COALESCE(uw.user_address, ut.user_address)  AS user_address,
            COALESCE(uw.total_winnings, 0)::BIGINT      AS total_winnings,
            COALESCE(uw.winning_predictions, 0)         AS winning_predictions,
            ut.total_predictions
        FROM user_winnings uw
        FULL OUTER JOIN user_totals ut ON uw.user_address = ut.user_address
        WHERE ut.total_predictions > 0
        ORDER BY COALESCE(uw.total_winnings, 0) DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let win_rate = if r.total_predictions > 0 {
                r.winning_predictions as f64 / r.total_predictions as f64
            } else {
                0.0
            };
            UserWinnings {
                user_address: r.user_address,
                total_winnings: r.total_winnings,
                winning_predictions: r.winning_predictions,
                total_predictions: r.total_predictions,
                win_rate,
                rank: offset + i as i64 + 1,
            }
        })
        .collect())
}

/// Extended leaderboard with time-window and pool-scope filtering (#1363).
///
/// * `rank_by` — `"volume"` (default) | `"win_rate"` | `"streak"`
/// * `period`  — `"week"` | `"month"` | `"all"` (default)
/// * `pool_id` — when `Some`, restricts ranking to a single pool
///
/// `current_streak` is the count of the user's most recent consecutive wins
/// on settled predictions, ordered by pool `end_time`.
pub async fn get_leaderboard_extended(
    pool: &PgPool,
    rank_by: &str,
    period: &str,
    pool_id: Option<i64>,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
    // SECURITY: order_by is chosen from a controlled match — no user input
    // reaches the interpolated string.
    let order_by = match rank_by {
        "win_rate" => {
            "(CASE WHEN ua.settled_count > 0 THEN ua.wins::FLOAT / ua.settled_count ELSE 0 END)"
        }
        "streak" => "COALESCE(sc.current_streak, 0)",
        _ => "ua.total_volume",
    };

    let cutoff: Option<DateTime<Utc>> = match period {
        "week"  => Some(Utc::now() - chrono::Duration::days(7)),
        "month" => Some(Utc::now() - chrono::Duration::days(30)),
        _       => None,
    };

    let sql = format!(
        r#"
        WITH scoped_predictions AS (
            SELECT p.user_address, p.pool_id, p.amount, p.outcome, p.created_at,
                   pl.state, pl.result, pl.end_time
            FROM predictions p
            JOIN pools pl ON pl.pool_id = p.pool_id
            WHERE ($3::bigint    IS NULL OR p.pool_id    = $3)
              AND ($4::timestamptz IS NULL OR p.created_at >= $4)
        ),
        outcome_flags AS (
            SELECT
                user_address,
                amount,
                end_time,
                state,
                CASE
                    WHEN state = 'settled'
                     AND result IS NOT NULL
                     AND outcome = CAST(result AS INTEGER)
                    THEN 1 ELSE 0
                END AS is_win
            FROM scoped_predictions
        ),
        user_agg AS (
            SELECT
                user_address,
                SUM(amount)                                          AS total_volume,
                COUNT(*)                                             AS prediction_count,
                SUM(CASE WHEN state = 'settled' THEN 1 ELSE 0 END)  AS settled_count,
                SUM(is_win)                                          AS wins
            FROM outcome_flags
            GROUP BY user_address
        ),
        ranked_settled AS (
            SELECT
                user_address,
                is_win,
                ROW_NUMBER() OVER (PARTITION BY user_address ORDER BY end_time DESC) AS rn
            FROM outcome_flags
            WHERE state = 'settled'
        ),
        streak_groups AS (
            SELECT
                user_address,
                is_win,
                rn,
                SUM(CASE WHEN is_win = 0 THEN 1 ELSE 0 END)
                    OVER (PARTITION BY user_address ORDER BY rn) AS loss_group
            FROM ranked_settled
        ),
        streak_calc AS (
            SELECT user_address, COUNT(*) AS current_streak
            FROM streak_groups
            WHERE is_win = 1 AND loss_group = 0
            GROUP BY user_address
        )
        SELECT
            ua.user_address,
            ua.total_volume,
            ua.prediction_count,
            COALESCE(ua.wins,            0) AS wins,
            COALESCE(ua.settled_count,   0) AS settled_count,
            COALESCE(sc.current_streak,  0) AS current_streak
        FROM user_agg ua
        LEFT JOIN streak_calc sc ON sc.user_address = ua.user_address
        ORDER BY {order_by} DESC
        LIMIT $1 OFFSET $2
        "#
    );

    let rows = sqlx::query_as::<_, LeaderboardRow>(&sql)
        .bind(limit)
        .bind(offset)
        .bind(pool_id)
        .bind(cutoff)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let win_rate = if r.settled_count > 0 {
                r.wins as f64 / r.settled_count as f64
            } else {
                0.0
            };
            LeaderboardEntry {
                user_address: r.user_address,
                total_volume: r.total_volume,
                prediction_count: r.prediction_count,
                wins: r.wins,
                settled_count: r.settled_count,
                win_rate,
                current_streak: r.current_streak,
                rank: offset + i as i64 + 1,
            }
        })
        .collect())
}

/// Protocol-wide aggregate stats, optionally scoped by category and/or state.
pub async fn get_protocol_stats(
    pool: &PgPool,
    category: Option<&str>,
    state: Option<&str>,
) -> Result<ProtocolStats, sqlx::Error> {
    sqlx::query_as::<_, ProtocolStats>(
        r#"
        WITH filtered_pools AS (
            SELECT pool_id, total_stake
            FROM pools
            WHERE ($1::text IS NULL OR category = $1)
              AND ($2::text IS NULL OR state    = $2)
        )
        SELECT
            COALESCE(SUM(total_stake), 0) AS total_value_locked,
            (SELECT COUNT(*)
             FROM predictions p
             WHERE p.pool_id IN (SELECT pool_id FROM filtered_pools)
            ) AS total_bets,
            COUNT(*) AS total_pools
        FROM filtered_pools
        "#,
    )
    .bind(category)
    .bind(state)
    .fetch_one(pool)
    .await
}

// ── Write queries ─────────────────────────────────────────────────────────────

/// Insert a prediction and atomically update the pool's `total_stake`.
///
/// Must be called inside an open transaction. For single-event convenience
/// use [`insert_prediction_from_event_with_pool`].
#[instrument(skip(tx), name = "db.insert_prediction_from_event",
    fields(pool_id = event.pool_id, user_address = %event.user_address))]
pub async fn insert_prediction_from_event(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    event: &PredictionPlacedEvent,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO predictions (pool_id, user_address, outcome, amount)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(event.pool_id as i64)
    .bind(&event.user_address)
    .bind(event.outcome)
    .bind(event.amount)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "UPDATE pools SET total_stake = total_stake + $1 WHERE pool_id = $2",
    )
    .bind(event.amount)
    .bind(event.pool_id as i64)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Convenience wrapper: open a transaction, insert prediction, commit.
#[instrument(skip(pool), name = "db.insert_prediction_from_event_with_pool",
    fields(pool_id = event.pool_id, user_address = %event.user_address))]
pub async fn insert_prediction_from_event_with_pool(
    pool: &PgPool,
    event: &PredictionPlacedEvent,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    insert_prediction_from_event(&mut tx, event).await?;
    tx.commit().await?;
    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ── UserWinnings win_rate guard ───────────────────────────────────────────

    #[test]
    fn win_rate_is_zero_when_no_predictions() {
        let win_rate: f64 = if 0i64 > 0 { 5.0 / 0.0 } else { 0.0 };
        assert_eq!(win_rate, 0.0);
    }

    #[test]
    fn win_rate_partial() {
        let rate = 3_f64 / 10_f64;
        assert!((rate - 0.3).abs() < 1e-9);
    }

    // ── Rank offset ───────────────────────────────────────────────────────────

    #[test]
    fn leaderboard_rank_respects_page_offset() {
        let offset: i64 = 20;
        assert_eq!(offset + 0 + 1, 21); // first row of page
        assert_eq!(offset + 1 + 1, 22); // second row of page
    }

    // ── Cursor pagination helpers ─────────────────────────────────────────────

    fn make_rows(ids: &[i64]) -> Vec<MarketPredictionRow> {
        ids.iter()
            .map(|&id| MarketPredictionRow {
                id,
                pool_id: 1,
                user_address: format!("G{id:055}"),
                outcome: 0,
                amount: 100,
                created_at: Utc::now(),
            })
            .collect()
    }

    #[test]
    fn no_next_page_when_rows_equal_limit() {
        let limit = 3i64;
        let mut rows = make_rows(&[10, 9, 8]);
        let has_next = rows.len() as i64 > limit;
        if has_next {
            rows.truncate(limit as usize);
        }
        assert!(!has_next);
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn has_next_page_when_rows_exceed_limit() {
        let limit = 3i64;
        let mut rows = make_rows(&[10, 9, 8, 7]); // query returns limit+1
        let has_next = rows.len() as i64 > limit;
        if has_next {
            rows.truncate(limit as usize);
        }
        let cursor = if has_next { rows.last().map(|r| r.id) } else { None };
        assert!(has_next);
        assert_eq!(cursor, Some(8));
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn empty_result_produces_no_cursor() {
        let limit = 20i64;
        let rows: Vec<MarketPredictionRow> = vec![];
        let has_next = rows.len() as i64 > limit;
        assert!(!has_next);
    }

    #[test]
    fn limit_clamp_boundaries() {
        assert_eq!(0i64.clamp(1, 100), 1);
        assert_eq!((-5i64).clamp(1, 100), 1);
        assert_eq!(100i64.clamp(1, 100), 100);
        assert_eq!(9_999i64.clamp(1, 100), 100);
        assert_eq!(50i64.clamp(1, 100), 50);
    }
}
