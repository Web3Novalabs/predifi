# Database Layer — Modular Repository Pattern

This directory replaces the original monolithic `db.rs` file (1,964 lines) with
a domain-driven repository structure.

## Module Organization

```
backend/src/db/
├── mod.rs           - Pool creation, retry, metrics re-exports
├── pools.rs         - Pool CRUD, templates, creator incentives
├── predictions.rs   - Predictions, leaderboard, protocol stats
├── referrals.rs     - Referral inserts and earnings
└── metrics.rs       - Connection pool metrics (Prometheus gauges)
```

## Backwards Compatibility

All public functions and types are re-exported from `mod.rs`, so existing
callers (`crate::db::get_active_pools`, `crate::db::PoolRow`, etc.) continue to
work without any changes:

```rust
// ✓ Before refactor:
use crate::db::{get_active_pools, PoolRow};

// ✓ After refactor (unchanged):
use crate::db::{get_active_pools, PoolRow};
```

## Benefits

- **Reduced cognitive load** — each module is 200–500 lines vs. 2k
- **Clear domain boundaries** — pools, predictions, referrals are distinct
- **Easier testing** — unit tests co-located with the queries they verify
- **Zero churn for callers** — all existing imports stay valid

## Connection Pool Metrics (#1405)

`metrics.rs` exposes four Prometheus gauges:

| Metric | Description |
|--------|-------------|
| `db_pool_size` | Total connection slots (idle + active) |
| `db_pool_idle` | Connections currently idle |
| `db_pool_active` | Connections currently checked out |
| `db_pool_utilization_ratio` | Fraction of slots in use (`active / size`) |

### Usage

```rust
use crate::db::metrics::{register_pool_gauges, PoolMetricGauges};

// At startup:
let gauges = register_pool_gauges(&prometheus_registry)?;

// Periodically (e.g. every 15 s in a background task):
gauges.record(&pool);
```

## Query Organization

### `pools.rs`

**Read queries:**
- `get_active_pools`, `get_pools_with_filters`, `count_pools_with_filters`
- `get_pool_by_id`, `get_pool_outcome_stakes`, `get_pool_with_odds`
- `get_creator_stats`, `is_creator_reward_eligible`
- `list_pool_templates`, `get_due_pool_templates`

**Write queries:**
- `insert_pool_from_event`, `resolve_pool_in_db`, `cancel_pool_in_db`
- `record_pool_created_for_creator`, `pay_creator_incentive`
- `create_pool_template`, `advance_pool_template`

**Business logic:**
- `calculate_odds`, `calculate_creator_incentive`

### `predictions.rs`

**Read queries:**
- `get_user_prediction_history`, `get_user_predictions`
- `get_market_predictions`, `count_market_predictions`
- `get_users_by_betting_volume`, `get_users_by_winnings`
- `get_leaderboard_extended`, `get_protocol_stats`

**Write queries:**
- `insert_prediction_from_event`, `insert_prediction_from_event_with_pool`

### `referrals.rs`

**Read queries:**
- `get_referral_earnings`

**Write queries:**
- `insert_referral_from_event`, `insert_referrals_bulk`

### `mod.rs`

**Infrastructure:**
- `create_pool` — retry loop with exponential backoff
- `is_transient_error` — classify sqlx errors as transient vs. permanent
- `PoolCreationError` — startup-failure error type

## Query audit and performance (#2204)

Target: **< 100 ms p99 on every API endpoint** (alert `ApiP99LatencyAboveTarget`,
per-query latency in `app_db_query_duration_seconds{query_type=...}`).

### Findings and fixes

| Area | Problem found | Fix |
| :--- | :--- | :--- |
| `notifications::run_notification_sweep` | **N+1**: one query per pool to list its users, then one `INSERT` per user — `1 + P + P·U` round trips every 5 min. The "resolved" pass also re-scanned every pool ever settled. | Four set-based `INSERT … SELECT … ON CONFLICT DO NOTHING` statements (4 round trips total); the resolved pass is bounded by `RESOLVED_LOOKBACK` (7 days). |
| `get_users_by_winnings` | Scanned `predictions ⨝ pools` **three times** (winning totals, winning predictions, user totals) and `FULL OUTER JOIN`ed the results. | One shared `settled` CTE scan feeding all three aggregates; deterministic `user_address` tie-break. |
| `get_pool_with_odds` | Three sequential round trips (pool row, outcome stakes, separate `COUNT(*)`). | Prediction count comes from the outcome aggregate; the two reads run concurrently via `try_join!`. |
| `pay_creator_incentive` | Three sequential reads (eligibility, pool row, reward bps) before the CAS `UPDATE`. | One read. The `UPDATE … WHERE NOT creator_reward_paid` guard is unchanged. |
| `get_market_predictions` | `ORDER BY id DESC` for a pool with no matching index → read + sort all of the pool's rows. | `idx_predictions_pool_id_desc (pool_id, id DESC)` (migration 016). |
| `get_users_by_betting_volume`, profile queries | `GROUP BY user_address` / per-user aggregates fetched every row from the heap. | Covering index `idx_predictions_user_covering` (migration 016) → index-only scans. |
| Notification interest matching | `WHERE interest = ANY($1)` against a PK that leads with `user_address`. | `idx_user_interests_interest` (migration 016). |
| Observability | Leaderboard, winnings, profile and tag queries were not timed, so p99 was invisible for the slowest endpoints. | Wrapped in `metrics::observe_db`. |

Unnecessary joins: none found — every `JOIN pools` in `get_user_*` reads columns
from `pools` that are part of the response.

Deliberately **not** changed: the pools list's `($2 IS NULL OR category = $2)`
filter (fine with Postgres' per-execution custom plans) and full-text search
(`to_tsvector(... array_to_string(tags))` is not `IMMUTABLE`, so it cannot be
indexed without a schema change). Both are cached in Redis (see #2205).

### Capturing `EXPLAIN ANALYZE` output

```bash
cd backend
cargo run --bin predifi-explain-queries                    # busiest user, biggest pool
cargo run --bin predifi-explain-queries -- <ADDRESS> <POOL_ID>
```

This prints `EXPLAIN (ANALYZE, BUFFERS)` for every critical query in
[`explain.rs`](explain.rs) (pool list, pool detail, market predictions, user
history, all leaderboards, protocol stats, profile stats). Each statement runs
in a transaction that is always rolled back, so it is safe against a replica.
Plans depend on table size and statistics, so capture them against production-
sized data (`ANALYZE` first) and paste the output into the PR / issue.

What to look for: `Seq Scan on predictions` for a pool- or user-scoped query,
a `Sort` node above a scan that an index could have ordered, `Heap Fetches`
much larger than 0 on an index-only scan (run `VACUUM`), and `Buffers:
shared read` far above `shared hit`.

## Testing

Each module has its own `#[cfg(test)]` section with unit tests for pure
functions (odds calculation, placeholders, rank logic) and regression guards
(migration presence checks, win-rate guards, cursor pagination).

All integration tests remain in `backend/src/db_integration_tests.rs` and
continue to work unchanged because the public API surface is identical.

## Migration Notes

- **Before:** 1 file, 1,964 lines
- **After:** 5 files, ~1,800 lines total (including new metrics + docs)
- **API changes:** None — all re-exported from `mod.rs`
- **Caller changes:** None — `use crate::db::*` continues to work

## Related Issues

- Closes #1405 (refactor db.rs, add pooling metrics, reduce duplication)
