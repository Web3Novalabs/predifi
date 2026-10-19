//! Query builder abstractions and helpers to eliminate duplication patterns.
//!
//! This module provides type-safe, domain-specific builders for common filtering,
//! sorting, and pagination patterns used across the database repository modules.
//!
//! # Design Philosophy
//!
//! Rather than a monolithic QueryBuilder, we provide focused, composable types:
//! - `StatusFilter` — validates and renders pool state values
//! - `SortOrder` — type-safe sort criteria with SQL generation
//! - `CategoryFilter` — optional category binding for WHERE clauses
//! - `PaginationBuilder` — offset-based pagination with rank offset calculation
//!
//! Each builder is opt-in and used by the repository functions that benefit from
//! the abstraction. This keeps queries readable while eliminating boilerplate.

use chrono::{DateTime, Utc};

/// Pool state filter with validation and SQL rendering.
///
/// Validates that the provided state is one of the allowed pool states.
/// If invalid, defaults to `"active"`. This prevents SQL injection and ensures
/// only valid state values reach the database.
///
/// # Examples
///
/// ```rust
/// let filter = StatusFilter::new("active");
/// assert_eq!(filter.to_sql(), "active");
///
/// let filter = StatusFilter::new("invalid");
/// assert_eq!(filter.to_sql(), "active"); // defaults to active
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    Active,
    Closed,
    Settled,
}

impl StatusFilter {
    /// Create a StatusFilter from a string, validating against allowed values.
    ///
    /// Invalid strings default to `Active`.
    pub fn new(status: &str) -> Self {
        match status {
            "active" => StatusFilter::Active,
            "closed" => StatusFilter::Closed,
            "settled" => StatusFilter::Settled,
            _ => StatusFilter::Active,
        }
    }

    /// Render this status as a SQL string literal (without quotes).
    pub fn to_sql(&self) -> &'static str {
        match self {
            StatusFilter::Active => "active",
            StatusFilter::Closed => "closed",
            StatusFilter::Settled => "settled",
        }
    }
}

impl std::fmt::Display for StatusFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_sql())
    }
}

/// Pool sort order with validation and SQL rendering.
///
/// Provides type-safe sorting criteria to prevent SQL injection via string
/// interpolation. Supports sorting pools by popularity, end time, or creation date,
/// and ranking users by volume, win rate, or streak.
///
/// # Examples
///
/// ```rust
/// let order = SortOrder::Popular;
/// assert_eq!(order.to_sql(), "total_stake DESC");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Sort by total stake (descending) — pools with highest stakes first.
    Popular,
    /// Sort by end time (ascending) — pools ending soon first.
    EndingSoon,
    /// Sort by creation date (descending) — newest pools first (default).
    Recent,
    /// Rank users by total betting volume (descending).
    Volume,
    /// Rank users by win rate (descending) — calculated as wins / settled_count.
    WinRate,
    /// Rank users by current win streak (descending).
    Streak,
}

impl SortOrder {
    /// Create a SortOrder from a string, validating against allowed values.
    ///
    /// Invalid strings default to `Recent` (for pools) or `Volume` (for users).
    /// For predictions module (leaderboard), use `from_leaderboard_rank_by`.
    pub fn from_pool_sort_by(sort_by: &str) -> Self {
        match sort_by {
            "popular" => SortOrder::Popular,
            "ending_soon" => SortOrder::EndingSoon,
            _ => SortOrder::Recent,
        }
    }

    /// Create a SortOrder from a leaderboard rank_by parameter.
    pub fn from_leaderboard_rank_by(rank_by: &str) -> Self {
        match rank_by {
            "win_rate" => SortOrder::WinRate,
            "streak" => SortOrder::Streak,
            _ => SortOrder::Volume,
        }
    }

    /// Render this sort order as a SQL ORDER BY clause fragment (without ORDER BY keyword).
    pub fn to_sql(&self) -> &'static str {
        match self {
            SortOrder::Popular => "total_stake DESC",
            SortOrder::EndingSoon => "end_time ASC",
            SortOrder::Recent => "created_at DESC",
            SortOrder::Volume => "ua.total_volume",
            SortOrder::WinRate => {
                "(CASE WHEN ua.settled_count > 0 THEN ua.wins::FLOAT / ua.settled_count ELSE 0 END)"
            }
            SortOrder::Streak => "COALESCE(sc.current_streak, 0)",
        }
    }
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_sql())
    }
}

/// Optional category filter for WHERE clause binding.
///
/// Eliminates boilerplate `($N::text IS NULL OR category = $N)` patterns by
/// providing a typed representation that can be checked directly in query
/// building code.
///
/// # Examples
///
/// ```rust
/// let filter = CategoryFilter::new(Some("sports"));
/// assert!(filter.is_some());
/// assert_eq!(filter.as_ref(), Some(&"sports".to_string()));
///
/// let filter = CategoryFilter::new(None);
/// assert!(filter.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryFilter(pub Option<String>);

impl CategoryFilter {
    /// Create a CategoryFilter from an optional string.
    pub fn new(category: Option<&str>) -> Self {
        CategoryFilter(category.map(|s| s.to_string()))
    }

    /// Check if a category filter is set.
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// Check if a category filter is unset (None).
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Get a reference to the category string if set.
    pub fn as_ref(&self) -> Option<&String> {
        self.0.as_ref()
    }

    /// Extract the category string, consuming the filter.
    pub fn into_inner(self) -> Option<String> {
        self.0
    }
}

impl From<Option<&str>> for CategoryFilter {
    fn from(category: Option<&str>) -> Self {
        CategoryFilter::new(category)
    }
}

impl From<Option<String>> for CategoryFilter {
    fn from(category: Option<String>) -> Self {
        CategoryFilter(category)
    }
}

/// Pagination builder for offset-based pagination with rank offset calculation.
///
/// Eliminates boilerplate `offset + i as i64 + 1` rank calculation that appears
/// in multiple leaderboard and ranking functions. Provides a clean API for
/// transforming rows into ranked results.
///
/// # Examples
///
/// ```rust
/// let rows = vec![
///     (1, "Alice"),
///     (2, "Bob"),
///     (3, "Charlie"),
/// ];
/// let pagination = PaginationBuilder::new(offset: 50);
/// let ranked: Vec<(i64, i64, &str)> = pagination.apply_rank(rows);
/// // Results: [(1, 51, "Alice"), (2, 52, "Bob"), (3, 53, "Charlie")]
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PaginationBuilder {
    /// The offset into the full result set (for rank calculation).
    pub offset: i64,
}

impl PaginationBuilder {
    /// Create a pagination builder with the given offset.
    pub fn new(offset: i64) -> Self {
        PaginationBuilder { offset }
    }

    /// Apply rank offset to a vector of items, returning them with calculated ranks.
    ///
    /// This is useful for paginated leaderboards where each page starts at a
    /// different rank. For example, if offset is 50, the first row gets rank 51,
    /// the second gets rank 52, etc.
    ///
    /// # Generic Parameters
    ///
    /// `T` — the row type (typically a struct like `LeaderboardEntry`)
    /// `F` — a function that transforms `(index, row) -> (T, rank)`
    ///
    /// # Examples
    ///
    /// ```rust
    /// let rows = vec!["Alice", "Bob"];
    /// let pagination = PaginationBuilder::new(10);
    /// let with_rank = pagination.apply_rank_fn(rows, |i, name| {
    ///     (name, pagination.calculate_rank(i as i64))
    /// });
    /// // with_rank = [("Alice", 11), ("Bob", 12)]
    /// ```
    pub fn apply_rank_fn<T, F>(self, rows: Vec<T>, mut transform: F) -> Vec<T>
    where
        F: FnMut(usize, T) -> T,
    {
        rows.into_iter()
            .enumerate()
            .map(|(i, row)| transform(i, row))
            .collect()
    }

    /// Calculate the rank for a given 0-indexed position within this page.
    pub fn calculate_rank(&self, page_position: i64) -> i64 {
        self.offset + page_position + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_filter_parses_active() {
        assert_eq!(StatusFilter::new("active"), StatusFilter::Active);
        assert_eq!(StatusFilter::new("active").to_sql(), "active");
    }

    #[test]
    fn status_filter_parses_closed() {
        assert_eq!(StatusFilter::new("closed"), StatusFilter::Closed);
        assert_eq!(StatusFilter::new("closed").to_sql(), "closed");
    }

    #[test]
    fn status_filter_parses_settled() {
        assert_eq!(StatusFilter::new("settled"), StatusFilter::Settled);
        assert_eq!(StatusFilter::new("settled").to_sql(), "settled");
    }

    #[test]
    fn status_filter_defaults_to_active_on_invalid() {
        assert_eq!(StatusFilter::new("invalid"), StatusFilter::Active);
        assert_eq!(StatusFilter::new(""), StatusFilter::Active);
        assert_eq!(StatusFilter::new("ACTIVE"), StatusFilter::Active); // case-sensitive
    }

    #[test]
    fn sort_order_pool_sorting() {
        assert_eq!(
            SortOrder::from_pool_sort_by("popular").to_sql(),
            "total_stake DESC"
        );
        assert_eq!(
            SortOrder::from_pool_sort_by("ending_soon").to_sql(),
            "end_time ASC"
        );
        assert_eq!(
            SortOrder::from_pool_sort_by("new").to_sql(),
            "created_at DESC"
        );
        assert_eq!(
            SortOrder::from_pool_sort_by("unknown").to_sql(),
            "created_at DESC" // defaults to Recent
        );
    }

    #[test]
    fn sort_order_leaderboard_ranking() {
        assert_eq!(
            SortOrder::from_leaderboard_rank_by("volume").to_sql(),
            "ua.total_volume"
        );
        assert_eq!(
            SortOrder::from_leaderboard_rank_by("win_rate").to_sql(),
            "(CASE WHEN ua.settled_count > 0 THEN ua.wins::FLOAT / ua.settled_count ELSE 0 END)"
        );
        assert_eq!(
            SortOrder::from_leaderboard_rank_by("streak").to_sql(),
            "COALESCE(sc.current_streak, 0)"
        );
    }

    #[test]
    fn category_filter_some() {
        let filter = CategoryFilter::new(Some("sports"));
        assert!(filter.is_some());
        assert_eq!(filter.as_ref(), Some(&"sports".to_string()));
    }

    #[test]
    fn category_filter_none() {
        let filter = CategoryFilter::new(None);
        assert!(filter.is_none());
        assert_eq!(filter.as_ref(), None);
    }

    #[test]
    fn category_filter_from_option_str() {
        let filter = CategoryFilter::from(Some("crypto"));
        assert_eq!(filter.as_ref(), Some(&"crypto".to_string()));

        let filter: CategoryFilter = None.into();
        assert!(filter.is_none());
    }

    #[test]
    fn category_filter_into_inner() {
        let filter = CategoryFilter::new(Some("sports"));
        let inner = filter.into_inner();
        assert_eq!(inner, Some("sports".to_string()));
    }

    #[test]
    fn pagination_builder_calculates_correct_ranks() {
        let pagination = PaginationBuilder::new(0);
        assert_eq!(pagination.calculate_rank(0), 1);
        assert_eq!(pagination.calculate_rank(1), 2);
        assert_eq!(pagination.calculate_rank(9), 10);

        let pagination = PaginationBuilder::new(50);
        assert_eq!(pagination.calculate_rank(0), 51);
        assert_eq!(pagination.calculate_rank(1), 52);
        assert_eq!(pagination.calculate_rank(49), 100);
    }

    #[test]
    fn pagination_builder_apply_rank_fn() {
        let rows = vec!["Alice", "Bob", "Charlie"];
        let pagination = PaginationBuilder::new(10);

        let results = pagination.apply_rank_fn(rows, |_i, name| name);
        assert_eq!(results, vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn pagination_builder_zero_offset() {
        let pagination = PaginationBuilder::new(0);
        assert_eq!(pagination.calculate_rank(0), 1);
        assert_eq!(pagination.calculate_rank(100), 101);
    }

    #[test]
    fn pagination_builder_large_offset() {
        let pagination = PaginationBuilder::new(1_000_000);
        assert_eq!(pagination.calculate_rank(0), 1_000_001);
        assert_eq!(pagination.calculate_rank(99), 1_000_100);
    }

    #[test]
    fn sort_order_display() {
        assert_eq!(format!("{}", SortOrder::Popular), "total_stake DESC");
        assert_eq!(format!("{}", SortOrder::EndingSoon), "end_time ASC");
        assert_eq!(format!("{}", SortOrder::Recent), "created_at DESC");
    }

    #[test]
    fn status_filter_display() {
        assert_eq!(format!("{}", StatusFilter::Active), "active");
        assert_eq!(format!("{}", StatusFilter::Closed), "closed");
        assert_eq!(format!("{}", StatusFilter::Settled), "settled");
    }
}
