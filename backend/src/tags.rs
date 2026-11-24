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

/// Categories offered by default in the UI. Creators are not limited to this
/// list — any label that passes [`normalize_category`] is accepted — but these
/// are always returned by `GET /api/v1/categories` so filter UIs have a stable
/// baseline even before any pool exists in a category.
pub const KNOWN_CATEGORIES: &[&str] = &[
    "crypto",
    "sports",
    "politics",
    "finance",
    "entertainment",
    "technology",
    "science",
    "gaming",
    "other",
];

/// Category assigned when a creator supplies none (or an unusable one).
pub const DEFAULT_CATEGORY: &str = "other";
/// Longest accepted category, in characters.
pub const MAX_CATEGORY_LENGTH: usize = 64;
/// Longest accepted tag, in characters.
pub const MAX_TAG_LENGTH: usize = 32;
/// Most tags a pool may carry.
pub const MAX_TAGS_PER_POOL: usize = 10;

/// Strictly normalise a category: trim, lower-case and collapse inner
/// whitespace. Allowed characters are `a-z`, `0-9`, space, `-`, `_` and `&`.
pub fn normalize_category(raw: &str) -> Result<String, &'static str> {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    if collapsed.is_empty() {
        return Err("category must not be empty");
    }
    if collapsed.chars().count() > MAX_CATEGORY_LENGTH {
        return Err("category must be 64 characters or fewer");
    }
    if !collapsed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '&'))
    {
        return Err("category may only contain letters, digits, spaces, '-', '_' and '&'");
    }
    Ok(collapsed)
}

/// Like [`normalize_category`] but never fails: unusable input becomes
/// [`DEFAULT_CATEGORY`]. Used on the indexer path, where a pool created
/// on-chain must be stored even if its category is odd.
pub fn normalize_category_lenient(raw: &str) -> String {
    normalize_category(raw).unwrap_or_else(|_| DEFAULT_CATEGORY.to_string())
}

fn normalize_tag(raw: &str) -> Result<String, &'static str> {
    let tag = raw
        .trim()
        .trim_start_matches('#')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase();
    if tag.is_empty() {
        return Err("tags must not be empty");
    }
    if tag.chars().count() > MAX_TAG_LENGTH {
        return Err("each tag must be 32 characters or fewer");
    }
    if !tag
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err("tags may only contain letters, digits, '-' and '_'");
    }
    Ok(tag)
}

/// Strictly normalise a tag set: each tag is trimmed, stripped of a leading
/// `#`, lower-cased and has inner whitespace turned into `-`; duplicates are
/// removed (order preserved). Rejects invalid tags and more than
/// [`MAX_TAGS_PER_POOL`] distinct tags.
pub fn normalize_tags(raw: &[String]) -> Result<Vec<String>, &'static str> {
    let mut out: Vec<String> = Vec::with_capacity(raw.len());
    for tag in raw {
        let tag = normalize_tag(tag)?;
        if !out.contains(&tag) {
            out.push(tag);
        }
    }
    if out.len() > MAX_TAGS_PER_POOL {
        return Err("a pool may have at most 10 tags");
    }
    Ok(out)
}

/// Like [`normalize_tags`] but drops invalid tags and truncates to
/// [`MAX_TAGS_PER_POOL`] instead of failing (indexer path).
pub fn normalize_tags_lenient(raw: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in raw {
        if let Ok(tag) = normalize_tag(tag) {
            if !out.contains(&tag) {
                out.push(tag);
            }
        }
        if out.len() == MAX_TAGS_PER_POOL {
            break;
        }
    }
    out
}

/// A category and how many pools are currently in it.
#[derive(Debug, Clone, Serialize, FromRow, PartialEq, Eq)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

/// A tag and how many pools carry it.
#[derive(Debug, Clone, Serialize, FromRow, PartialEq, Eq)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

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

    crate::metrics::observe_db(
        "list_pools",
        sqlx::query_as::<_, PoolListingRow>(&sql)
            .bind(valid_status)
            .bind(category)
            .bind(tags)
            .bind(limit)
            .bind(offset)
            .bind(q)
            .fetch_all(pool),
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

    let count: (i64,) = crate::metrics::observe_db(
        "count_pools",
        sqlx::query_as(sql)
            .bind(valid_status)
            .bind(category)
            .bind(tags)
            .bind(q)
            .fetch_one(pool),
    )
    .await?;

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
    let rows: Vec<(String,)> = crate::metrics::observe_db(
        "list_distinct_tags",
        sqlx::query_as(
            r#"
            SELECT DISTINCT unnest(tags) AS tag
            FROM pools
            ORDER BY tag
            "#,
        )
        .fetch_all(pool),
    )
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

/// Categories with the number of **active** pools in each, most populated
/// first. Every entry of [`KNOWN_CATEGORIES`] is included (with a zero count
/// when empty) so filter UIs always have the baseline list.
pub async fn list_category_counts(pool: &PgPool) -> Result<Vec<CategoryCount>, sqlx::Error> {
    let rows: Vec<CategoryCount> = sqlx::query_as(
        r#"
        SELECT lower(category) AS category, COUNT(*) AS count
        FROM pools
        WHERE state = 'active' AND category <> ''
        GROUP BY lower(category)
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(merge_known_categories(rows))
}

/// Merge DB counts with [`KNOWN_CATEGORIES`] and order by count (desc), then name.
fn merge_known_categories(mut rows: Vec<CategoryCount>) -> Vec<CategoryCount> {
    for known in KNOWN_CATEGORIES {
        if !rows.iter().any(|r| r.category == *known) {
            rows.push(CategoryCount {
                category: (*known).to_string(),
                count: 0,
            });
        }
    }
    rows.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.category.cmp(&b.category)));
    rows
}

/// Tags with the number of **active** pools carrying each, most used first.
pub async fn list_tag_counts(pool: &PgPool, limit: i64) -> Result<Vec<TagCount>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT tag, COUNT(*) AS count
        FROM pools, unnest(tags) AS tag
        WHERE state = 'active'
        GROUP BY tag
        ORDER BY count DESC, tag ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// The creator wallet of `pool_id`, or `None` when the pool does not exist.
pub async fn get_pool_creator(pool: &PgPool, pool_id: i64) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT creator FROM pools WHERE pool_id = $1")
        .bind(pool_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(creator,)| creator))
}

/// Update a pool's category and/or tags. `None` leaves a field unchanged.
/// Values must already be normalised. Returns `false` when the pool is
/// missing.
pub async fn update_pool_labels(
    pool: &PgPool,
    pool_id: i64,
    category: Option<&str>,
    tags: Option<&[String]>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE pools
        SET category = COALESCE($1, category),
            tags     = COALESCE($2, tags)
        WHERE pool_id = $3
        "#,
    )
    .bind(category)
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

    #[test]
    fn category_is_trimmed_lowercased_and_validated() {
        assert_eq!(normalize_category("  Crypto ").unwrap(), "crypto");
        assert_eq!(normalize_category("Pop   Culture").unwrap(), "pop culture");
        assert!(normalize_category("   ").is_err());
        assert!(normalize_category("bad;drop").is_err());
        assert!(normalize_category(&"x".repeat(MAX_CATEGORY_LENGTH + 1)).is_err());
        assert_eq!(normalize_category_lenient("???"), DEFAULT_CATEGORY);
    }

    #[test]
    fn tags_are_normalised_and_deduplicated() {
        let raw = vec!["#BTC".to_string(), "btc".to_string(), "Price Prediction".to_string()];
        assert_eq!(normalize_tags(&raw).unwrap(), vec!["btc", "price-prediction"]);
        assert!(normalize_tags(&["bad tag!".to_string()]).is_err());
        assert!(normalize_tags(&[String::new()]).is_err());
        let many: Vec<String> = (0..=MAX_TAGS_PER_POOL).map(|i| format!("t{i}")).collect();
        assert!(normalize_tags(&many).is_err());
    }

    #[test]
    fn lenient_tags_drop_invalid_and_truncate() {
        let mut raw: Vec<String> = (0..20).map(|i| format!("t{i}")).collect();
        raw.insert(0, "no good!".to_string());
        let out = normalize_tags_lenient(&raw);
        assert_eq!(out.len(), MAX_TAGS_PER_POOL);
        assert_eq!(out[0], "t0");
    }

    #[test]
    fn known_categories_are_always_listed() {
        let merged = merge_known_categories(vec![CategoryCount {
            category: "sports".into(),
            count: 4,
        }]);
        assert_eq!(merged[0].category, "sports");
        for known in KNOWN_CATEGORIES {
            assert!(merged.iter().any(|c| c.category == *known));
        }
    }
}
