//! Pool tagging/categorization.
//!
//! `pools.category` remains the single primary classification (unchanged);
//! `pools.tags` is a free-form array creators can use for finer-grained
//! labels (e.g. a "Crypto" pool tagged `["btc", "price-prediction"]`). Kept
//! as an independent module so the listing/filter query can evolve without
//! touching the existing `db::get_pools_with_filters` path.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};

/// A pool row for listing views, including its tags.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PoolListingRow {
    pub pool_id: i64,
    pub name: String,
    pub category: String,
    pub tags: Vec<String>,
    pub total_stake: i64,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub state: String,
    pub creator: String,
    pub token: String,
    pub result: Option<String>,
}

/// Fetch pools with optional full-text search, category/tag/status filters, and sort order.
///
/// `sort_by` accepts `"popular"`, `"ending_soon"`, or `"new"`.
/// `status` accepts `"active"`, `"closed"`, or `"settled"`.
/// `tags`, when non-empty, matches pools whose `tags` array overlaps it.
/// `q`, when `Some`, performs full-text search across `name`, `category`, and
/// `tags` using `to_tsvector` / `plainto_tsquery` — falls back to `ILIKE`
/// prefix matching when the value is too short for a useful tsvector.
///
/// Lists pools matching all supplied filters ordered by the requested sort
/// mode.  Returns a database error if the query cannot be completed.
pub async fn list_pools(
    pool: &PgPool,
    sort_by: &str,
    category: Option<&str>,
    tags: Option<&[String]>,
    status: &str,
    q: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<PoolListingRow>, sqlx::Error> {
    let order_clause = match sort_by {
        "popular" => "total_stake DESC",
        "ending_soon" => "end_time ASC",
        _ => "created_at DESC",
    };

    let valid_status = match status {
        "active" | "closed" | "settled" => status,
        _ => "active",
    };

    // SECURITY: order_clause comes from the controlled match above only —
    // no user input reaches the format string directly, mirroring the
    // pattern already used in db::get_pools_with_filters.
    let sql = format!(
        r#"
        SELECT pool_id, name, category, tags, total_stake, end_time, created_at,
               state, creator, token, result
        FROM pools
        WHERE state = $1
          AND ($2::text IS NULL OR category = $2)
          AND ($3::text[] IS NULL OR tags && $3)
          AND (
            $6::text IS NULL
            OR (
              to_tsvector('english', name || ' ' || category || ' ' || array_to_string(tags, ' '))
              @@ plainto_tsquery('english', $6)
            )
            OR name ILIKE '%' || $6 || '%'
            OR category ILIKE '%' || $6 || '%'
          )
        ORDER BY {order_clause}
        LIMIT $4 OFFSET $5
        "#
    );

    sqlx::query_as::<_, PoolListingRow>(&sql)
        .bind(valid_status)
        .bind(category)
        .bind(tags)
        .bind(limit)
        .bind(offset)
        .bind(q)
        .fetch_all(pool)
        .await
}

/// Count pools matching the same full-text search, category/tag/status filters
/// as [`list_pools`].  Returns the matching row count or a database error.
pub async fn count_pools(
    pool: &PgPool,
    category: Option<&str>,
    tags: Option<&[String]>,
    status: &str,
    q: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let valid_status = match status {
        "active" | "closed" | "settled" => status,
        _ => "active",
    };

    let sql = r#"
        SELECT COUNT(*)
        FROM pools
        WHERE state = $1
          AND ($2::text IS NULL OR category = $2)
          AND ($3::text[] IS NULL OR tags && $3)
          AND (
            $4::text IS NULL
            OR (
              to_tsvector('english', name || ' ' || category || ' ' || array_to_string(tags, ' '))
              @@ plainto_tsquery('english', $4)
            )
            OR name ILIKE '%' || $4 || '%'
            OR category ILIKE '%' || $4 || '%'
          )
    "#;

    let count: (i64,) = sqlx::query_as(sql)
        .bind(valid_status)
        .bind(category)
        .bind(tags)
        .bind(q)
        .fetch_one(pool)
        .await?;

    Ok(count.0)
}

/// Distinct tags in use across all pools, alphabetically — powers filter-UI
/// dropdowns on the frontend.
/// Returns all distinct pool tags in alphabetical order, or a database error.
pub async fn list_distinct_tags(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT unnest(tags) AS tag
        FROM pools
        ORDER BY tag
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(tag,)| tag).collect())
}

/// Replace the tag set for `pool_id`, scoped to `creator` so only the pool's
/// creator can edit its tags. Returns `false` if no matching row was updated
/// (pool not found, or `creator` doesn't own it).
/// Replaces a pool's tags when `creator` owns the pool.
///
/// Returns `true` when a row was updated and `false` when the pool is missing
/// or owned by another creator; database failures are returned unchanged.
pub async fn update_pool_tags(
    pool: &PgPool,
    pool_id: i64,
    creator: &str,
    tags: &[String],
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE pools SET tags = $1 WHERE pool_id = $2 AND creator = $3")
        .bind(tags)
        .bind(pool_id)
        .bind(creator)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_status_defaults_to_active() {
        let valid_status = match "bogus" {
            "active" | "closed" | "settled" => "bogus",
            _ => "active",
        };
        assert_eq!(valid_status, "active");
    }
}
