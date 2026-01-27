### Description
Implements a query function to retrieve pool details by pool ID with proper error handling and gas-optimized storage access.

### Changes Made

**Data Structures:**
- Added `Pool` struct containing `pool_id`, `name`, `total_liquidity`, `token_a`, `token_b`, `fee_rate`, `is_active`, `status`, and `end_time` fields
- Added `DataKey` enum for type-safe storage key management
- Added `Error` enum with `PoolNotFound` error variant using `#[contracterror]` macro

**Functions:**
- `get_pool(pool_id: u64)` - Returns `Result<Pool, Error>` for pool retrieval
  - Uses persistent storage for data access
  - Returns `PoolNotFound` error for non-existent pools
  - Single storage lookup for gas efficiency
- `create_pool()` - Helper function for creating pools (used in tests)

**Tests:**
- `test_get_pool()` - Verifies successful pool retrieval with all fields
- `test_get_pool_not_found()` - Validates error handling for invalid pool IDs

### Gas Optimization
- Direct storage access with `.get()` and `.ok_or()` combinator
- Single lookup operation per query
- No unnecessary iterations or computations

### Acceptance Criteria
- Returns complete pool data structure
- Handles invalid pool IDs with appropriate errors
- Gas-optimized implementation
- All tests passing
```