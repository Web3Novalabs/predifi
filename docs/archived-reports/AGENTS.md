# Agent Notes

## Project Structure
- `backend/src/db/` - Database connection pool and query helpers
  - `backend/src/db/mod.rs` - Pool creation with retry/backoff, error types, and re-exports
  - `backend/src/db/pools.rs` - Pool CRUD, templates, and creator incentives
  - `backend/src/db/predictions.rs` - Predictions, leaderboard, and protocol stats
  - `backend/src/db/referrals.rs` - Referral inserts and earnings
  - `backend/src/db/metrics.rs` - Connection pool Prometheus gauges
- `backend/src/server.rs` - Server startup and HTTP handlers
- `backend/src/config.rs` - Configuration with retry settings (`db_connect_max_attempts`, `db_connect_base_delay_ms`, `db_connect_max_delay_ms`)

## Lint/Check Commands
```bash
cd backend && cargo check
cd backend && cargo clippy
cd backend && cargo fmt --check
cd backend && cargo test
```

## Configuration Defaults
- `db_connect_max_attempts`: 5
- `db_connect_base_delay_ms`: 200
- `db_connect_max_delay_ms`: 5_000
