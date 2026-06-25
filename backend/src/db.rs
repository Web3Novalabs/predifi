use std::{future::Future, time::Duration};

use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::config::Config;

/// Create a PostgreSQL connection pool using conservative defaults.
///
/// This uses a retry loop on startup with exponential backoff, so transient
/// database downtime (e.g. container still starting) does not crash the service
/// immediately.
pub async fn create_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    let connect = || async {
        let future = PgPoolOptions::new()
            .max_connections(config.db_max_connections)
            .min_connections(config.db_min_connections)
            .acquire_timeout(Duration::from_secs(config.db_acquire_timeout_secs))
            .connect(&config.database_url);

        match tokio::time::timeout(Duration::from_secs(config.db_acquire_timeout_secs), future)
            .await
        {
            Ok(result) => result,
            Err(_) => Err(sqlx::Error::PoolTimedOut),
        }
    };

    retry_with_backoff(
        config.db_connect_max_attempts,
        config.db_connect_base_delay_ms,
        config.db_connect_max_delay_ms,
        "postgres",
        connect,
    )
    .await
}

fn backoff_delay_ms(attempt: u32, base_delay_ms: u64, max_delay_ms: u64) -> u64 {
    let exponent = attempt.saturating_sub(1).min(31);
    let delay = base_delay_ms.saturating_mul(1u64 << exponent);
    delay.min(max_delay_ms)
}

async fn retry_with_backoff<T, E, Fut, F>(
    max_attempts: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    target: &'static str,
    mut op: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let max_attempts = max_attempts.max(1);

    for attempt in 1..=max_attempts {
        match op().await {
            Ok(value) => {
                if attempt > 1 {
                    info!(
                        target,
                        attempts = attempt,
                        "connection established after retries"
                    );
                }
                return Ok(value);
            }
            Err(error) if attempt < max_attempts => {
                let delay_ms = backoff_delay_ms(attempt, base_delay_ms, max_delay_ms);
                warn!(
                    target,
                    attempt,
                    max_attempts,
                    delay_ms,
                    error = %error,
                    "startup connection attempt failed; retrying"
                );
                if delay_ms > 0 {
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
            Err(error) => {
                return Err(error);
            }
        }
    }

    unreachable!("loop covers attempt range")
}

/// A single row returned by the user prediction history query.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PredictionHistoryRow {
    pub pool_id: i64,
    pub pool_name: String,
    pub pool_result: Option<String>,
    pub outcome: i32,
    pub amount: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Enhanced prediction information with current pool status.
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

/// Fetch paginated prediction history for a given user address.
///
/// Joins the `predictions` table with the `pools` table to include the pool
/// name and result alongside each bet.
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

/// Raw row structure for user predictions query.
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

/// Fetch enhanced user predictions with current pool status and details.
///
/// Joins predictions with pools to provide comprehensive information about
/// each bet including current pool state, total stakes, and results.
pub async fn get_user_predictions(
    pool: &PgPool,
    address: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<UserPrediction>, sqlx::Error> {
    let sql = r#"
        SELECT
            p.id as prediction_id,
            p.pool_id,
            pl.name as pool_name,
            pl.category as pool_category,
            pl.state as pool_state,
            pl.end_time as pool_end_time,
            pl.total_stake as pool_total_stake,
            pl.result as pool_result,
            p.outcome as user_outcome,
            p.amount as user_amount,
            p.created_at as prediction_created_at
        FROM predictions p
        JOIN pools pl ON pl.pool_id = p.pool_id
        WHERE p.user_address = $1
        ORDER BY p.created_at DESC
        LIMIT $2 OFFSET $3
    "#;

    let rows = sqlx::query_as::<_, UserPredictionRow>(sql)
        .bind(address)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let predictions = rows
        .into_iter()
        .map(|row| {
            // Determine if this is a winning outcome
            let is_winning_outcome = match &row.pool_result {
                Some(result) => {
                    // Try to parse the result as an outcome number
                    result
                        .parse::<i32>()
                        .ok()
                        .map(|winning_outcome| winning_outcome == row.user_outcome)
                }
                None => None, // Pool not settled yet
            };

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
        .collect();

    Ok(predictions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    #[tokio::test]
    async fn retry_with_backoff_succeeds_after_transient_failures() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();
        let result: Result<&'static str, &'static str> =
            retry_with_backoff(5, 0, 0, "test", || async {
                let current = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 3 {
                    Err("nope")
                } else {
                    Ok("ok")
                }
            })
            .await;

        assert_eq!(result, Ok("ok"));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retry_with_backoff_gives_up_after_max_attempts() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();
        let result: Result<(), &'static str> = retry_with_backoff(3, 0, 0, "test", || async {
            attempts_clone.fetch_add(1, Ordering::SeqCst);
            Err("still down")
        })
        .await;

        assert_eq!(result, Err("still down"));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn backoff_delay_is_exponential_and_capped() {
        assert_eq!(backoff_delay_ms(1, 200, 5_000), 200);
        assert_eq!(backoff_delay_ms(2, 200, 5_000), 400);
        assert_eq!(backoff_delay_ms(3, 200, 5_000), 800);
        assert_eq!(backoff_delay_ms(10, 200, 5_000), 5_000);
    }
}

/// A single row returned by the active pools query.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PoolRow {
    pub pool_id: i64,
    pub name: String,
    pub category: String,
    pub total_stake: i64,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Detailed pool information including metadata.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PoolDetails {
    pub pool_id: i64,
    pub name: String,
    pub category: String,
    pub total_stake: i64,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub state: String,
    pub creator: String,
    pub token: String,
    pub result: Option<String>,
}

/// Outcome odds information.
#[derive(Debug, serde::Serialize)]
pub struct OutcomeOdds {
    pub outcome: i32,
    pub stake: i64,
    pub odds: f64,
}

/// Complete pool information with odds.
#[derive(Debug, serde::Serialize)]
pub struct PoolWithOdds {
    pub pool_id: i64,
    pub name: String,
    pub category: String,
    pub total_stake: i64,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub state: String,
    pub creator: String,
    pub token: String,
    pub result: Option<String>,
    pub odds: Vec<OutcomeOdds>,
}

/// User ranking by betting volume.
#[derive(Debug, serde::Serialize)]
pub struct UserBettingVolume {
    pub user_address: String,
    pub total_volume: i64,
    pub prediction_count: i64,
    pub rank: i64,
}

/// User ranking by winnings.
#[derive(Debug, serde::Serialize)]
pub struct UserWinnings {
    pub user_address: String,
    pub total_winnings: i64,
    pub winning_predictions: i64,
    pub total_predictions: i64,
    pub win_rate: f64,
    pub rank: i64,
}

/// Fetch active pools with optional category filter and sort order.
///
/// `sort_by` accepts `"popular"`, `"ending_soon"`, or `"new"`.
pub async fn get_active_pools(
    pool: &PgPool,
    sort_by: &str,
    category: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<PoolRow>, sqlx::Error> {
    get_pools_with_filters(pool, sort_by, category, "active", limit, offset).await
}

/// Fetch pools with optional category, status filters and sort order.
///
/// `sort_by` accepts `"popular"`, `"ending_soon"`, or `"new"`.
/// `status` accepts `"active"`, `"closed"`, or `"settled"`.
pub async fn get_pools_with_filters(
    pool: &PgPool,
    sort_by: &str,
    category: Option<&str>,
    status: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<PoolRow>, sqlx::Error> {
    // Build ORDER BY clause from sort_by parameter.
    let order_clause = match sort_by {
        "popular" => "total_stake DESC",
        "ending_soon" => "end_time ASC",
        _ => "created_at DESC", // "new" and default
    };

    // Validate status parameter to prevent SQL injection
    let valid_status = match status {
        "active" | "closed" | "settled" => status,
        _ => "active", // default to active for invalid status
    };

    // sqlx doesn't support dynamic ORDER BY via bind params, so we build the
    // query string manually. The order_clause is constructed from a controlled
    // match arm — no user input reaches it directly.
    let sql = format!(
        r#"
        SELECT pool_id, name, category, total_stake, end_time, created_at
        FROM pools
        WHERE state = $1
          AND ($2::text IS NULL OR category = $2)
        ORDER BY {order_clause}
        LIMIT $3 OFFSET $4
        "#
    );

    sqlx::query_as::<_, PoolRow>(&sql)
        .bind(valid_status)
        .bind(category)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

/// Count total number of pools matching the filters.
pub async fn count_pools_with_filters(
    pool: &PgPool,
    category: Option<&str>,
    status: &str,
) -> Result<i64, sqlx::Error> {
    // Validate status parameter to prevent SQL injection
    let valid_status = match status {
        "active" | "closed" | "settled" => status,
        _ => "active", // default to active for invalid status
    };

    let sql = r#"
        SELECT COUNT(*)
        FROM pools
        WHERE state = $1
          AND ($2::text IS NULL OR category = $2)
        "#;

    let count: (i64,) = sqlx::query_as(sql)
        .bind(valid_status)
        .bind(category)
        .fetch_one(pool)
        .await?;

    Ok(count.0)
}

/// Fetch detailed information for a specific pool by ID.
pub async fn get_pool_by_id(
    pool: &PgPool,
    pool_id: i64,
) -> Result<Option<PoolDetails>, sqlx::Error> {
    let sql = r#"
        SELECT pool_id, name, category, total_stake, end_time, created_at, 
               state, creator, token, result
        FROM pools
        WHERE pool_id = $1
    "#;

    sqlx::query_as::<_, PoolDetails>(sql)
        .bind(pool_id)
        .fetch_optional(pool)
        .await
}

/// Raw row for outcome stakes query.
#[derive(sqlx::FromRow)]
struct OutcomeStakeRow {
    outcome: i32,
    total_stake: i64,
}

/// Fetch outcome stakes for a specific pool to calculate odds.
pub async fn get_pool_outcome_stakes(
    pool: &PgPool,
    pool_id: i64,
) -> Result<Vec<(i32, i64)>, sqlx::Error> {
    let sql = r#"
        SELECT outcome, COALESCE(SUM(amount), 0) as total_stake
        FROM predictions
        WHERE pool_id = $1
        GROUP BY outcome
        ORDER BY outcome
    "#;

    let rows = sqlx::query_as::<_, OutcomeStakeRow>(sql)
        .bind(pool_id)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.outcome, row.total_stake))
        .collect())
}

/// Calculate odds for each outcome based on stakes.
/// Formula: odds = 1.0 / (outcome_stake / total_stake)
/// If outcome_stake is 0, odds are set to 0.0
pub fn calculate_odds(outcome_stakes: &[(i32, i64)], total_stake: i64) -> Vec<OutcomeOdds> {
    if total_stake == 0 {
        return outcome_stakes
            .iter()
            .map(|(outcome, stake)| OutcomeOdds {
                outcome: *outcome,
                stake: *stake,
                odds: 0.0,
            })
            .collect();
    }

    outcome_stakes
        .iter()
        .map(|(outcome, stake)| {
            let odds = if *stake == 0 {
                0.0
            } else {
                1.0 / (*stake as f64 / total_stake as f64)
            };
            OutcomeOdds {
                outcome: *outcome,
                stake: *stake,
                odds,
            }
        })
        .collect()
}

/// Fetch complete pool information with real-time odds calculation.
pub async fn get_pool_with_odds(
    pool: &PgPool,
    pool_id: i64,
) -> Result<Option<PoolWithOdds>, sqlx::Error> {
    // Fetch pool details
    let pool_details = match get_pool_by_id(pool, pool_id).await? {
        Some(details) => details,
        None => return Ok(None),
    };

    // Fetch outcome stakes
    let outcome_stakes = get_pool_outcome_stakes(pool, pool_id).await?;

    // Calculate total stake from predictions (this might differ from pool.total_stake)
    let calculated_total_stake: i64 = outcome_stakes.iter().map(|(_, stake)| stake).sum();

    // Use the higher of the two totals (pool.total_stake or calculated)
    let total_stake = std::cmp::max(pool_details.total_stake, calculated_total_stake);

    // Calculate odds
    let odds = calculate_odds(&outcome_stakes, total_stake);

    Ok(Some(PoolWithOdds {
        pool_id: pool_details.pool_id,
        name: pool_details.name,
        category: pool_details.category,
        total_stake,
        end_time: pool_details.end_time,
        created_at: pool_details.created_at,
        state: pool_details.state,
        creator: pool_details.creator,
        token: pool_details.token,
        result: pool_details.result,
        odds,
    }))
}

/// Raw row structure for user betting volume query.
#[derive(sqlx::FromRow)]
struct UserVolumeRow {
    user_address: String,
    total_volume: i64,
    prediction_count: i64,
}

/// Get top users ranked by total betting volume.
pub async fn get_users_by_betting_volume(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<UserBettingVolume>, sqlx::Error> {
    let sql = r#"
        SELECT 
            user_address,
            SUM(amount) as total_volume,
            COUNT(*) as prediction_count
        FROM predictions
        GROUP BY user_address
        ORDER BY SUM(amount) DESC
        LIMIT $1 OFFSET $2
    "#;

    let rows = sqlx::query_as::<_, UserVolumeRow>(sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let rankings = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| UserBettingVolume {
            user_address: row.user_address,
            total_volume: row.total_volume,
            prediction_count: row.prediction_count,
            rank: (offset + index as i64 + 1),
        })
        .collect();

    Ok(rankings)
}

/// Raw row structure for user winnings query.
#[derive(sqlx::FromRow)]
struct UserWinningsRow {
    user_address: String,
    total_winnings: i64,
    winning_predictions: i64,
    total_predictions: i64,
}

/// Get top users ranked by total winnings from settled pools.
///
/// This calculates winnings based on a simplified model where winners
/// split the total pool proportionally to their stakes.
pub async fn get_users_by_winnings(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<UserWinnings>, sqlx::Error> {
    let sql = r#"
        WITH winning_predictions AS (
            SELECT 
                p.user_address,
                p.amount,
                pl.total_stake,
                pl.pool_id
            FROM predictions p
            JOIN pools pl ON pl.pool_id = p.pool_id
            WHERE pl.state = 'settled' 
              AND pl.result IS NOT NULL
              AND p.outcome = CAST(pl.result AS INTEGER)
        ),
        pool_winning_totals AS (
            SELECT
                pool_id,
                SUM(amount) AS winning_stake
            FROM winning_predictions
            GROUP BY pool_id
        ),
        user_winnings AS (
            SELECT
                wp.user_address,
                SUM(wp.amount * (wp.total_stake::FLOAT / pwt.winning_stake)) as total_winnings,
                COUNT(*) as winning_predictions
            FROM winning_predictions wp
            JOIN pool_winning_totals pwt ON pwt.pool_id = wp.pool_id
            GROUP BY wp.user_address
        ),
        user_totals AS (
            SELECT 
                p.user_address,
                COUNT(*) as total_predictions
            FROM predictions p
            JOIN pools pl ON pl.pool_id = p.pool_id
            WHERE pl.state = 'settled'
            GROUP BY p.user_address
        )
        SELECT 
            COALESCE(uw.user_address, ut.user_address) as user_address,
            COALESCE(uw.total_winnings, 0) as total_winnings,
            COALESCE(uw.winning_predictions, 0) as winning_predictions,
            ut.total_predictions
        FROM user_winnings uw
        FULL OUTER JOIN user_totals ut ON uw.user_address = ut.user_address
        WHERE ut.total_predictions > 0
        ORDER BY COALESCE(uw.total_winnings, 0) DESC
        LIMIT $1 OFFSET $2
    "#;

    let rows = sqlx::query_as::<_, UserWinningsRow>(sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let rankings = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let win_rate = if row.total_predictions > 0 {
                row.winning_predictions as f64 / row.total_predictions as f64
            } else {
                0.0
            };

            UserWinnings {
                user_address: row.user_address,
                total_winnings: row.total_winnings,
                winning_predictions: row.winning_predictions,
                total_predictions: row.total_predictions,
                win_rate,
                rank: (offset + index as i64 + 1),
            }
        })
        .collect();

    Ok(rankings)
}

/// Mark a pool as settled and record the winning outcome.
pub async fn resolve_pool_in_db(
    pool: &PgPool,
    pool_id: u64,
    winning_outcome: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE pools SET state = 'settled', result = $1 WHERE pool_id = $2")
        .bind(winning_outcome.to_string())
        .bind(pool_id as i64)
        .execute(pool)
        .await?;
    Ok(())
}

/// Mark a pool as closed (cancelled on-chain).
pub async fn cancel_pool_in_db(pool: &PgPool, pool_id: u64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE pools SET state = 'closed' WHERE pool_id = $1")
        .bind(pool_id as i64)
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert a new pool record decoded from a `PoolCreated` contract event.
pub async fn insert_pool_from_event(
    pool: &PgPool,
    event: &PoolCreatedEvent,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO pools (pool_id, name, category, total_stake, end_time, state, creator, token, created_at)
        VALUES ($1, $2, $3, 0, to_timestamp($4), 'active', $5, $6, NOW())
        ON CONFLICT (pool_id) DO NOTHING
        "#,
    )
    .bind(event.pool_id as i64)
    .bind(&event.description)
    .bind(&event.category)
    .bind(event.end_time as f64)
    .bind(&event.creator)
    .bind(&event.token)
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert a decoded `prediction_placed` contract event into the prediction index.
pub async fn insert_prediction_from_event(
    pool: &PgPool,
    event: &PredictionPlacedEvent,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

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
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE pools SET total_stake = total_stake + $1 WHERE pool_id = $2")
        .bind(event.amount)
        .bind(event.pool_id as i64)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// A single row in the referral earnings breakdown — one entry per pool.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ReferralEarningRow {
    pub pool_id: i64,
    pub pool_name: String,
    pub total_earned: i64,
    pub referral_count: i64,
}

/// Fetch referral earnings grouped by pool for a given referrer address.
pub async fn get_referral_earnings(
    pool: &PgPool,
    address: &str,
) -> Result<Vec<ReferralEarningRow>, sqlx::Error> {
    sqlx::query_as::<_, ReferralEarningRow>(
        r#"
        SELECT
            r.pool_id,
            pl.name          AS pool_name,
            COALESCE(SUM(r.amount), 0) AS total_earned,
            COUNT(r.id)               AS referral_count
        FROM referrals r
        JOIN pools pl ON pl.pool_id = r.pool_id
        WHERE r.referrer = $1
        GROUP BY r.pool_id, pl.name
        ORDER BY SUM(r.amount) DESC
        "#,
    )
    .bind(address)
    .fetch_all(pool)
    .await
}

/// Protocol-wide aggregate statistics.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolStats {
    /// Sum of `total_stake` across all pools (TVL proxy).
    pub total_value_locked: i64,
    /// Total number of prediction records (bets placed).
    pub total_bets: i64,
    /// Total number of pools ever created.
    pub total_pools: i64,
}

/// Fetch protocol-wide aggregate statistics in a single query.
pub async fn get_protocol_stats(pool: &PgPool) -> Result<ProtocolStats, sqlx::Error> {
    sqlx::query_as::<_, ProtocolStats>(
        r#"
        SELECT
            COALESCE(SUM(total_stake), 0)       AS total_value_locked,
            (SELECT COUNT(*) FROM predictions)  AS total_bets,
            COUNT(*)                            AS total_pools
        FROM pools
        "#,
    )
    .fetch_one(pool)
    .await
}

/// Decoded data from a `pool_created` contract event.
#[derive(Debug)]
pub struct PoolCreatedEvent {
    pub pool_id: u64,
    pub creator: String,
    pub end_time: u64,
    pub token: String,
    pub category: String,
    pub description: String,
}

/// Decoded data from a `prediction_placed` contract event.
pub struct PredictionPlacedEvent {
    pub pool_id: u64,
    pub user_address: String,
    pub outcome: i32,
    pub amount: i64,
}

/// Decoded data from a `referral_paid` contract event.
#[derive(Debug)]
pub struct ReferralPaidEvent {
    pub pool_id: u64,
    pub referrer: String,
    pub referred_user: String,
    pub referral_amount: i64,
}

/// Insert multiple referral records using bulk insert for optimal performance.
///
/// This function uses PostgreSQL's `INSERT INTO ... VALUES (...), (...), ...` syntax
/// to insert all referral records in a single database round-trip, significantly
/// improving performance over individual inserts when processing multiple events.
pub async fn insert_referrals_bulk(
    pool: &PgPool,
    events: &[ReferralPaidEvent],
) -> Result<(), sqlx::Error> {
    if events.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;

    // Build bulk insert query with dynamic values
    let query = r#"
        INSERT INTO referrals (referrer, user_address, pool_id, amount)
        VALUES 
    "#;

    let mut values = Vec::new();
    let mut param_index = 1i32;

    for _event in events {
        values.push(format!(
            "(${}, ${}, ${}, ${})",
            param_index,
            param_index + 1,
            param_index + 2,
            param_index + 3
        ));
        param_index += 4;
    }

    let full_query = format!("{} {}", query, values.join(", "));

    let mut query_builder = sqlx::query(&full_query);

    for event in events {
        query_builder = query_builder
            .bind(&event.referrer)
            .bind(&event.referred_user)
            .bind(event.pool_id as i64)
            .bind(event.referral_amount);
    }

    query_builder.execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(())
}

/// Insert a single referral record.
///
/// For inserting multiple referrals, use `insert_referrals_bulk` instead for better performance.
pub async fn insert_referral_from_event(
    pool: &PgPool,
    event: &ReferralPaidEvent,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO referrals (referrer, user_address, pool_id, amount)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(&event.referrer)
    .bind(&event.referred_user)
    .bind(event.pool_id as i64)
    .bind(event.referral_amount)
    .execute(pool)
    .await?;
    Ok(())
}

// Tests live near the top-level helpers (see `retry_with_backoff` tests).
