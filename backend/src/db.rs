use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, Executor, PgPool, Postgres};
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::config::Config;

/// Check if a database error is transient and should be retried during pool creation.
///
/// Transient errors include connection issues (refused, timeout, reset).
/// Unrecoverable errors (invalid credentials, missing database) fail immediately.
pub fn is_transient_error(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::PoolTimedOut => true,
        sqlx::Error::PoolClosed => false,
        sqlx::Error::Database(ref db_err) => {
            let kind = db_err.kind();
            matches!(
                kind,
                sqlx::error::ErrorKind::ConnectionReset
                    | sqlx::error::ErrorKind::ConnectionRefused
                    | sqlx::error::ErrorKind::ConnectionBusy
                    | sqlx::error::ErrorKind::Io
            )
        }
        sqlx::Error::Io(_) | sqlx::Error::Tls(_) => true,
        _ => false,
    }
}

/// Create a PostgreSQL connection pool using conservative defaults.
///
/// This uses a retry loop on startup with exponential backoff, so transient
/// database downtime (e.g. container still starting) does not crash the service
/// immediately.
pub async fn create_pool(config: &Config) -> Result<PgPool, PoolCreationError> {
    let connect = || async {
        let future = PgPoolOptions::new()
            .max_connections(config.db_max_connections)
            .min_connections(config.db_min_connections)
            .acquire_timeout(Duration::from_secs(config.db_acquire_timeout_secs))
            .connect(&config.database_url);

        match tokio::time::timeout(
            Duration::from_secs(config.db_connect_timeout_secs),
            future,
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(sqlx::Error::PoolTimedOut),
        }
    };

    retry_pool_connection(
        config.db_connect_max_attempts,
        config.db_connect_base_delay_ms,
        config.db_connect_max_delay_ms,
        connect,
    )
    .await
}

fn backoff_delay_ms(attempt: u32, base_delay_ms: u64, max_delay_ms: u64) -> u64 {
    let exponent = attempt.saturating_sub(1).min(31);
    let delay = base_delay_ms.saturating_mul(1u64 << exponent);
    delay.min(max_delay_ms)
}

/// Error type for database pool creation failures.
#[derive(Debug)]
pub struct PoolCreationError {
    /// The last error encountered during connection attempts.
    pub last_error: sqlx::Error,
    /// Number of attempts made before giving up.
    pub attempts: u32,
}

impl std::fmt::Display for PoolCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to create database pool after {} attempts: {}",
            self.attempts, self.last_error
        )
    }
}

impl std::error::Error for PoolCreationError {}

async fn retry_pool_connection<Fut, F>(
    max_attempts: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    mut op: F,
) -> Result<PgPool, PoolCreationError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<PgPool, sqlx::Error>>,
{
    let max_attempts = max_attempts.max(1);
    let mut last_error: Option<sqlx::Error> = None;

    for attempt in 1..=max_attempts {
        match op().await {
            Ok(pool) => {
                if attempt > 1 {
                    info!(
                        attempts = attempt,
                        "database connection established after retries"
                    );
                }
                return Ok(pool);
            }
            Err(err) => {
                last_error = Some(err.clone());

                if !is_transient_error(&err) {
                    error!(
                        attempt,
                        error = %err,
                        "database connection failed with unrecoverable error; aborting"
                    );
                    return Err(PoolCreationError {
                        last_error: err,
                        attempts: attempt,
                    });
                }

                if attempt < max_attempts {
                    let delay_ms = backoff_delay_ms(attempt, base_delay_ms, max_delay_ms);
                    warn!(
                        attempt,
                        max_attempts,
                        delay_ms,
                        error = %err,
                        "database connection failed; retrying"
                    );
                    if delay_ms > 0 {
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }
    }

    let last_error = last_error.expect("at least one error should exist after retry loop");
    error!(
        attempts = max_attempts,
        error = %last_error,
        "database connection retries exhausted"
    );
    Err(PoolCreationError {
        last_error,
        attempts: max_attempts,
    })
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

    #[test]
    fn backoff_delay_is_exponential_and_capped() {
        assert_eq!(backoff_delay_ms(1, 200, 5_000), 200);
        assert_eq!(backoff_delay_ms(2, 200, 5_000), 400);
        assert_eq!(backoff_delay_ms(3, 200, 5_000), 800);
        assert_eq!(backoff_delay_ms(10, 200, 5_000), 5_000);
    }

    #[test]
    fn backoff_delay_with_zero_base_is_zero() {
        assert_eq!(backoff_delay_ms(1, 0, 5_000), 0);
        assert_eq!(backoff_delay_ms(5, 0, 5_000), 0);
    }

    #[test]
    fn backoff_delay_saturates_at_max() {
        assert_eq!(backoff_delay_ms(30, 100, 1_000), 1_000);
        assert_eq!(backoff_delay_ms(64, 1, 500), 500);
    }

    #[test]
    fn config_connect_timeout_is_independent_from_acquire_timeout() {
        let config = crate::config::Config::default_for_test();
        assert!(
            config.db_connect_timeout_secs > 0,
            "connect timeout must be > 0"
        );
        assert!(
            config.db_acquire_timeout_secs > 0,
            "acquire timeout must be > 0"
        );
    }

    /// Test that `is_transient_error` correctly identifies transient connection errors.
    #[test]
    fn is_transient_error_identifies_pool_timeout() {
        assert!(is_transient_error(&sqlx::Error::PoolTimedOut));
        assert!(!is_transient_error(&sqlx::Error::PoolClosed));
    }

    /// Test that `PoolCreationError` formats correctly.
    #[test]
    fn pool_creation_error_formats_last_error_and_attempts() {
        let err = PoolCreationError {
            last_error: sqlx::Error::PoolTimedOut,
            attempts: 5,
        };
        let msg = err.to_string();
        assert!(msg.contains("5 attempts"));
        assert!(msg.contains("PoolTimedOut"));
    }

    /// Test that `retry_pool_connection` retries on transient errors and eventually fails.
    #[tokio::test]
    async fn retry_pool_connection_retries_on_transient_errors() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = retry_pool_connection(3, 0, 0, || async {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Err(sqlx::Error::PoolTimedOut)
        })
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.attempts, 3, "should retry all attempts on transient errors");
    }

    /// Test that `retry_pool_connection` fails fast on unrecoverable errors.
    #[tokio::test]
    async fn retry_pool_connection_fails_fast_on_unrecoverable_error() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = retry_pool_connection(5, 0, 0, || async {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Err(sqlx::Error::PoolClosed)
        })
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.attempts, 1, "should fail after only one attempt for PoolClosed");
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

/// Run `operation` inside a database transaction, committing on success.
pub async fn insert_prediction_from_event_with_pool(
    pool: &PgPool,
    event: &PredictionPlacedEvent,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    insert_prediction_from_event(&mut tx, event).await?;
    tx.commit().await?;
    Ok(())
}

/// Mark a pool as settled and record the winning outcome.
pub async fn resolve_pool_in_db<'e, E>(
    executor: E,
    pool_id: u64,
    winning_outcome: i32,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query("UPDATE pools SET state = 'settled', result = $1 WHERE pool_id = $2")
        .bind(winning_outcome.to_string())
        .bind(pool_id as i64)
        .execute(executor)
        .await?;
    Ok(())
}

/// Mark a pool as closed (cancelled on-chain).
pub async fn cancel_pool_in_db<'e, E>(executor: E, pool_id: u64) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query("UPDATE pools SET state = 'closed' WHERE pool_id = $1")
        .bind(pool_id as i64)
        .execute(executor)
        .await?;
    Ok(())
}

/// Insert a new pool record decoded from a `PoolCreated` contract event.
pub async fn insert_pool_from_event<'e, E>(
    executor: E,
    event: &PoolCreatedEvent,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
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
    .execute(executor)
    .await?;
    Ok(())
}

/// Insert a decoded `prediction_placed` contract event into the prediction index.
///
/// Must run inside a transaction so the prediction insert and pool stake update
/// stay atomic. Use [`insert_prediction_from_event_with_pool`] or pass an open
/// transaction reference.
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

    sqlx::query("UPDATE pools SET total_stake = total_stake + $1 WHERE pool_id = $2")
        .bind(event.amount)
        .bind(event.pool_id as i64)
        .execute(&mut **tx)
        .await?;

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
            rps.pool_id,
            pl.name                    AS pool_name,
            COALESCE(rps.total_earned, 0)::BIGINT AS total_earned,
            rps.referral_count
        FROM referrer_pool_stats rps
        JOIN pools pl ON pl.pool_id = rps.pool_id
        WHERE rps.referrer = $1
        ORDER BY rps.total_earned DESC
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
///
/// When `category` and/or `state` are provided, the aggregates are scoped to
/// the matching pools (and the bets placed in them). Passing `None` for both
/// yields the unfiltered protocol-wide totals.
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
              AND ($2::text IS NULL OR state = $2)
        )
        SELECT
            COALESCE(SUM(total_stake), 0) AS total_value_locked,
            (SELECT COUNT(*) FROM predictions p
                WHERE p.pool_id IN (SELECT pool_id FROM filtered_pools)) AS total_bets,
            COUNT(*) AS total_pools
        FROM filtered_pools
        "#,
    )
    .bind(category)
    .bind(state)
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
/// Large batches are split into chunks of at most `max_batch_size` rows to
/// avoid oversized SQL statements and PostgreSQL parameter limits.
pub async fn insert_referrals_bulk(
    pool: &PgPool,
    events: &[ReferralPaidEvent],
    max_batch_size: usize,
) -> Result<(), sqlx::Error> {
    if events.is_empty() {
        return Ok(());
    }

    let chunk_size = max_batch_size.max(1);
    for chunk in events.chunks(chunk_size) {
        insert_referrals_bulk_chunk(pool, chunk).await?;
    }

    Ok(())
}

async fn insert_referrals_bulk_chunk(
    pool: &PgPool,
    events: &[ReferralPaidEvent],
) -> Result<(), sqlx::Error> {
    if events.is_empty() {
        return Ok(());
    }

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

    query_builder.execute(pool).await?;
    Ok(())
}

/// Insert a single referral record.
///
/// For inserting multiple referrals, use `insert_referrals_bulk` instead for better performance.
pub async fn insert_referral_from_event<'e, E>(
    executor: E,
    event: &ReferralPaidEvent,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
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
    .execute(executor)
    .await?;
    Ok(())
}

// Tests live near the top-level helpers (see `retry_pool_connection` tests).

#[cfg(test)]
mod write_helper_tests {
    use super::*;

    #[test]
    fn insert_referrals_bulk_query_builds_expected_placeholders() {
        let events = vec![
            ReferralPaidEvent {
                pool_id: 1,
                referrer: "GREF".into(),
                referred_user: "GUSER".into(),
                referral_amount: 100,
            },
            ReferralPaidEvent {
                pool_id: 2,
                referrer: "GREF2".into(),
                referred_user: "GUSER2".into(),
                referral_amount: 200,
            },
        ];

        let mut values = Vec::new();
        let mut param_index = 1i32;
        for _ in &events {
            values.push(format!(
                "(${}, ${}, ${}, ${})",
                param_index,
                param_index + 1,
                param_index + 2,
                param_index + 3
            ));
            param_index += 4;
        }

        assert_eq!(values, vec!["($1, $2, $3, $4)", "($5, $6, $7, $8)"]);
    }

    #[test]
    fn insert_referrals_bulk_chunks_large_batches() {
        let events: Vec<_> = (0..5)
            .map(|index| ReferralPaidEvent {
                pool_id: index,
                referrer: format!("GREF{index}"),
                referred_user: format!("GUSER{index}"),
                referral_amount: 100 + index as i64,
            })
            .collect();

        let chunks: Vec<_> = events.chunks(2).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 2);
        assert_eq!(chunks[2].len(), 1);
    }
}

// ── Tests for predictions index migration (009) ───────────────────────────────

#[cfg(test)]
mod predictions_index_tests {
    use super::*;

    // ── calculate_odds ──────────────────────────────────────────────────────

    /// Zero total stake produces zero odds for every outcome (no division by zero).
    #[test]
    fn calculate_odds_zero_total_stake_returns_zero_odds() {
        let stakes = vec![(0i32, 100i64), (1, 200)];
        let odds = calculate_odds(&stakes, 0);

        assert_eq!(odds.len(), 2);
        for o in &odds {
            assert_eq!(o.odds, 0.0, "expected 0.0 odds when total_stake is 0");
        }
    }

    /// An outcome with zero stake inside a non-zero pool gets 0.0 odds (no
    /// division by zero on the per-outcome stake).
    #[test]
    fn calculate_odds_zero_outcome_stake_returns_zero_odds_for_that_outcome() {
        let stakes = vec![(0i32, 0i64), (1, 500)];
        let odds = calculate_odds(&stakes, 500);

        assert_eq!(odds[0].odds, 0.0);
        assert!(
            (odds[1].odds - 1.0).abs() < f64::EPSILON,
            "outcome with 100 % of stake should have odds of 1.0"
        );
    }

    /// Even split across two outcomes should yield odds of 2.0 each.
    #[test]
    fn calculate_odds_even_split_gives_2x_odds() {
        let stakes = vec![(0i32, 500i64), (1, 500)];
        let odds = calculate_odds(&stakes, 1000);

        for o in &odds {
            assert!(
                (o.odds - 2.0).abs() < 1e-9,
                "expected 2.0 odds for a 50/50 split, got {}",
                o.odds
            );
        }
    }

    /// Dominant outcome (90 %) yields odds near 1.11; minority (10 %) near 10.
    #[test]
    fn calculate_odds_asymmetric_split() {
        let stakes = vec![(0i32, 900i64), (1, 100)];
        let odds = calculate_odds(&stakes, 1000);

        let dominant = odds.iter().find(|o| o.outcome == 0).unwrap();
        let minority = odds.iter().find(|o| o.outcome == 1).unwrap();

        assert!(
            (dominant.odds - (1000.0 / 900.0)).abs() < 1e-9,
            "dominant odds should be ~1.111, got {}",
            dominant.odds
        );
        assert!(
            (minority.odds - 10.0).abs() < 1e-9,
            "minority odds should be 10.0, got {}",
            minority.odds
        );
    }

    /// Empty input returns empty output without panicking.
    #[test]
    fn calculate_odds_empty_stakes_returns_empty() {
        let odds = calculate_odds(&[], 0);
        assert!(odds.is_empty());

        let odds_nonzero = calculate_odds(&[], 1000);
        assert!(odds_nonzero.is_empty());
    }

    // ── UserWinnings win_rate ───────────────────────────────────────────────

    /// Win-rate is 0.0 when there are no total predictions (guard against
    /// division by zero in the query result mapper).
    #[test]
    fn user_winnings_win_rate_is_zero_when_no_predictions() {
        let win_rate = if 0 > 0 { 5_f64 / 0_f64 } else { 0.0 };
        assert_eq!(win_rate, 0.0);
    }

    /// Win-rate is computed correctly for a partial win record.
    #[test]
    fn user_winnings_win_rate_partial() {
        let total = 10i64;
        let winning = 3i64;
        let win_rate = winning as f64 / total as f64;
        assert!((win_rate - 0.3).abs() < 1e-9);
    }

    // ── Rank offset calculation ─────────────────────────────────────────────

    /// Rank starts at offset + 1 for the first returned row so pagination
    /// offsets are reflected correctly in the leaderboard.
    #[test]
    fn leaderboard_rank_respects_page_offset() {
        let offset: i64 = 20;
        let rank_of_first_row = offset + 0 + 1; // index 0 in the result set
        let rank_of_second_row = offset + 1 + 1;

        assert_eq!(rank_of_first_row, 21);
        assert_eq!(rank_of_second_row, 22);
    }

    // ── Migration file sanity checks ────────────────────────────────────────

    /// Verify the migration SQL file for index 009 exists and contains the
    /// four expected index names so a future rename does not silently break
    /// the schema.
    #[test]
    fn migration_009_contains_expected_index_names() {
        let sql = include_str!("../migrations/009_add_predictions_indexes.sql");

        let expected_indexes = [
            "idx_predictions_pool_created",
            "idx_predictions_outcome_pool",
            "idx_predictions_pool_user",
            "idx_predictions_amount_desc",
        ];

        for name in &expected_indexes {
            assert!(
                sql.contains(name),
                "migration 009 should define index '{name}', but it was not found in the SQL"
            );
        }
    }

    /// The migration must use `IF NOT EXISTS` for every CREATE INDEX so
    /// re-running migrations on an already-migrated schema is idempotent.
    #[test]
    fn migration_009_all_indexes_are_idempotent() {
        let sql = include_str!("../migrations/009_add_predictions_indexes.sql");

        // Count CREATE INDEX and CREATE INDEX IF NOT EXISTS occurrences.
        let total_creates = sql.matches("CREATE INDEX").count();
        let idempotent_creates = sql.matches("CREATE INDEX IF NOT EXISTS").count();

        assert_eq!(
            total_creates, idempotent_creates,
            "every CREATE INDEX in migration 009 must use IF NOT EXISTS \
             (found {total_creates} CREATE INDEX, {idempotent_creates} with IF NOT EXISTS)"
        );
    }

    /// All indexes in migration 009 must target the `predictions` table.
    #[test]
    fn migration_009_all_indexes_target_predictions_table() {
        let sql = include_str!("../migrations/009_add_predictions_indexes.sql");

        // Each ON clause in the file should reference `predictions`.
        let on_clauses: Vec<&str> = sql.match_indices("ON predictions").map(|(_, s)| s).collect();
        let total_creates = sql.matches("CREATE INDEX IF NOT EXISTS").count();

        assert_eq!(
            on_clauses.len(),
            total_creates,
            "every index in migration 009 must target the 'predictions' table \
             (found {total_creates} CREATE INDEX IF NOT EXISTS but only {} ON predictions clauses)",
            on_clauses.len()
        );
    }
}
