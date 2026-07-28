# Error Handling Reference

This document provides comprehensive documentation for all error types across the PrediFi system, including smart contract errors (predifi-errors crate) and backend API errors (errors.rs).

## Table of Contents

- [Smart Contract Errors (predifi-errors)](#smart-contract-errors-predifi-errors)
- [Backend API Errors (errors.rs)](#backend-api-errors-errorsrs)
- [Error Code Mappings](#error-code-mappings)
- [Alert Severity Tiers](#alert-severity-tiers)

---

## Smart Contract Errors (predifi-errors)

The `predifi-errors` crate defines 33 error variants used across all PrediFi smart contracts. Each error includes a numeric code, category, machine-readable label, and human-readable message.

### Initialization & Configuration (Codes 1-2)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 1 | `NotInitialized` | `INIT_NOT_INITIALIZED` | No | Contract is not initialized. Call init before this operation. | Contract deployed but init() not called | Call the initialize function on the contract |
| 2 | `AlreadyInitializedOrConfigNotSet` | `INIT_ALREADY_INITIALIZED_OR_CONFIG_NOT_SET` | No | Contract already initialized or required config (treasury/access control) is missing | Attempting to reinitialize or missing required configuration | Ensure treasury and access control contracts are properly configured before initialization |

### Authorization & Access Control (Codes 10-11)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 10 | `Unauthorized` | `AUTH_UNAUTHORIZED` | Yes | Caller is not authorized to perform this action | Caller lacks required role or permission | Verify caller has appropriate role in access control contract |
| 11 | `InsufficientPermissions` | `AUTH_INSUFFICIENT_PERMISSIONS` | Yes | Caller role is missing or does not grant required permission | Role exists but doesn't grant specific permission | Grant required permissions to the caller's role |

### Pool State (Codes 20-26)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 20 | `PoolNotFound` | `POOL_NOT_FOUND` | Yes | Pool ID does not exist | Referenced pool doesn't exist in contract | Verify pool ID is correct and pool has been created |
| 21 | `PoolAlreadyResolved` | `POOL_ALREADY_RESOLVED` | Yes | Pool is already resolved | Attempting to resolve an already resolved pool | Check pool status before attempting resolution |
| 22 | `PoolNotResolved` | `POOL_NOT_RESOLVED` | Yes | Pool is not resolved yet | Attempting to claim before pool is resolved | Wait for pool resolution before claiming |
| 23 | `PoolExpiryError` | `POOL_EXPIRY_ERROR` | Yes | Pool expiry state is invalid for this operation | Pool expiry time constraints not met for operation | Verify pool is in correct time window for the operation |
| 24 | `InvalidPoolState` | `POOL_INVALID_STATE` | Yes | Invalid pool state | Pool is in an invalid state for the requested operation | Check pool state machine and ensure operation is valid for current state |
| 25 | `InvalidOutcome` | `POOL_INVALID_OUTCOME` | Yes | Invalid outcome or outcome index out of bounds | Outcome index exceeds number of options or is invalid | Verify outcome index is within valid range (0 to num_options-1) |
| 26 | `StateError` | `POOL_STATE_ERROR` | No | State inconsistency or invalid options count detected | Internal state corruption or invalid configuration | Contact support; may require contract upgrade or emergency intervention |

### Prediction & Betting (Codes 40-44)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 40 | `PredictionNotFound` | `PREDICTION_NOT_FOUND` | Yes | No prediction found for this user and pool | User hasn't placed a prediction on this pool | Place a prediction before attempting prediction-related operations |
| 41 | `PredictionAlreadyExists` | `PREDICTION_ALREADY_EXISTS` | Yes | User already placed a prediction in this pool | Attempting to place duplicate prediction | Users can only place one prediction per pool |
| 42 | `InvalidPredictionAmount` | `PREDICTION_INVALID_AMOUNT` | Yes | Invalid prediction amount (zero, negative, or invalid) | Amount is zero, negative, or exceeds limits | Provide a valid positive amount within stake limits |
| 43 | `PredictionTooLate` | `PREDICTION_TOO_LATE` | Yes | Prediction window has closed for this pool | Current time exceeds pool's prediction end time | Place predictions before the pool's prediction deadline |
| 44 | `InsufficientBalanceOrStakeLimit` | `PREDICTION_INSUFFICIENT_BALANCE_OR_STAKE_LIMIT` | Yes | Insufficient balance, below min stake, or above max stake limit | User lacks token balance or amount violates stake limits | Ensure sufficient token balance and amount is within min/max stake limits |

### Claiming & Rewards (Codes 60-62)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 60 | `AlreadyClaimed` | `CLAIM_ALREADY_CLAIMED` | Yes | Winnings already claimed for this pool | User already claimed winnings for this pool | Each pool can only be claimed once per user |
| 61 | `NotAWinner` | `CLAIM_NOT_A_WINNER` | Yes | User is not in a winning outcome for this pool | User's prediction didn't match the winning outcome | No action needed; user didn't win this pool |
| 62 | `RewardError` | `CLAIM_REWARD_ERROR` | No | Reward calculation failed, winning stake is zero, or payout exceeds pool | Calculation error or insufficient pool balance | Contact support; may indicate contract bug or configuration issue |

### Timestamp & Time Validation (Codes 80-81)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 80 | `InvalidTimestamp` | `TIME_INVALID_TIMESTAMP` | Yes | Invalid timestamp or time constraints not met | Timestamp is zero, in the past, or invalid | Provide a valid future timestamp |
| 81 | `TimeConstraintError` | `TIME_CONSTRAINT_ERROR` | Yes | End time or resolution time constraints are not met | Time constraints violated (e.g., end time before start time) | Ensure timestamps meet all time constraint requirements |

### Data & Validation (Codes 90-94)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 90 | `InvalidData` | `VALIDATION_INVALID_DATA` | Yes | Input data failed validation | Data structure or format is invalid | Verify data format matches expected schema |
| 91 | `InvalidAddressOrToken` | `VALIDATION_INVALID_ADDRESS_OR_TOKEN` | Yes | Provided address or token contract is invalid | Address is malformed or token contract doesn't exist | Provide valid Stellar address and token contract address |
| 92 | `InvalidPagination` | `VALIDATION_INVALID_PAGINATION` | Yes | Invalid pagination offset or limit | Pagination parameters are invalid | Provide valid offset (>=0) and limit (>0) values |
| 93 | `InvalidFeeBps` | `VALIDATION_INVALID_FEE_BPS` | Yes | Invalid fee basis points (max 10000) | Fee basis points exceed 10000 (100%) | Provide fee basis points between 0 and 10000 |
| 94 | `MetadataError` | `VALIDATION_METADATA_ERROR` | Yes | Metadata, label invalid/too long, or duplicate labels detected | Metadata validation failed | Ensure labels are unique, not too long, and metadata is valid |

### Arithmetic & Calculation (Codes 110-112)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 110 | `ArithmeticError` | `MATH_ARITHMETIC_ERROR` | Yes | Arithmetic overflow, underflow, or division-by-zero occurred | Calculation exceeded numeric bounds or divided by zero | Reduce input values or check for zero denominators |
| 111 | `FeeExceedsAmount` | `MATH_FEE_EXCEEDS_AMOUNT` | Yes | Calculated fee exceeds total amount | Fee configuration causes fee to be larger than amount | Adjust fee configuration or increase amount |
| 112 | `InvalidAmount` | `MATH_INVALID_AMOUNT` | Yes | Input amount is invalid or would cause arithmetic overflow | Amount is zero, negative, or would cause overflow | Provide valid positive amount within safe bounds |

### Storage & State (Codes 120-122)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 120 | `StorageError` | `STORAGE_ERROR` | No | Required storage key missing or storage is corrupted | Storage key not found or ledger data corrupted | Contact support; may require contract intervention |
| 121 | `ConsistencyError` | `STORAGE_CONSISTENCY_ERROR` | No | Pool stake or index inconsistency detected | Internal accounting mismatch between stake and index | Contact support; indicates potential state corruption |
| 122 | `BalanceMismatch` | `STORAGE_BALANCE_MISMATCH` | No | Contract token balance does not match internal accounting | Contract's actual token balance differs from tracked balance | Contact support; critical accounting discrepancy |

### Token & Transfer (Codes 150-151)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 150 | `TokenError` | `TOKEN_ERROR` | Yes | Token transfer/approval or token contract call failed | Token transfer failed or token contract error | Ensure sufficient allowance, balance, and token contract is operational |
| 151 | `WithdrawalOrTreasuryError` | `TOKEN_WITHDRAWAL_OR_TREASURY_ERROR` | Yes | Withdrawal or treasury transfer failed | Treasury withdrawal or transfer operation failed | Verify treasury has sufficient balance and transfer is authorized |

### Oracle & Resolution (Codes 160-161)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 160 | `OracleError` | `ORACLE_ERROR` | Yes | Oracle is not configured, returned invalid data, or data is stale | Oracle misconfiguration or invalid/stale data | Configure oracle or wait for fresh oracle data |
| 161 | `ResolutionError` | `ORACLE_RESOLUTION_ERROR` | Yes | Pool resolution failed due to duplicate attempt, mismatch, or unauthorized resolver | Duplicate resolution, outcome mismatch, or unauthorized resolver | Ensure resolver is authorized and resolution hasn't been performed |

### Emergency & Admin (Code 180)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 180 | `AdminError` | `ADMIN_ERROR` | No | Administrative operation failed (pause/emergency/version/upgrade) | Admin operation validation failed | Verify admin permissions and operation parameters |

### Rate Limiting & Spam Prevention (Code 190)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 190 | `RateLimitOrSuspiciousActivity` | `RATE_LIMIT_OR_SUSPICIOUS_ACTIVITY` | Yes | Rate limit exceeded, cooldown active, or suspicious activity detected | Too many operations or suspicious pattern detected | Wait for cooldown period or reduce operation frequency |

### Pool Configuration (Code 200)

| Code | Variant | Label | Recoverable | Message | Cause | Resolution |
|------|---------|-------|-------------|---------|-------|------------|
| 200 | `RequiredResolutionsExceedOperators` | `POOL_REQUIRED_RESOLUTIONS_EXCEED_OPERATORS` | Yes | Required resolutions exceeds the number of active operators; pool can never be resolved | Pool requires more resolutions than available operators | Reduce required_resolutions or add more operators |

---

## Backend API Errors (errors.rs)

The backend defines 5 high-level error types that map database and system errors to standardized API responses.

### Error Variants

| Variant | HTTP Status | Error Code | Database Mapping | Cause | Resolution |
|---------|-------------|------------|------------------|-------|------------|
| `NotFound` | 404 Not Found | `NOT_FOUND` | `sqlx::Error::RowNotFound` | Requested resource not found in database | Verify resource ID exists |
| `Conflict` | 409 Conflict | `CONFLICT` | PostgreSQL 23505 (unique_violation), 23503 (foreign_key_violation) | Constraint violation (duplicate, foreign key) | Ensure data uniqueness and referential integrity |
| `InvalidInput` | 400 Bad Request | `INVALID_INPUT` | PostgreSQL 23502 (not_null_violation) | Input validation failed or required field missing | Provide valid input data with all required fields |
| `ServiceUnavailable` | 503 Service Unavailable | `SERVICE_UNAVAILABLE` | - | Service temporarily unavailable (e.g., database connection failed) | Retry request after delay |
| `Internal` | 500 Internal Server Error | `INTERNAL_ERROR` | All other database errors | Unexpected server error | Contact support with request ID |

### PostgreSQL Error Code Mappings

| SQLSTATE | Error Name | Mapped To | Description |
|----------|------------|------------|-------------|
| 23502 | not_null_violation | `InvalidInput` | Required field is NULL |
| 23503 | foreign_key_violation | `Conflict` | Referenced key doesn't exist |
| 23505 | unique_violation | `Conflict` | Duplicate value in unique column |

---

## Error Code Mappings

### Backend Error Codes (response.rs)

| Error Code | String Constant | HTTP Status | Usage |
|------------|-----------------|-------------|-------|
| NOT_FOUND | `"NOT_FOUND"` | 404 | Resource not found |
| CONFLICT | `"CONFLICT"` | 409 | Constraint violation |
| INVALID_INPUT | `"INVALID_INPUT"` | 400 | Invalid request data |
| SERVICE_UNAVAILABLE | `"SERVICE_UNAVAILABLE"` | 503 | Service temporarily unavailable |
| INTERNAL_ERROR | `"INTERNAL_ERROR"` | 500 | Unexpected server error |
| DATABASE_UNAVAILABLE | `"DATABASE_UNAVAILABLE"` | 503 | Database connection failed |
| RATE_LIMIT_EXCEEDED | `"RATE_LIMIT_EXCEEDED"` | 429 | Too many requests |
| UNAUTHORIZED | `"UNAUTHORIZED"` | 401 | Authentication required |
| FORBIDDEN | `"FORBIDDEN"` | 403 | Insufficient permissions |

### Contract to Backend Mapping

While contract errors and backend errors serve different layers, here's the conceptual mapping:

| Contract Code | Contract Variant | Backend Equivalent | HTTP Status |
|---------------|-----------------|-------------------|-------------|
| 20 | `PoolNotFound` | `NotFound` | 404 |
| 21 | `PoolAlreadyResolved` | `Conflict` | 409 |
| 41 | `PredictionAlreadyExists` | `Conflict` | 409 |
| 42 | `InvalidPredictionAmount` | `InvalidInput` | 400 |
| 43 | `PredictionTooLate` | `InvalidInput` | 400 |
| 44 | `InsufficientBalanceOrStakeLimit` | `InvalidInput` | 400 |
| 60 | `AlreadyClaimed` | `Conflict` | 409 |
| 10 | `Unauthorized` | `Unauthorized` | 401 |
| 11 | `InsufficientPermissions` | `Forbidden` | 403 |
| 120-122 | Storage errors | `Internal` | 500 |
| 180 | `AdminError` | `Internal` | 500 |

---

## Alert Severity Tiers

### 🔴 HIGH — Page immediately; potential attack or critical bug

These errors indicate potential security issues, state corruption, or critical system failures requiring immediate attention.

| Contract Codes | Variants | Backend Equivalent |
|----------------|----------|-------------------|
| 10 | `Unauthorized` | `Unauthorized` |
| 11 | `InsufficientPermissions` | `Forbidden` |
| 120 | `StorageError` | `Internal` |
| 121 | `ConsistencyError` | `Internal` |
| 122 | `BalanceMismatch` | `Internal` |
| 160 | `OracleError` | `ServiceUnavailable` |
| 161 | `ResolutionError` | `Internal` |
| 180 | `AdminError` | `Internal` |
| 190 | `RateLimitOrSuspiciousActivity` | `RateLimitExceeded` |

**Monitoring Regex**: `Error\(Contract, #(10|11|120|121|122|160|161|180|190)\)`

### 🟡 MEDIUM — Alert within 15 minutes; user-impacting but not critical

These errors impact user experience but don't indicate system failure or security issues.

| Contract Codes | Variants | Backend Equivalent |
|----------------|----------|-------------------|
| 60 | `AlreadyClaimed` | `Conflict` |
| 62 | `RewardError` | `Internal` |
| 110 | `ArithmeticError` | `Internal` |
| 111 | `FeeExceedsAmount` | `InvalidInput` |
| 150 | `TokenError` | `Internal` |
| 151 | `WithdrawalOrTreasuryError` | `Internal` |

### 🟢 LOW — Log and review during business hours

These are expected user-facing validation errors that require no immediate action.

| Contract Codes | Variants | Backend Equivalent |
|----------------|----------|-------------------|
| 1, 2 | Initialization errors | `Internal` |
| 20-26 | Pool state errors | `NotFound`, `Conflict`, `InvalidInput` |
| 40-44 | Prediction errors | `NotFound`, `Conflict`, `InvalidInput` |
| 61 | `NotAWinner` | `InvalidInput` |
| 80-81 | Timestamp errors | `InvalidInput` |
| 90-94 | Validation errors | `InvalidInput` |
| 112 | `InvalidAmount` | `InvalidInput` |
| 200 | `RequiredResolutionsExceedOperators` | `InvalidInput` |

---

## API Response Format

All backend API errors follow a standardized JSON envelope:

```json
{
  "status": "error",
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "request_id": "uuid-for-tracing"
  }
}
```

### Example Responses

**404 Not Found:**
```json
{
  "status": "error",
  "error": {
    "code": "NOT_FOUND",
    "message": "Pool not found",
    "request_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

**409 Conflict:**
```json
{
  "status": "error",
  "error": {
    "code": "CONFLICT",
    "message": "Pool already resolved",
    "request_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

**400 Bad Request:**
```json
{
  "status": "error",
  "error": {
    "code": "INVALID_INPUT",
    "message": "Invalid prediction amount",
    "request_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

**429 Rate Limited:**
```json
{
  "status": "error",
  "error": "Too many requests"
}
```

---

## Error Categories

### Contract Error Categories

| Category | Description | Codes |
|----------|-------------|-------|
| `initialization` | Contract setup and configuration | 1-2 |
| `authorization` | Access control and permissions | 10-11 |
| `pool_state` | Pool lifecycle and state management | 20-26 |
| `prediction` | User prediction operations | 40-44 |
| `claiming` | Reward claiming operations | 60-62 |
| `timestamp` | Time-based validations | 80-81 |
| `validation` | Input data validation | 90-94 |
| `arithmetic` | Mathematical operations | 110-112 |
| `storage` | Ledger storage and state consistency | 120-122 |
| `token` | Token transfers and treasury operations | 150-151 |
| `oracle` | Oracle data and resolution | 160-161 |
| `admin` | Administrative operations | 180 |
| `rate_limiting` | Rate limiting and spam prevention | 190 |
| `pool_configuration` | Pool setup parameters | 200 |

---

## Best Practices

### For Backend Developers

1. **Use appropriate error types**: Map database errors to the correct `AppError` variant
2. **Include context**: Provide descriptive messages in error variants
3. **Log request IDs**: Always include request IDs for debugging
4. **Handle transient errors**: Implement retry logic for `ServiceUnavailable` errors
5. **Validate early**: Return `InvalidInput` before database operations when possible

### For Frontend Developers

1. **Check error codes**: Use the `code` field for programmatic error handling
2. **Display user-friendly messages**: Show the `message` field to users
3. **Handle rate limits**: Implement backoff for 429 responses
4. **Log request IDs**: Include request IDs in bug reports
5. **Distinguish recoverable errors**: Allow users to fix validation errors (400, 409) but not system errors (500)

### For Smart Contract Developers

1. **Use descriptive labels**: Error labels should be machine-readable and descriptive
2. **Categorize errors**: Use appropriate error codes for the category
3. **Emit events**: Pair errors with on-chain events for debugging
4. **Document recoverability**: Mark non-recoverable errors appropriately
5. **Test error paths**: Ensure all error variants are tested

---

## Monitoring and Observability

### Log Pattern for External Scrapers

Horizon returns contract errors as `Error(Contract, #<code>)` in the transaction result XDR. Use this regex to catch HIGH-severity errors:

```regex
Error\(Contract, #(10|11|120|121|122|160|161|180|190)\)
```

### Recommended Metrics

Track the following metrics for each error type:

- **Error rate**: Errors per minute/hour
- **Error distribution**: Percentage of each error type
- **Recoverable vs non-recoverable**: Ratio of user-fixable vs system errors
- **High-severity alerts**: Count of HIGH-tier errors

### Alert Thresholds

- **HIGH errors**: Alert immediately, page on-call
- **MEDIUM errors**: Alert if rate > 10/minute for 5 minutes
- **LOW errors**: Log only; alert if rate > 100/minute for 10 minutes

---

## Appendix: Error Code Quick Reference

### Contract Error Codes (Numeric Order)

```
1  - NotInitialized
2  - AlreadyInitializedOrConfigNotSet
10 - Unauthorized
11 - InsufficientPermissions
20 - PoolNotFound
21 - PoolAlreadyResolved
22 - PoolNotResolved
23 - PoolExpiryError
24 - InvalidPoolState
25 - InvalidOutcome
26 - StateError
40 - PredictionNotFound
41 - PredictionAlreadyExists
42 - InvalidPredictionAmount
43 - PredictionTooLate
44 - InsufficientBalanceOrStakeLimit
60 - AlreadyClaimed
61 - NotAWinner
62 - RewardError
80 - InvalidTimestamp
81 - TimeConstraintError
90 - InvalidData
91 - InvalidAddressOrToken
92 - InvalidPagination
93 - InvalidFeeBps
94 - MetadataError
110 - ArithmeticError
111 - FeeExceedsAmount
112 - InvalidAmount
120 - StorageError
121 - ConsistencyError
122 - BalanceMismatch
150 - TokenError
151 - WithdrawalOrTreasuryError
160 - OracleError
161 - ResolutionError
180 - AdminError
190 - RateLimitOrSuspiciousActivity
200 - RequiredResolutionsExceedOperators
```

### Backend Error Codes

```
NOT_FOUND              - 404
CONFLICT               - 409
INVALID_INPUT          - 400
SERVICE_UNAVAILABLE    - 503
INTERNAL_ERROR         - 500
DATABASE_UNAVAILABLE   - 503
RATE_LIMIT_EXCEEDED    - 429
UNAUTHORIZED           - 401
FORBIDDEN              - 403
```

---

## Related Documentation

- [predifi-errors crate README](../contract/contracts/predifi-errors/README.md)
- [Backend API Documentation](../backend/README.md)
- [Database Schema](../backend/db/schema.sql)
