//! Pool search and advanced filtering.
//!
//! Builds the `GET /api/v1/pools` query from a [`PoolFilter`]. Every
//! user-supplied value is passed as a bind parameter; the only text spliced
//! into SQL is the `ORDER BY` clause, which comes from the closed
//! [`PoolSort`] enum.
//!
//! Supported filters: status, category, tags (overlap), token, creator, stake
//! range, end-time range and full-text search (`q`). Supported sorts:
//! popularity (prediction count), stake amount (asc/desc), end date
//! (asc/desc), newest and — when `q` is set — relevance.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::tags::PoolListingRow;

/// Longest accepted search string, in characters.
pub const MAX_SEARCH_LENGTH: usize = 100;
/// Longest accepted token / creator filter, in characters.
pub const MAX_ADDRESS_FILTER_LENGTH: usize = 128;

/// Sort orders for pool listings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolSort {
    /// Most predictions first, ties broken by total stake.
    Popular,
    /// Highest total stake first.
    StakeDesc,
    /// Lowest total stake first.
    StakeAsc,
    /// Earliest end time first.
    EndingSoon,
    /// Latest end time first.
    EndingLatest,
    /// Newest pools first (default).
    New,
    /// Best full-text match first (requires `q`; otherwise behaves as `New`).
    Relevance,
}

impl PoolSort {
    /// Parse a sort key; unknown values fall back to [`PoolSort::New`].
    pub fn parse(raw: &str) -> Self {
        match raw {
            "popular" => PoolSort::Popular,
            "stake_desc" => PoolSort::StakeDesc,
            "stake_asc" => PoolSort::StakeAsc,
            "ending_soon" => PoolSort::EndingSoon,
            "ending_latest" => PoolSort::EndingLatest,
            "relevance" => PoolSort::Relevance,
            _ => PoolSort::New,
        }
    }
}

/// Normalise a user-facing status to the stored `pools.state` value.
///
/// `resolved` maps to `settled` and `cancelled` / `canceled` map to `closed`
/// (cancelled pools are stored as `closed`). Anything unrecognised is
/// `active`.
pub fn normalize_status(status: &str) -> &'static str {
    match status {
        "closed" | "cancelled" | "canceled" => "closed",
        "settled" | "resolved" => "settled",
        _ => "active",
    }
}

/// All filters accepted by the pool listing. `Default` matches every active
/// pool.
#[derive(Debug, Clone, Default)]
pub struct PoolFilter {
    /// Stored state; normalised through [`normalize_status`] when queried.
    pub status: String,
    pub category: Option<String>,
    /// Matches pools whose tags overlap this list.
    pub tags: Vec<String>,
    pub token: Option<String>,
    pub creator: Option<String>,
    pub min_stake: Option<i64>,
    pub max_stake: Option<i64>,
    /// Only pools ending at or after this instant.
    pub ends_after: Option<DateTime<Utc>>,
    /// Only pools ending at or before this instant.
    pub ends_before: Option<DateTime<Utc>>,
    /// Full-text search over name, category and tags.
    pub q: Option<String>,
}

impl PoolFilter {
    /// Validate cross-field constraints.
    pub fn validate(&self) -> Result<(), &'static str> {
        if let (Some(min), Some(max)) = (self.min_stake, self.max_stake) {
            if min > max {
                return Err("min_stake must not exceed max_stake");
            }
        }
        if self.min_stake.is_some_and(|v| v < 0) || self.max_stake.is_some_and(|v| v < 0) {
            return Err("stake bounds must not be negative");
        }
        if let (Some(after), Some(before)) = (self.ends_after, self.ends_before) {
            if after > before {
                return Err("ends_after must not be later than ends_before");
            }
        }
        if self.q.as_ref().is_some_and(|q| q.chars().count() > MAX_SEARCH_LENGTH) {
            return Err("q must be 100 characters or fewer");
        }
        for value in [&self.token, &self.creator].into_iter().flatten() {
            if value.chars().count() > MAX_ADDRESS_FILTER_LENGTH {
                return Err("token and creator filters must be 128 characters or fewer");
            }
        }
        Ok(())
    }

    /// Deterministic cache-key suffix covering every filter beyond status,
    /// category and sort (which `pools_cache_key` already encodes). Empty for
    /// an unfiltered request so warmed default keys keep matching.
    pub fn cache_suffix(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.tags.is_empty() {
            parts.push(format!("tags={}", self.tags.join("\u{1f}")));
        }
        if let Some(v) = &self.q {
            parts.push(format!("q={v}"));
        }
        if let Some(v) = &self.token {
            parts.push(format!("token={v}"));
        }
        if let Some(v) = &self.creator {
            parts.push(format!("creator={v}"));
        }
        if let Some(v) = self.min_stake {
            parts.push(format!("min={v}"));
        }
        if let Some(v) = self.max_stake {
            parts.push(format!("max={v}"));
        }
        if let Some(v) = self.ends_after {
            parts.push(format!("after={}", v.timestamp()));
        }
        if let Some(v) = self.ends_before {
            parts.push(format!("before={}", v.timestamp()));
        }
        if parts.is_empty() {
            return String::new();
        }
        // Hashed (not sanitised) so distinct filters can never collide on
        // characters a Redis key must strip.
        let mut hasher = DefaultHasher::new();
        parts.hash(&mut hasher);
        format!(":f:{:016x}", hasher.finish())
    }
}

/// Escape `LIKE` wildcards so user input is matched literally.
fn escape_like(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

const SEARCH_VECTOR: &str = "to_tsvector('english', pool_search_text(name, category, tags))";

fn push_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filter: &'a PoolFilter, status: &'static str) {
    qb.push(" WHERE state = ").push_bind(status);

    if let Some(category) = &filter.category {
        qb.push(" AND lower(category) = lower(").push_bind(category).push(")");
    }
    if !filter.tags.is_empty() {
        qb.push(" AND tags && ").push_bind(&filter.tags[..]);
    }
    if let Some(token) = &filter.token {
        qb.push(" AND token = ").push_bind(token);
    }
    if let Some(creator) = &filter.creator {
        qb.push(" AND creator = ").push_bind(creator);
    }
    if let Some(min) = filter.min_stake {
        qb.push(" AND total_stake >= ").push_bind(min);
    }
    if let Some(max) = filter.max_stake {
        qb.push(" AND total_stake <= ").push_bind(max);
    }
    if let Some(after) = filter.ends_after {
        qb.push(" AND end_time >= ").push_bind(after);
    }
    if let Some(before) = filter.ends_before {
        qb.push(" AND end_time <= ").push_bind(before);
    }
    if let Some(q) = &filter.q {
        qb.push(" AND (")
            .push(SEARCH_VECTOR)
            .push(" @@ plainto_tsquery('english', ")
            .push_bind(q)
            .push(") OR name ILIKE '%' || ")
            .push_bind(escape_like(q))
            .push(" || '%' OR category ILIKE '%' || ")
            .push_bind(escape_like(q))
            .push(" || '%')");
    }
}

/// Fetch one page of pools matching `filter`, ordered by `sort`.
pub async fn search_pools(
    pool: &PgPool,
    filter: &PoolFilter,
    sort: PoolSort,
    limit: i64,
    offset: i64,
) -> Result<Vec<PoolListingRow>, sqlx::Error> {
    let status = normalize_status(&filter.status);
    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT pool_id, name, category, tags, total_stake, prediction_count, \
         end_time, created_at, state, creator, token, result FROM pools",
    );
    push_filters(&mut qb, filter, status);

    // SECURITY: ORDER BY text is chosen from this closed match only.
    match (sort, &filter.q) {
        (PoolSort::Relevance, Some(q)) => {
            qb.push(" ORDER BY ts_rank(")
                .push(SEARCH_VECTOR)
                .push(", plainto_tsquery('english', ")
                .push_bind(q)
                .push(")) DESC, created_at DESC, pool_id DESC");
        }
        (PoolSort::Popular, _) => {
            qb.push(" ORDER BY prediction_count DESC, total_stake DESC, pool_id DESC");
        }
        (PoolSort::StakeDesc, _) => {
            qb.push(" ORDER BY total_stake DESC, pool_id DESC");
        }
        (PoolSort::StakeAsc, _) => {
            qb.push(" ORDER BY total_stake ASC, pool_id DESC");
        }
        (PoolSort::EndingSoon, _) => {
            qb.push(" ORDER BY end_time ASC, pool_id DESC");
        }
        (PoolSort::EndingLatest, _) => {
            qb.push(" ORDER BY end_time DESC, pool_id DESC");
        }
        _ => {
            qb.push(" ORDER BY created_at DESC, pool_id DESC");
        }
    }

    qb.push(" LIMIT ").push_bind(limit);
    qb.push(" OFFSET ").push_bind(offset);

    qb.build_query_as::<PoolListingRow>().fetch_all(pool).await
}

/// Count pools matching `filter` (ignores sort and pagination).
pub async fn count_filtered_pools(pool: &PgPool, filter: &PoolFilter) -> Result<i64, sqlx::Error> {
    let status = normalize_status(&filter.status);
    let mut qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM pools");
    push_filters(&mut qb, filter, status);
    let (count,): (i64,) = qb.build_query_as().fetch_one(pool).await?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_aliases_map_to_stored_states() {
        assert_eq!(normalize_status("resolved"), "settled");
        assert_eq!(normalize_status("settled"), "settled");
        assert_eq!(normalize_status("cancelled"), "closed");
        assert_eq!(normalize_status("canceled"), "closed");
        assert_eq!(normalize_status("closed"), "closed");
        assert_eq!(normalize_status("active"), "active");
        assert_eq!(normalize_status("bogus"), "active");
    }

    #[test]
    fn sort_parse_falls_back_to_new() {
        assert_eq!(PoolSort::parse("popular"), PoolSort::Popular);
        assert_eq!(PoolSort::parse("stake_asc"), PoolSort::StakeAsc);
        assert_eq!(PoolSort::parse("ending_latest"), PoolSort::EndingLatest);
        assert_eq!(PoolSort::parse("nonsense"), PoolSort::New);
    }

    #[test]
    fn validate_rejects_inverted_ranges() {
        let f = PoolFilter { min_stake: Some(10), max_stake: Some(5), ..Default::default() };
        assert!(f.validate().is_err());
        let f = PoolFilter { min_stake: Some(-1), ..Default::default() };
        assert!(f.validate().is_err());
        let f = PoolFilter {
            ends_after: DateTime::from_timestamp(200, 0),
            ends_before: DateTime::from_timestamp(100, 0),
            ..Default::default()
        };
        assert!(f.validate().is_err());
        assert!(PoolFilter::default().validate().is_ok());
    }

    #[test]
    fn cache_suffix_empty_when_unfiltered_and_distinct_otherwise() {
        assert_eq!(PoolFilter::default().cache_suffix(), "");
        let a = PoolFilter { creator: Some("GA".into()), ..Default::default() };
        let b = PoolFilter { creator: Some("GB".into()), ..Default::default() };
        assert_ne!(a.cache_suffix(), b.cache_suffix());
        assert_eq!(a.cache_suffix(), a.clone().cache_suffix());
    }

    #[test]
    fn like_wildcards_are_escaped() {
        assert_eq!(escape_like("50%_off\\"), "50\\%\\_off\\\\");
    }
}
