//! `EXPLAIN ANALYZE` harness for the API's critical read queries (#2204).
//!
//! The goal of #2204 is **< 100 ms p99** on every endpoint. Whether a query can
//! meet that depends on its plan (index scan vs. sequential scan + sort), which
//! only a real planner on realistic data can tell you — so rather than
//! hard-coding plan text that would go stale, this module makes it trivial to
//! capture fresh plans:
//!
//! ```text
//! cargo run --bin predifi-explain-queries                 # all queries
//! cargo run --bin predifi-explain-queries -- GABC… 42     # user address, pool id
//! ```
//!
//! Each statement is run as `EXPLAIN (ANALYZE, BUFFERS)` inside a transaction
//! that is **always rolled back**, so the harness is safe to point at a
//! production replica. See `backend/src/db/README.md` for what to look for.
//!
//! The leaderboard and winnings queries are taken from the same
//! `leaderboard_sql` / `USERS_BY_WINNINGS_SQL` the API runs, so they cannot
//! drift. The remaining statements mirror the SQL of the function named in
//! [`CriticalQuery::source`]; update both together.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::predictions::{leaderboard_cutoff, leaderboard_sql, USERS_BY_WINNINGS_SQL};

/// A bind parameter for an explained statement.
#[derive(Debug, Clone)]
pub enum Param {
    I64(i64),
    OptI64(Option<i64>),
    Text(String),
    OptText(Option<String>),
    OptTimestamp(Option<DateTime<Utc>>),
}

/// One critical query to profile.
#[derive(Debug, Clone)]
pub struct CriticalQuery {
    /// Short label printed above the plan.
    pub name: &'static str,
    /// The endpoint(s) this query backs.
    pub endpoint: &'static str,
    /// The `db` function whose SQL this mirrors.
    pub source: &'static str,
    pub sql: String,
    pub params: Vec<Param>,
}

/// Run `EXPLAIN (ANALYZE, BUFFERS)` for `sql` and return the plan text.
///
/// The statement executes for real (that is what `ANALYZE` means), but inside a
/// transaction that is rolled back.
pub async fn explain_analyze(
    pool: &PgPool,
    sql: &str,
    params: &[Param],
) -> Result<String, sqlx::Error> {
    let explain_sql = format!("EXPLAIN (ANALYZE, BUFFERS) {sql}");
    let mut tx = pool.begin().await?;

    let mut query = sqlx::query_scalar::<_, String>(&explain_sql);
    for param in params {
        query = match param {
            Param::I64(v) => query.bind(*v),
            Param::OptI64(v) => query.bind(*v),
            Param::Text(v) => query.bind(v.clone()),
            Param::OptText(v) => query.bind(v.clone()),
            Param::OptTimestamp(v) => query.bind(*v),
        };
    }
    let lines = query.fetch_all(&mut *tx).await;

    // Always roll back, whether or not the EXPLAIN succeeded.
    tx.rollback().await?;
    Ok(lines?.join("\n"))
}

/// The critical read queries, parameterised for `address` and `pool_id`.
pub fn critical_queries(address: &str, pool_id: i64) -> Vec<CriticalQuery> {
    let addr = || Param::Text(address.to_string());

    vec![
        CriticalQuery {
            name: "pool list (popular, active)",
            endpoint: "GET /api/v1/pools",
            source: "tags::list_pools",
            sql: r#"
                SELECT pool_id, name, category, tags, total_stake, end_time, created_at,
                       state, creator, token, result
                FROM pools
                WHERE state = $1
                  AND ($2::text IS NULL OR category = $2)
                ORDER BY total_stake DESC
                LIMIT $3 OFFSET $4
            "#
            .to_string(),
            params: vec![
                Param::Text("active".into()),
                Param::OptText(None),
                Param::I64(20),
                Param::I64(0),
            ],
        },
        CriticalQuery {
            name: "pool detail row",
            endpoint: "GET /api/v1/pools/:id",
            source: "db::pools::get_pool_by_id",
            sql: r#"
                SELECT pool_id, name, category, total_stake, end_time, created_at,
                       state, creator, token, result, outcome_descriptions
                FROM pools
                WHERE pool_id = $1
            "#
            .to_string(),
            params: vec![Param::I64(pool_id)],
        },
        CriticalQuery {
            name: "pool outcome stats + prediction count",
            endpoint: "GET /api/v1/pools/:id",
            source: "db::pools::get_pool_outcome_stats",
            sql: r#"
                SELECT outcome,
                       COALESCE(SUM(amount), 0) AS total_stake,
                       COUNT(*)                 AS prediction_count
                FROM predictions
                WHERE pool_id = $1
                GROUP BY outcome
                ORDER BY outcome
            "#
            .to_string(),
            params: vec![Param::I64(pool_id)],
        },
        CriticalQuery {
            name: "market predictions (cursor page)",
            endpoint: "GET /api/v1/markets/:id/predictions",
            source: "db::predictions::get_market_predictions",
            sql: r#"
                SELECT id, pool_id, user_address, outcome, amount, created_at
                FROM predictions
                WHERE pool_id = $1
                  AND ($2::bigint IS NULL OR id < $2)
                ORDER BY id DESC
                LIMIT $3
            "#
            .to_string(),
            params: vec![Param::I64(pool_id), Param::OptI64(None), Param::I64(21)],
        },
        CriticalQuery {
            name: "user prediction history",
            endpoint: "GET /api/v1/users/:address/history",
            source: "db::predictions::get_user_prediction_history",
            sql: r#"
                SELECT p.pool_id, pl.name AS pool_name, pl.result AS pool_result,
                       p.outcome, p.amount, p.created_at
                FROM predictions p
                JOIN pools pl ON pl.pool_id = p.pool_id
                WHERE p.user_address = $1
                ORDER BY p.created_at DESC
                LIMIT $2 OFFSET $3
            "#
            .to_string(),
            params: vec![addr(), Param::I64(20), Param::I64(0)],
        },
        CriticalQuery {
            name: "leaderboard — volume, all time",
            endpoint: "GET /api/v1/leaderboard",
            source: "db::predictions::get_leaderboard_extended",
            sql: leaderboard_sql("volume"),
            params: vec![
                Param::I64(20),
                Param::I64(0),
                Param::OptI64(None),
                Param::OptTimestamp(leaderboard_cutoff("all")),
            ],
        },
        CriticalQuery {
            name: "leaderboard — win rate, last week",
            endpoint: "GET /api/v1/leaderboard?period=week",
            source: "db::predictions::get_leaderboard_extended",
            sql: leaderboard_sql("win_rate"),
            params: vec![
                Param::I64(20),
                Param::I64(0),
                Param::OptI64(None),
                Param::OptTimestamp(leaderboard_cutoff("week")),
            ],
        },
        CriticalQuery {
            name: "leaderboard — pool scoped",
            endpoint: "GET /api/v1/pools/:id/leaderboard",
            source: "db::predictions::get_leaderboard_extended",
            sql: leaderboard_sql("volume"),
            params: vec![
                Param::I64(20),
                Param::I64(0),
                Param::OptI64(Some(pool_id)),
                Param::OptTimestamp(None),
            ],
        },
        CriticalQuery {
            name: "leaderboard — winnings",
            endpoint: "GET /api/v1/leaderboard?rank_by=winnings",
            source: "db::predictions::get_users_by_winnings",
            sql: USERS_BY_WINNINGS_SQL.to_string(),
            params: vec![Param::I64(20), Param::I64(0)],
        },
        CriticalQuery {
            name: "protocol stats",
            endpoint: "GET /api/v1/stats",
            source: "db::predictions::get_protocol_stats",
            sql: r#"
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
            "#
            .to_string(),
            params: vec![Param::OptText(None), Param::OptText(None)],
        },
        CriticalQuery {
            name: "user profile stats",
            endpoint: "GET /api/v1/users/:address/profile",
            source: "profile::get_profile_stats",
            sql: r#"
                SELECT
                    COUNT(*)::BIGINT AS total_predictions,
                    COUNT(*) FILTER (
                        WHERE pl.state = 'settled' AND pl.result IS NOT NULL
                          AND pl.result ~ '^\d+$' AND pl.result::int = p.outcome
                    )::BIGINT AS wins,
                    COALESCE(SUM(p.amount), 0)::BIGINT AS total_staked,
                    COALESCE(SUM(p.claimed_amount) FILTER (WHERE p.claimed), 0)::BIGINT
                        AS total_earnings
                FROM predictions p
                JOIN pools pl ON pl.pool_id = p.pool_id
                WHERE p.user_address = $1
            "#
            .to_string(),
            params: vec![addr()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Highest `$n` placeholder referenced by `sql`.
    fn max_placeholder(sql: &str) -> usize {
        let bytes = sql.as_bytes();
        let mut max = 0;
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'$' {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 1 {
                    max = max.max(sql[i + 1..j].parse::<usize>().unwrap_or(0));
                }
                i = j;
            } else {
                i += 1;
            }
        }
        max
    }

    /// Every query must bind exactly as many parameters as it references,
    /// otherwise Postgres would reject the EXPLAIN at runtime.
    #[test]
    fn param_counts_match_placeholders() {
        for q in critical_queries("GTEST", 1) {
            assert_eq!(
                max_placeholder(&q.sql),
                q.params.len(),
                "parameter count mismatch for `{}`",
                q.name
            );
        }
    }

    #[test]
    fn query_names_are_unique() {
        let queries = critical_queries("GTEST", 1);
        let names: HashSet<_> = queries.iter().map(|q| q.name).collect();
        assert_eq!(names.len(), queries.len());
    }
}
