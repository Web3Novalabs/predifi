use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::config::Config;

/// Create a PostgreSQL connection pool using conservative defaults.
///
/// This uses lazy connection mode so local development can start the server
/// without requiring an active database until a query is executed.
pub fn create_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .min_connections(config.db_min_connections)
        .acquire_timeout(Duration::from_secs(config.db_acquire_timeout_secs))
        .connect_lazy(&config.database_url)
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
    sqlx::query_as!(
        PredictionHistoryRow,
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
        address,
        limit,
        offset
    )
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
#[derive(Debug, Clone, serde::Serialize)]
pub struct OutcomeOdds {
    pub outcome: i32,
    pub stake: i64,
    pub odds: f64,
}

/// Complete pool information with odds.
#[derive(Debug, Clone, serde::Serialize)]
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
    // Previously this computed each winning pool's total winning-outcome
    // stake via a correlated subquery re-executed once per winning
    // prediction row (an N+1-shaped query within a single SQL statement).
    // `pool_winning_totals` pre-aggregates that sum once per pool instead,
    // and `winning_predictions` joins against it directly (#1370).
    let sql = r#"
        WITH pool_winning_totals AS (
            SELECT pl.pool_id, SUM(p.amount) AS winning_stake_total
            FROM predictions p
            JOIN pools pl ON pl.pool_id = p.pool_id
            WHERE pl.state = 'settled'
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
            JOIN pools pl ON pl.pool_id = p.pool_id
            JOIN pool_winning_totals pwt ON pwt.pool_id = pl.pool_id
            WHERE pl.state = 'settled'
              AND pl.result IS NOT NULL
              AND p.outcome = CAST(pl.result AS INTEGER)
        ),
        user_winnings AS (
            SELECT
                user_address,
                SUM(amount * (total_stake::FLOAT / NULLIF(winning_stake_total, 0))) as total_winnings,
                COUNT(*) as winning_predictions
            FROM winning_predictions
            GROUP BY user_address
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

/// Insert a new pool record decoded from a `PoolCreated` contract event.
pub async fn insert_pool_from_event(
    pool: &PgPool,
    event: &PoolCreatedEvent,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO pools (pool_id, name, category, total_stake, end_time, state, creator, token, created_at)
        VALUES ($1, $2, $3, 0, to_timestamp($4), 'active', $5, $6, NOW())
        ON CONFLICT (pool_id) DO NOTHING
        "#,
        event.pool_id as i64,
        event.description,
        event.category,
        event.end_time as f64,
        event.creator,
        event.token,
    )
    .execute(pool)
    .await?;

    record_pool_created_for_creator(pool, &event.creator).await?;

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
    sqlx::query_as!(
        ReferralEarningRow,
        r#"
        SELECT
            r.pool_id,
            pl.name          AS pool_name,
            SUM(r.amount)    AS "total_earned!: i64",
            COUNT(r.id)      AS "referral_count!: i64"
        FROM referrals r
        JOIN pools pl ON pl.pool_id = r.pool_id
        WHERE r.referrer = $1
        GROUP BY r.pool_id, pl.name
        ORDER BY SUM(r.amount) DESC
        "#,
        address
    )
    .fetch_all(pool)
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

// ── Pool creator incentive system (#1366) ───────────────────────────────────

/// Aggregate reputation / quality metrics for a pool creator, tracked across
/// every pool they've created — used to gauge creator trustworthiness and
/// pool quality independent of any single pool's outcome.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CreatorStats {
    pub creator: String,
    pub pools_created: i64,
    pub pools_reward_eligible: i64,
    pub total_volume: i64,
}

/// Upsert creator stats after a new pool is indexed, incrementing
/// `pools_created`. Called from [`insert_pool_from_event`].
pub async fn record_pool_created_for_creator(
    pool: &PgPool,
    creator: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO creator_stats (creator, pools_created, pools_reward_eligible, total_volume, updated_at)
        VALUES ($1, 1, 0, 0, NOW())
        ON CONFLICT (creator) DO UPDATE
        SET pools_created = creator_stats.pools_created + 1,
            updated_at = NOW()
        "#,
    )
    .bind(creator)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch aggregate reputation / quality metrics for a pool creator.
pub async fn get_creator_stats(
    pool: &PgPool,
    creator: &str,
) -> Result<Option<CreatorStats>, sqlx::Error> {
    sqlx::query_as::<_, CreatorStats>(
        "SELECT creator, pools_created, pools_reward_eligible, total_volume FROM creator_stats WHERE creator = $1",
    )
    .bind(creator)
    .fetch_optional(pool)
    .await
}

/// Check whether a pool has reached its configured minimum participation
/// threshold (measured in total stake) and its creator reward has not
/// already been paid out.
pub async fn is_creator_reward_eligible(pool: &PgPool, pool_id: i64) -> Result<bool, sqlx::Error> {
    let row: Option<(bool,)> = sqlx::query_as(
        r#"
        SELECT (total_stake >= min_participation_threshold AND NOT creator_reward_paid)
        FROM pools
        WHERE pool_id = $1
        "#,
    )
    .bind(pool_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(eligible,)| eligible).unwrap_or(false))
}

/// Calculate the creator's incentive amount: a share (in basis points) of
/// the protocol's treasury fee, itself a share of the pool's total stake.
pub fn calculate_creator_incentive(
    total_stake: i64,
    treasury_fee_bps: u32,
    creator_reward_bps: i32,
) -> i64 {
    let treasury_fee = (total_stake as i128 * treasury_fee_bps as i128) / 10_000;
    let creator_share = (treasury_fee * creator_reward_bps.max(0) as i128) / 10_000;
    creator_share as i64
}

/// Pay out the creator incentive for a pool that has reached its
/// participation threshold. Idempotent: returns `Ok(None)` if the pool
/// doesn't exist, isn't yet eligible, or has already been paid.
pub async fn pay_creator_incentive(
    pool: &PgPool,
    pool_id: i64,
    treasury_fee_bps: u32,
) -> Result<Option<i64>, sqlx::Error> {
    if !is_creator_reward_eligible(pool, pool_id).await? {
        return Ok(None);
    }

    let Some(details) = get_pool_by_id(pool, pool_id).await? else {
        return Ok(None);
    };

    let creator_reward_bps: i32 =
        sqlx::query_scalar("SELECT creator_reward_bps FROM pools WHERE pool_id = $1")
            .bind(pool_id)
            .fetch_one(pool)
            .await?;

    let amount =
        calculate_creator_incentive(details.total_stake, treasury_fee_bps, creator_reward_bps);

    // Guard the update with `NOT creator_reward_paid` so a concurrent call
    // can't double-pay the same pool.
    let updated = sqlx::query(
        r#"
        UPDATE pools
        SET creator_reward_paid = TRUE, creator_reward_amount = $2
        WHERE pool_id = $1 AND NOT creator_reward_paid
        "#,
    )
    .bind(pool_id)
    .bind(amount)
    .execute(pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Ok(None);
    }

    sqlx::query(
        r#"
        UPDATE creator_stats
        SET pools_reward_eligible = pools_reward_eligible + 1,
            total_volume = total_volume + $2,
            updated_at = NOW()
        WHERE creator = $1
        "#,
    )
    .bind(&details.creator)
    .bind(details.total_stake)
    .execute(pool)
    .await?;

    Ok(Some(amount))
}

// ── Pool templates for recurring markets (#1368) ────────────────────────────

/// A saved pool configuration that can be reused to spin up new pools on a
/// recurring schedule (e.g. "weekly BTC price prediction").
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PoolTemplate {
    pub id: i64,
    pub creator: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub token: String,
    pub duration_seconds: i64,
    pub recurrence_interval_seconds: i64,
    pub next_run_at: DateTime<Utc>,
    pub active: bool,
}

const POOL_TEMPLATE_COLUMNS: &str = "id, creator, name, category, description, token, \
     duration_seconds, recurrence_interval_seconds, next_run_at, active";

/// Create a new recurring pool template.
#[allow(clippy::too_many_arguments)]
pub async fn create_pool_template(
    pool: &PgPool,
    creator: &str,
    name: &str,
    category: &str,
    description: &str,
    token: &str,
    duration_seconds: i64,
    recurrence_interval_seconds: i64,
) -> Result<PoolTemplate, sqlx::Error> {
    let next_run_at = Utc::now() + chrono::Duration::seconds(recurrence_interval_seconds);
    let sql = format!(
        r#"
        INSERT INTO pool_templates
            (creator, name, category, description, token, duration_seconds, recurrence_interval_seconds, next_run_at, active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, TRUE)
        RETURNING {POOL_TEMPLATE_COLUMNS}
        "#
    );

    sqlx::query_as::<_, PoolTemplate>(&sql)
        .bind(creator)
        .bind(name)
        .bind(category)
        .bind(description)
        .bind(token)
        .bind(duration_seconds)
        .bind(recurrence_interval_seconds)
        .bind(next_run_at)
        .fetch_one(pool)
        .await
}

/// List all templates owned by a creator, most recently created first.
pub async fn list_pool_templates(
    pool: &PgPool,
    creator: &str,
) -> Result<Vec<PoolTemplate>, sqlx::Error> {
    let sql = format!(
        r#"
        SELECT {POOL_TEMPLATE_COLUMNS}
        FROM pool_templates
        WHERE creator = $1
        ORDER BY created_at DESC
        "#
    );

    sqlx::query_as::<_, PoolTemplate>(&sql)
        .bind(creator)
        .fetch_all(pool)
        .await
}

/// Fetch all active templates whose scheduled `next_run_at` has passed —
/// these are due for a new pool to be created from them.
///
/// Note: this backend is a read-only indexer for the on-chain PrediFi
/// contract (see [`crate::worker::sync`]) and does not itself submit Soroban
/// transactions, so actually creating the on-chain pool from a due template
/// is the responsibility of the frontend/operator tooling that submits
/// transactions. This function only surfaces which templates are due; call
/// [`advance_pool_template`] once the caller has submitted that transaction.
pub async fn get_due_pool_templates(pool: &PgPool) -> Result<Vec<PoolTemplate>, sqlx::Error> {
    let sql = format!(
        r#"
        SELECT {POOL_TEMPLATE_COLUMNS}
        FROM pool_templates
        WHERE active AND next_run_at <= NOW()
        ORDER BY next_run_at ASC
        "#
    );

    sqlx::query_as::<_, PoolTemplate>(&sql).fetch_all(pool).await
}

/// Advance a template's `next_run_at` by its recurrence interval after a
/// pool has been created from it.
pub async fn advance_pool_template(pool: &PgPool, template_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE pool_templates
        SET next_run_at = next_run_at + (recurrence_interval_seconds * INTERVAL '1 second')
        WHERE id = $1
        "#,
    )
    .bind(template_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn creates_pool_from_valid_config() {
        let mut config = Config::default_for_test();
        config.database_url = String::from("postgres://postgres:postgres@localhost:5432/predifi");

        let pool = create_pool(&config).expect("pool should initialize in lazy mode");
        assert!(!pool.is_closed(), "new pool should start open");
    }
}
