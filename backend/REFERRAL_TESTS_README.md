# Referral Integration Tests

This document describes the end-to-end integration tests for the referral system.

## Location

- Test file: `src/referral_integration_tests.rs`

## Test Coverage

The tests cover the following scenarios:

### 1. Referrer Registration and Volume Tracking
- Verifies that when a referral is inserted, it's properly recorded in the `referrals` table
- Confirms that referrer statistics are automatically updated via triggers
- Tests that total volume and unique users count are correctly tracked

### 2. Referral Cut Calculation
- Validates that multiple referrals from the same referrer are aggregated correctly
- Confirms per-pool breakdown of referral earnings
- Tests that referral counts are accurate

### 3. Referral Volume Tracking Across Multiple Pools
- Verifies that referrals to different pools are tracked separately
- Confirms that global totals match the sum of per-pool volumes
- Tests that unique users are counted correctly across multiple pools

### 4. Referrer Update Scenarios
- **Duplicate Insert**: Verifies that inserting the same referral twice doesn't cause double-counting (ON CONFLICT behavior)
- **Same User, Different Pools**: Tests that a user can be referred multiple times to different pools, with correct volume aggregation

### 5. Multiple Referrers
- Ensures that different referrers don't contaminate each other's stats
- Validates that per-referrer volume and user counts are isolated

### 6. Edge Cases
- Zero amount referrals
- Large volume handling (1M+ XLM)
- Concurrent referrals (multiple inserts in parallel)
- Timestamp verification (created_at auto-population)

### 7. Index Verification
- Confirms that all indexes are functional:
  - `idx_referrals_referrer` - for referrer queries
  - `idx_referrals_user_address` - for user address queries
  - `idx_referrals_referrer_pool` - composite index for referrer+pool queries
  - `idx_referrals_pool_id` - for pool_id queries

### 8. Function Tests
- `get_referral_earnings`: Tests the per-pool breakdown function
- `estimate_referral_reward`: Tests the reward calculation logic
- `insert_referral_from_event`: Tests event-based insertion
- `insert_referrals_bulk`: Tests bulk insertion of multiple referrals

## Running the Tests

### Prerequisites

1. **Docker** - Required for testcontainers to run Postgres
2. **Rust/Cargo** - Rust toolchain installed

### Execution

```bash
cd backend
cargo test --test referral_integration_tests --features integration-tests
```

Or run all integration tests:

```bash
cargo test --features integration-tests
```

### Test Container Lifecycle

- Each test starts a fresh Postgres container via `testcontainers-modules`
- The container is automatically stopped when the test completes
- Migrations are applied automatically before each test runs
- The pool is closed before the container is dropped

## Test Implementation Details

### Setup Function

```rust
async fn setup() -> (PgPool, testcontainers::ContainerAsync<Postgres>)
```

Bootstraps a Postgres container, runs all migrations, and returns:
- `PgPool` - SQLx connection pool
- `ContainerAsync` - Docker container handle (must be bound to a variable)

### Helper Functions

- `insert_pool(pool, pool_id, name)` - Creates a test pool
- `insert_prediction(pool, pool_id, user_address, amount)` - Inserts a prediction
- `insert_referral(pool, referrer, user_address, pool_id, amount)` - Inserts a referral
- `get_referrer_stats(pool, referrer)` - Retrieves referrer's global stats
- `get_referrer_pool_stats(pool, referrer, pool_id)` - Retrieves per-pool stats

### Test Pattern

Each test follows this structure:

```rust
#[tokio::test]
#[ignore = "Requires Docker container for Postgres"]
async fn test_name() {
    let (pool, _container) = setup().await;

    // Setup data
    insert_pool(&pool, 1, "Test Pool").await.unwrap();

    // Execute test logic
    insert_referral(&pool, referrer, user_address, 1, amount).await.unwrap();

    // Verify results
    let stats = get_referrer_stats(&pool, referrer).await.unwrap().unwrap();
    assert_eq!(stats.total_volume, expected);

    pool.close().await;
}
```

## Key Behaviors Verified

### Trigger-Based Statistics Maintenance

The tests verify that the `maintain_referrer_stats()` trigger correctly updates:

1. `referrer_users` - Tracks referrer-user pairs
2. `referrer_stats` - Global volume and unique users per referrer
3. `referrer_pool_stats` - Per-pool earnings and counts

### Idempotency

Tests confirm that:
- Duplicate referral inserts (same referrer + user + pool) don't cause errors
- Statistics are not double-counted for duplicates
- `ON CONFLICT DO NOTHING` works as expected

### Data Integrity

- Global totals match per-pool sums
- Unique user counts are accurate (same user, different pools)
- Zero-amount referrals are tracked correctly
- Large volumes don't cause overflow

## Writing New Tests

When adding new tests:

1. Follow the same setup pattern
2. Use the helper functions for common operations
3. Keep tests focused on a single behavior
4. Use descriptive test names
5. Add `#[ignore]` attribute (tests require Docker)
6. Verify both the `referrals` table and the pre-aggregated stats tables
