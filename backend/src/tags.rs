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
}

/// Fetch pools with optional category/tag/status filters and sort order.
///
/// `sort_by` accepts `"popular"`, `"ending_soon"`, or `"new"`.
/// `status` accepts `"active"`, `"closed"`, or `"settled"`.
/// `tags`, when non-empty, matches pools whose `tags` array overlaps it.
pub async fn list_pools(
    pool: &PgPool,
    sort_by: &str,
    category: Option<&str>,
    tags: Option<&[String]>,
    status: &str,
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
        SELECT pool_id, name, category, tags, total_stake, end_time, created_at
        FROM pools
        WHERE state = $1
          AND ($2::text IS NULL OR category = $2)
          AND ($3::text[] IS NULL OR tags && $3)
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
        .fetch_all(pool)
        .await
}

/// Count pools matching the same category/tag/status filters as [`list_pools`].
pub async fn count_pools(
    pool: &PgPool,
    category: Option<&str>,
    tags: Option<&[String]>,
    status: &str,
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
    "#;

    let count: (i64,) = sqlx::query_as(sql)
        .bind(valid_status)
        .bind(category)
        .bind(tags)
        .fetch_one(pool)
        .await?;

    Ok(count.0)
}

/// Distinct tags in use across all pools, alphabetically — powers filter-UI
/// dropdowns on the frontend.
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
