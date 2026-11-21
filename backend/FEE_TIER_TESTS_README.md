# Fee Tier Integration Tests

This document describes the integration tests for fee tier transitions in the PrediFi backend.

## Location

- Test file: `src/fee_tier_integration_tests.rs`

## Test Coverage

The 24 tests cover the following scenarios:

### 1. Fee Tier Boundary Tests
- **`fee_tier_boundary_at_threshold`** - Verifies correct fee at exact tier thresholds
- **`fee_below_all_tiers_uses_base_rate`** - Base fee applies when stake < all thresholds
- **`fee_at_exact_threshold_uses_tier_rate`** - Tier fee applies at exact thresholds
- **`fee_between_tiers_uses_lower_tier_rate`** - Lower tier fee applies between thresholds
- **`fee_tier_boundary_transitions`** - Tests transitions between consecutive tiers
- **`fee_multiple_tiers_descending_order`** - Multiple tiers in descending order

### 2. Mid-Pool Fee Change Tests
- **`fee_changes_as_pool_stake_increases`** - Fee changes as pool stake crosses thresholds
- **`new_pool_uses_current_fee_rate`** - New pools use current fee configuration

### 3. Treasury Accumulation Tests
- **`treasury_accumulates_fees_across_pools`** - Fees accumulate across multiple pools
- **`treasury_partial_withdrawal_accuracy`** - Partial treasury withdrawal accuracy
- **`treasury_withdrawal_to_multiple_recipients`** - Multiple recipient withdrawals
- **`treasury_accumulation_with_fee_changes`** - Accumulation with mid-period fee changes

### 4. Edge Case Tests
- **`fee_zero_stake`** - Zero stake fee (should be 0)
- **`fee_maximum_fee_rate`** - 100% fee rate (10,000 bps)
- **`fee_minimum_fee_rate`** - Minimum 1 bps fee rate
- **`fee_large_volume`** - Large volume overflow protection
- **`fee_fractional_rounding`** - Floor rounding of fractional fees

### 5. API Endpoint Tests
- **`api_fees_endpoint_returns_configured_values`** - `/api/v1/fees` endpoint validation

### 6. Formula Verification Tests
- **`fee_formula_verification`** - Fee formula correctness
- **`fee_tier_selection_logic`** - Tier selection logic
- **`fee_rates_monotonically_decreasing`** - Fee rates decrease with thresholds

### 7. Realistic Scenario Tests
- **`fee_realistic_scenarios`** - Realistic pool sizes and fee calculations

### 8. Utility Tests
- **`fee_idempotency`** - Same inputs produce same outputs
- **`fee_precision_tests`** - Various fee rate precision tests

## Fee Tier System Overview

### Fee Calculation Formula

```
fee = floor(stake × fee_bps / 10_000)
```

Where:
- `stake` = total pool stake in stroops (1 XLM = 10,000,000 stroops)
- `fee_bps` = fee rate in basis points (100 bps = 1%)
- Result = fee amount in stroops

### Tier Selection Logic

1. Find the highest `stake_threshold` where `pool.total_stake >= threshold`
2. If no threshold is met, use the base fee rate
3. Apply the corresponding `fee_bps` to calculate the fee

Example:
```
Base fee:     300 bps (3%)
Tier 1:       1,000 stake → 200 bps (2%)
Tier 2:       5,000 stake → 100 bps (1%)
Tier 3:       10,000 stake → 50 bps (0.5%)

Pool stake: 7,500 → Tier 2 applies (1% fee)
Pool stake: 10,001 → Tier 3 applies (0.5% fee)
```

## Running Tests

```bash
cd backend
cargo test --test fee_tier_integration_tests --features integration-tests
```

Or run all tests in the file:

```bash
cargo test --features integration-tests -- fee_tier_
```

## Key Behaviors Verified

### 1. Boundary Transitions

Tests confirm that:
- At `threshold - 1`: uses base or lower tier rate
- At `threshold`: uses current tier rate
- At `threshold + 1`: uses current tier rate (not base)

### 2. Mid-Pool Changes

Tests verify that:
- Fee changes are immediately effective for new pools
- Pool resolution uses the current fee configuration at resolution time
- Fee tier updates mid-pool affect subsequent pools

### 3. Treasury Accumulation

Tests ensure:
- Fees are correctly calculated per pool
- Multiple pools accumulate fees correctly
- Partial withdrawals reduce treasury balance accurately
- Full withdrawals empty treasury correctly

### 4. Edge Cases

Tests validate:
- Zero stake produces zero fee
- Maximum fee (100%) works correctly
- Minimum fee (1 bps = 0.01%) works correctly
- Large volumes don't cause overflow
- Fractional fees are floor-truncated

## Test Examples

### Example 1: Boundary at 10,000 stake threshold

```rust
// Stake 9,999 → Base fee 3%
fee = 9999 * 300 / 10000 = 299

// Stake 10,000 → Tier 3 fee 0.5%
fee = 10000 * 50 / 10000 = 50
```

### Example 2: Treasury accumulation

```rust
Pool 1: 1,000 stake at 3% → 30 fee
Pool 2: 2,500 stake at 2% → 50 fee
Pool 3: 10,000 stake at 0.5% → 50 fee

Total accumulated: 130
Partial withdrawal: 70 → remaining: 60
```

### Example 3: Mid-pool stake growth

```rust
Initial stake: 800 → below Tier 1 (1,000)
After bet 1: 800
After bet 2: 5,300 → crossed Tier 1 and Tier 2
Final stake: 11,300 → Tier 3 applies (0.5%)
```

## Implementation Details

### Helper Function

```rust
fn calculate_fee(stake: i64, fee_bps: u32) -> i64 {
    (stake as i128 * fee_bps as i128 / 10_000i128) as i64
}
```

### Test Categories

1. **Boundary Tests** (6 tests)
   - Exact threshold values
   - Just below/above thresholds
   - Multiple consecutive tiers

2. **Accumulation Tests** (4 tests)
   - Cross-pool fee aggregation
   - Partial withdrawals
   - Multiple recipients

3. **Edge Case Tests** (5 tests)
   - Zero/minimum/maximum values
   - Large volumes
   - Fractional rounding

4. **Realistic Scenarios** (3 tests)
   - Practical pool sizes
   - Common fee configurations

5. **Formula Verification** (6 tests)
   - Calculation correctness
   - Tier selection logic
   - Monotonicity

## Integration with Backend

The fee tier tests focus on:

1. **Fee Calculation Logic** - Core formula implementation
2. **Configuration Validation** - Fee configuration through `Config`
3. **API Endpoint** - `/api/v1/fees` returns correct values
4. **Integration Points** - Connection to `price_cache`, `redis_cache`, etc.

## Comparison with Contract Tests

- **Contract tests** (`fee_tier_transition_tests.rs`): Solidity/Soroban smart contract tests
- **Backend tests** (`fee_tier_integration_tests.rs`): Rust backend service tests

Both test suites verify:
- Fee calculation correctness
- Tier threshold transitions
- Treasury accumulation
- Edge cases

Backend tests focus on:
- API endpoints
- Configuration management
- Database integration
- Service-level behaviors
