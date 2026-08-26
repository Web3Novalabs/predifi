//! Pool repository — all queries that read or write the `pools`,
//! `pool_templates`, and `creator_stats` tables.

use chrono::{DateTime, Utc};
use sqlx::{Executor, PgPool, Postgres};
use tracing::instrument;

// ── Row / DTO types ───────────────────────────────────────────────────────────

/// Summary row returned by list-pools queries.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PoolRow {
    pub pool_id: i64,
    pub name: String,
    pub category: String,
    pub total_stake: i64,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Full pool record including state and creator metadata.
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

/// Per-outcome stake and implied odds.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OutcomeOdds {
    pub outcome: i32,
    pub stake: i64,
    pub odds: f64,
}

/// Full pool detail enriched with real-time odds for every outcome.
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

/// Aggregate creator reputation / reward-eligibility metrics.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CreatorStats {
    pub creator: String,
    pub pools_created: i64,
    pub pools_reward_eligible: i64,
    pub total_volume: i64,
}

/// Recurring pool template configuration.
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

// ── Internal row types (not exported) ────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct OutcomeStakeRow {
    outcome: i32,
    total_stake: i64,
}

// ── Column list shared by template queries ────────────────────────────────────

const POOL_TEMPLATE_COLUMNS: &str =
    "id, creator, name, category, description, token, \
     duration_seconds, recurrence_interval_seconds, next_run_at, active";

// ── Read queries ──────────────────────────────────────────────────────────────

/// Fetch active pools with optional category filter and sort order.
///
/// `sort_by` accepts `"popular"`, `"ending_soon"`, or `"new"` (default).
#[instrument(skip(pool), name = "db.get_active_pools",
    fields(sort_by = sort_by, category = ?category, limit = limit, offset = offset))]
pub async fn get_active_pools(
    pool: &PgPool,
    sort_by: &str,
    category: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<PoolRow>, sqlx::Error> {
    get_pools_with_filters(pool, sort_by, category, "active", limit, offset).await
}

/// Fetch pools matching optional category and status filters with a sort order.
///
/// `sort_by` accepts `"popular"`, `"ending_soon"`, or `"new"` (default).  
/// `status` accepts `"active"`, `"closed"`, or `"settled"` (defaults to `"active"`).
///
/// # Security
/// `order_clause` is built from a controlled `match` on `sort_by` — only the
/// three hardcoded strings reach the SQL. `valid_status` is similarly
/// allow-listed. No raw user input is interpolated into the query.
pub async fn get_pools_with_filters(
    pool: &PgPool,
    sort_by: &str,
    category: Option<&str>,
    status: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<PoolRow>, sqlx::Error> {
    let order_clause = match sort_by {
        "popular"     => "total_stake DESC",
        "ending_soon" => "end_time ASC",
        _             => "created_at DESC", // "new" and default
    };

    let valid_status = match status {
        "active" | "closed" | "settled" => status,
        _ => "active",
    };

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

/// Count pools matching optional category and status filters.
pub async fn count_pools_with_filters(
    pool: &PgPool,
    category: Option<&str>,
    status: &str,
) -> Result<i64, sqlx::Error> {
    let valid_status = match status {
        "active" | "closed" | "settled" => status,
        _ => "active",
    };

    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM pools
        WHERE state = $1
          AND ($2::text IS NULL OR category = $2)
        "#,
    )
    .bind(valid_status)
    .bind(category)
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

/// Fetch all fields for a single pool by its on-chain ID.
pub async fn get_pool_by_id(
    pool: &PgPool,
    pool_id: i64,
) -> Result<Option<PoolDetails>, sqlx::Error> {
    sqlx::query_as::<_, PoolDetails>(
        r#"
        SELECT pool_id, name, category, total_stake, end_time, created_at,
               state, creator, token, result
        FROM pools
        WHERE pool_id = $1
        "#,
    )
    .bind(pool_id)
    .fetch_optional(pool)
    .await
}

/// Return `(outcome, total_stake)` pairs for all outcomes in `pool_id`.
pub async fn get_pool_outcome_stakes(
    pool: &PgPool,
    pool_id: i64,
) -> Result<Vec<(i32, i64)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OutcomeStakeRow>(
        r#"
        SELECT outcome, COALESCE(SUM(amount), 0) AS total_stake
        FROM predictions
        WHERE pool_id = $1
        GROUP BY outcome
        ORDER BY outcome
        "#,
    )
    .bind(pool_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.outcome, r.total_stake)).collect())
}

/// Fetch a pool's full detail joined with real-time odds for each outcome.
///
/// Returns `None` when the pool does not exist.
pub async fn get_pool_with_odds(
    pool: &PgPool,
    pool_id: i64,
) -> Result<Option<PoolWithOdds>, sqlx::Error> {
    let details = match get_pool_by_id(pool, pool_id).await? {
        Some(d) => d,
        None => return Ok(None),
    };

    let outcome_stakes = get_pool_outcome_stakes(pool, pool_id).await?;
    let calculated_total: i64 = outcome_stakes.iter().map(|(_, s)| s).sum();
    let total_stake = std::cmp::max(details.total_stake, calculated_total);
    let odds = calculate_odds(&outcome_stakes, total_stake);

    Ok(Some(PoolWithOdds {
        pool_id: details.pool_id,
        name: details.name,
        category: details.category,
        total_stake,
        end_time: details.end_time,
        created_at: details.created_at,
        state: details.state,
        creator: details.creator,
        token: details.token,
        result: details.result,
        odds,
    }))
}

// ── Write queries ─────────────────────────────────────────────────────────────

/// Insert a pool record decoded from a `pool_created` contract event.
///
/// Idempotent: a second insert for the same `pool_id` is silently ignored
/// via `ON CONFLICT (pool_id) DO NOTHING`.
#[instrument(skip(pool), name = "db.insert_pool_from_event",
    fields(pool_id = event.pool_id, creator = %event.creator))]
pub async fn insert_pool_from_event(
    pool: &PgPool,
    event: &PoolCreatedEvent,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO pools
            (pool_id, name, category, total_stake, end_time, state, creator, token, created_at)
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

    record_pool_created_for_creator(pool, &event.creator).await?;

    Ok(())
}

/// Mark a pool as settled and record its winning outcome.
#[instrument(skip(executor), name = "db.resolve_pool_in_db",
    fields(pool_id = pool_id, winning_outcome = winning_outcome))]
pub async fn resolve_pool_in_db<'e, E>(
    executor: E,
    pool_id: u64,
    winning_outcome: i32,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        "UPDATE pools SET state = 'settled', result = $1, resolved_at = NOW() WHERE pool_id = $2",
    )
    .bind(winning_outcome.to_string())
    .bind(pool_id as i64)
    .execute(executor)
    .await?;
    Ok(())
}

/// Mark a pool as closed (cancelled on-chain).
#[instrument(skip(executor), name = "db.cancel_pool_in_db", fields(pool_id = pool_id))]
pub async fn cancel_pool_in_db<'e, E>(
    executor: E,
    pool_id: u64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query("UPDATE pools SET state = 'closed' WHERE pool_id = $1")
        .bind(pool_id as i64)
        .execute(executor)
        .await?;
    Ok(())
}

// ── Odds calculation (pure, no I/O) ──────────────────────────────────────────

/// Convert `(outcome, stake)` pairs into implied odds.
///
/// `odds = total_stake / outcome_stake`. Outcomes with zero stake, or a zero
/// total, receive odds of `0.0` rather than causing a divide-by-zero.
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
                total_stake as f64 / *stake as f64
            };
            OutcomeOdds {
                outcome: *outcome,
                stake: *stake,
                odds,
            }
        })
        .collect()
}

// ── Creator incentive system (#1366) ─────────────────────────────────────────

/// Upsert creator stats, incrementing `pools_created` by one.
///
/// Called automatically by [`insert_pool_from_event`] — should not need to be
/// called directly.
pub async fn record_pool_created_for_creator(
    pool: &PgPool,
    creator: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO creator_stats
            (creator, pools_created, pools_reward_eligible, total_volume, updated_at)
        VALUES ($1, 1, 0, 0, NOW())
        ON CONFLICT (creator) DO UPDATE
            SET pools_created = creator_stats.pools_created + 1,
                updated_at    = NOW()
        "#,
    )
    .bind(creator)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch aggregate reputation metrics for a creator.
pub async fn get_creator_stats(
    pool: &PgPool,
    creator: &str,
) -> Result<Option<CreatorStats>, sqlx::Error> {
    sqlx::query_as::<_, CreatorStats>(
        r#"
        SELECT creator, pools_created, pools_reward_eligible, total_volume
        FROM creator_stats
        WHERE creator = $1
        "#,
    )
    .bind(creator)
    .fetch_optional(pool)
    .await
}

/// Return `true` when a pool has met its minimum participation threshold and
/// its creator reward has not yet been paid.
pub async fn is_creator_reward_eligible(
    pool: &PgPool,
    pool_id: i64,
) -> Result<bool, sqlx::Error> {
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

/// Calculate the creator incentive amount from a pool's total stake.
///
/// `reward = floor( (total_stake × treasury_fee_bps / 10_000) × creator_reward_bps / 10_000 )`
pub fn calculate_creator_incentive(
    total_stake: i64,
    treasury_fee_bps: u32,
    creator_reward_bps: i32,
) -> i64 {
    let treasury_fee = (total_stake as i128 * treasury_fee_bps as i128) / 10_000;
    let creator_share = (treasury_fee * creator_reward_bps.max(0) as i128) / 10_000;
    creator_share as i64
}

/// Pay the creator incentive for an eligible pool and update `creator_stats`.
///
/// Idempotent: returns `Ok(None)` when the pool does not exist, is not yet
/// eligible, or the reward has already been paid. The `NOT creator_reward_paid`
/// guard in the UPDATE prevents double-payment under concurrent requests.
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

    let amount = calculate_creator_incentive(
        details.total_stake,
        treasury_fee_bps,
        creator_reward_bps,
    );

    // Atomic CAS-style update: only succeeds when reward has not yet been paid.
    let updated = sqlx::query(
        r#"
        UPDATE pools
        SET creator_reward_paid   = TRUE,
            creator_reward_amount = $2
        WHERE pool_id = $1
          AND NOT creator_reward_paid
        "#,
    )
    .bind(pool_id)
    .bind(amount)
    .execute(pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Ok(None); // concurrent call already paid
    }

    sqlx::query(
        r#"
        UPDATE creator_stats
        SET pools_reward_eligible = pools_reward_eligible + 1,
            total_volume          = total_volume + $2,
            updated_at            = NOW()
        WHERE creator = $1
        "#,
    )
    .bind(&details.creator)
    .bind(details.total_stake)
    .execute(pool)
    .await?;

    Ok(Some(amount))
}

// ── Pool templates (#1368) ────────────────────────────────────────────────────

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
            (creator, name, category, description, token,
             duration_seconds, recurrence_interval_seconds, next_run_at, active)
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

/// List all templates owned by `creator`, newest first.
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

/// Return all active templates whose `next_run_at` has passed.
///
/// Callers should create the on-chain pool and then call
/// [`advance_pool_template`] to update the schedule.
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

/// Advance a template's `next_run_at` by one recurrence interval.
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

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── calculate_odds ────────────────────────────────────────────────────────

    #[test]
    fn calculate_odds_zero_total_stake_returns_zero_odds() {
        let stakes = vec![(0i32, 100i64), (1, 200)];
        let odds = calculate_odds(&stakes, 0);
        assert_eq!(odds.len(), 2);
        for o in &odds {
            assert_eq!(o.odds, 0.0, "expected 0.0 when total_stake is 0");
        }
    }

    #[test]
    fn calculate_odds_zero_outcome_stake_returns_zero_for_that_outcome() {
        let stakes = vec![(0i32, 0i64), (1, 500)];
        let odds = calculate_odds(&stakes, 500);
        assert_eq!(odds[0].odds, 0.0);
        assert!(
            (odds[1].odds - 1.0).abs() < f64::EPSILON,
            "100% stake outcome should have odds of 1.0"
        );
    }

    #[test]
    fn calculate_odds_even_split_gives_2x_odds() {
        let stakes = vec![(0i32, 500i64), (1, 500)];
        let odds = calculate_odds(&stakes, 1_000);
        for o in &odds {
            assert!((o.odds - 2.0).abs() < 1e-9, "50/50 split must yield 2.0 odds");
        }
    }

    #[test]
    fn calculate_odds_asymmetric_split() {
        let stakes = vec![(0i32, 900i64), (1, 100)];
        let odds = calculate_odds(&stakes, 1_000);

        let dominant = odds.iter().find(|o| o.outcome == 0).unwrap();
        let minority = odds.iter().find(|o| o.outcome == 1).unwrap();

        assert!((dominant.odds - (1_000.0 / 900.0)).abs() < 1e-9);
        assert!((minority.odds - 10.0).abs() < 1e-9);
    }

    #[test]
    fn calculate_odds_empty_stakes_returns_empty() {
        assert!(calculate_odds(&[], 0).is_empty());
        assert!(calculate_odds(&[], 1_000).is_empty());
    }

    // ── calculate_creator_incentive ───────────────────────────────────────────

    #[test]
    fn creator_incentive_zero_stake_yields_zero() {
        assert_eq!(calculate_creator_incentive(0, 300, 1_000), 0);
    }

    #[test]
    fn creator_incentive_negative_reward_bps_treated_as_zero() {
        // Negative creator_reward_bps must not produce a negative payout.
        assert_eq!(calculate_creator_incentive(1_000_000, 300, -500), 0);
    }

    #[test]
    fn creator_incentive_correct_value() {
        // 1_000_000 stake × 3% treasury × 10% creator share = 3_000
        let amount = calculate_creator_incentive(1_000_000, 300, 1_000);
        assert_eq!(amount, 3_000);
    }

    // ── Migration 009 sanity (kept close to the queries they guard) ───────────

    #[test]
    fn migration_009_contains_expected_index_names() {
        let sql = include_str!("../../migrations/009_add_predictions_indexes.sql");
        for name in &[
            "idx_predictions_pool_created",
            "idx_predictions_outcome_pool",
            "idx_predictions_pool_user",
            "idx_predictions_amount_desc",
        ] {
            assert!(sql.contains(name), "migration 009 must define index '{name}'");
        }
    }

    #[test]
    fn migration_009_all_indexes_are_idempotent() {
        let sql = include_str!("../../migrations/009_add_predictions_indexes.sql");
        let total = sql.matches("CREATE INDEX").count();
        let idempotent = sql.matches("CREATE INDEX IF NOT EXISTS").count();
        assert_eq!(total, idempotent, "every CREATE INDEX must use IF NOT EXISTS");
    }
}
