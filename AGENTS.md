# Agent Notes

## Project Structure
- `backend/src/db.rs` - Database connection pool and query helpers
- `backend/src/server.rs` - Server startup and HTTP handlers
- `backend/src/config.rs` - Configuration with retry settings (`db_connect_max_attempts`, `db_connect_base_delay_ms`, `db_connect_max_delay_ms`)

## Lint/Check Commands
```bash
cd backend && cargo check
cd backend && cargo clippy
cd backend && cargo fmt --check
cd backend && cargo test
```

## Key Changes Made
1. Fixed `create_pool` return type: `PoolStartupError` → `PoolCreationError` (line 38 in db.rs)
2. The `retry_pool_connection` function now properly uses `PoolCreationError` for error handling
3. `is_transient_error` correctly classifies `PoolClosed` as non-transient and `PoolTimedOut` as transient

## Configuration Defaults
- `db_connect_max_attempts`: 5
- `db_connect_base_delay_ms`: 200
- `db_connect_max_delay_ms`: 5_000