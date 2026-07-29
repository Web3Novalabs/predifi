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
