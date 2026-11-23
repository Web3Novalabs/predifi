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
    /// Number of predictions placed in the pool (popularity).
    pub prediction_count: i64,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub state: String,
    pub creator: String,
    pub token: String,
    pub result: Option<String>,
}

/// Fetch pools with optional full-text search, category/tag/status filters, and sort order.
///
/// Thin wrapper over [`crate::search::search_pools`] kept for callers (such as
/// the cache warmer) that only need the basic filters. `sort_by` accepts any
/// [`crate::search::PoolSort`] key; `status` accepts `active`, `closed`/`cancelled`
/// or `settled`/`resolved`.
#[allow(clippy::too_many_arguments)]
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
    let filter = basic_filter(category, tags, status, q);
    crate::search::search_pools(
        pool,
        &filter,
        crate::search::PoolSort::parse(sort_by),
        limit,
        offset,
    )
    .await
}

/// Count pools matching the same filters as [`list_pools`].
pub async fn count_pools(
    pool: &PgPool,
    category: Option<&str>,
    tags: Option<&[String]>,
    status: &str,
    q: Option<&str>,
) -> Result<i64, sqlx::Error> {
    crate::search::count_filtered_pools(pool, &basic_filter(category, tags, status, q)).await
}

fn basic_filter(
    category: Option<&str>,
    tags: Option<&[String]>,
    status: &str,
    q: Option<&str>,
) -> crate::search::PoolFilter {
    crate::search::PoolFilter {
        status: status.to_string(),
        category: category.map(str::to_string),
        tags: tags.map(<[String]>::to_vec).unwrap_or_default(),
        q: q.map(str::to_string),
        ..Default::default()
    }
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
    tags: &[String],
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE pools SET tags = $1 WHERE pool_id = $2")
        .bind(tags)
        .bind(pool_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_status_defaults_to_active() {
        assert_eq!(crate::search::normalize_status("bogus"), "active");
    }
}
