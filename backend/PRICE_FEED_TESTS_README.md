# Price Feed Integration Tests

This document describes the integration tests for price feed interactions during pool resolution.

## Location

- Test file: `src/price_feed_integration_tests.rs`

## Test Coverage

The 35 tests cover the following scenarios:

### 1. Price Feed Configuration Tests (6 tests)
- **`price_cache_multiple_oracle_sources`** - Multiple oracle sources for price data
- **`price_cache_fallback_oracle_sources`** - Fallback when primary source fails
- **`price_cache_multiple_feeds_same_asset`** - Multiple feeds for same asset
- **`oracle_whitelist_verification`** - Oracle whitelist validation
- **`oracle_priority_configuration`** - Oracle priority ordering
- **`oracle_health_check`** - Oracle health monitoring

### 2. Price Feed Staleness Handling Tests (4 tests)
- **`price_cache_detects_stale_prices`** - Detect stale price data
- **`price_cache_handles_expired_prices`** - Handle expired price feeds
- **`price_cache_mixed_stale_fresh_prices`** - Mixed stale/fresh price scenarios
- **`resolution_rejects_stale_price`** - Reject stale prices during resolution

### 3. Price Feed Failure Scenarios Tests (6 tests)
- **`price_cache_empty_feed`** - Empty price feed handling
- **`price_cache_missing_oracle_sources`** - Missing oracle sources
- **`price_cache_invalid_price_values`** - Invalid price values (negative, zero, infinite)
- **`price_cache_extreme_price_values`** - Extreme price values
- **`price_cache_timeout_scenario`** - Price feed timeout handling
- **`price_cache_oracle_connection_failure`** - Connection failure behavior

### 4. Resolution Correctness Tests (8 tests)
- **`resolution_exact_threshold_calculation`** - Exact threshold resolution
- **`resolution_above_threshold`** - Above threshold resolution
- **`resolution_below_threshold`** - Below threshold resolution
- **`resolution_with_price_tolerance`** - Price tolerance handling
- **`resolution_multiple_price_feeds`** - Multiple price feed resolution
- **`resolution_handles_price_feed_failure`** - Price feed failure handling
- **`resolution_oracle_fallback`** - Oracle fallback resolution
- **`fee_calculation_at_thresholds`** - Fee calculation verification

### 5. Multi-Oracle Tests (5 tests)
- **`multi_oracle_consensus_resolution`** - Oracle consensus for resolution
- **`multi_oracle_disagreement_handling`** - Handling oracle disagreements
- **`multi_oracle_outlier_rejection`** - Outlier rejection from oracles
- **`price_tolerance_bands`** - Price tolerance band verification
- **`price_condition_comparison_operators`** - Comparison operator tests

### 6. Edge Case Tests (6 tests)
- **`resolution_zero_pool_stake`** - Zero stake edge case
- **`resolution_maximum_price_threshold`** - Maximum price threshold
- **`resolution_negative_price_scenario`** - Negative price handling
- **`resolution_large_price_values`** - Large price values
- **`resolution_floating_point_precision`** - Floating point precision
- **`treasury_accumulation_across_pools`** - Treasury accumulation

## Price Feed System Overview

### Oracle Sources

The system supports multiple oracle sources:
- **Pyth Network** - High-frequency decentralized price feeds
- **CoinGecko** - Cryptocurrency price aggregation
- **Stellar Price Oracle** - Native price feeds

### Price Feed Configuration

```rust
// Oracle whitelist
whitelisted_oracles: ["oracle1", "oracle2", "oracle3"]

// Priority configuration (higher = more trusted)
oracles: [
    ("pyth", 100),
    ("coingecko", 80),
    ("kraken", 60),
]
```

### Resolution Logic

1. Check if price feed is fresh (not stale)
2. Verify oracle is whitelisted
3. Evaluate price condition against threshold
4. Calculate fees based on tier configuration
5. Distribute winnings to winners

### Staleness Detection

Price feeds have an `expires_at` timestamp. Resolution rejects:
- Price feeds where `expires_at < resolution_time`
- Price feeds with invalid timestamps
- Price feeds from untrusted oracles

### Failure Handling

The system handles failures gracefully:
- Empty price cache: Returns error
- Oracle connection failure: Uses cached data
- Price feed timeout: Returns error
- Invalid price values: Returns error

## Running Tests

```bash
cd backend
cargo test --test price_feed_integration_tests --features integration-tests
```

Or run all tests in the file:

```bash
cargo test --features integration-tests -- price_feed_
```

## Key Behaviors Verified

### 1. Multi-Oracle Support

Tests confirm:
- Multiple oracles can be whitelisted
- Priority configuration works correctly
- Health checks identify unhealthy oracles
- Fallback to secondary oracles on failure

### 2. Staleness Detection

Tests verify:
- Stale price feeds are rejected
- Expired feeds return errors
- Mixed stale/fresh prices are handled
- Timestamp validation works correctly

### 3. Failure Scenarios

Tests ensure:
- Empty feeds don't cause panics
- Missing oracles are handled
- Invalid prices are rejected
- Connection failures don't crash the system

### 4. Resolution Correctness

Tests confirm:
- Price conditions evaluate correctly
- Fee calculations are accurate
- Treasury accumulation works
- Tolerance bands function properly

### 5. Edge Cases

Tests validate:
- Zero stake resolution
- Maximum price values
- Negative price rejection
- Large volume handling
- Floating point precision

## Implementation Details

### Price Cache

The `PriceCache` maintains an in-memory snapshot of prices:

```rust
let cache = PriceCache::new();
cache.update(prices);
let snapshot = cache.snapshot();
```

### Oracle Whitelist

Oracles are verified before price feed updates:

```rust
if whitelisted_oracles.contains(oracle) {
    // Process price feed
} else {
    // Reject with Unauthorized error
}
```

### Resolution Logic

```rust
if current_price >= threshold {
    // Resolve to winning outcome
} else {
    // Resolve to losing outcome
}
```

### Fee Calculation

```rust
fn calculate_fee(stake: i64, fee_bps: u32) -> i64 {
    (stake as i128 * fee_bps as i128 / 10_000i128) as i64
}
```

## Comparison with Contract Tests

- **Contract tests** (`price_feed_integration_test.rs`): Solidity/Soroban smart contract tests
- **Backend tests** (`price_feed_integration_tests.rs`): Rust backend service tests

Both test suites verify:
- Oracle whitelist functionality
- Price feed validation
- Resolution logic
- Staleness detection

Backend tests focus on:
- In-memory price cache
- API endpoint responses
- Service-level behaviors
- Error handling scenarios

## Integration Points

The price feed tests integrate with:

1. **PriceCache** - In-memory price storage
2. **RedisCache** - Distributed price caching (optional)
3. **Health Checks** - Price feed health monitoring
4. **API Endpoints** - `/api/v1/fees` returns price info
