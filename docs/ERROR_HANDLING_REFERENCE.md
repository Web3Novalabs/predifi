# PrediFi Error Handling Reference

**Version**: 1.0.0  
**Scope**: Smart Contract Errors (`predifi-errors` crate & `predifi-contract`), Native Backend Errors (`errors.rs` & `response.rs`), and Cross-Layer Mappings.

---

## Table of Contents

1. [Overview & Error Architecture](#overview--error-architecture)
2. [Smart Contract Errors (`predifi-errors` Crate)](#smart-contract-errors-predifi-errors-crate)
3. [Contract-Specific Errors (`predifi-contract`)](#contract-specific-errors-predifi-contract)
4. [Native Configuration Errors (`native_errors.rs`)](#native-configuration-errors-native_errorsrs)
5. [Backend Application Errors (`errors.rs`)](#backend-application-errors-errorsrs)
6. [Backend API Response Envelopes & Error Codes (`response.rs`)](#backend-api-response-envelopes--error-codes-responsers)
7. [Comprehensive Error Mapping Table](#comprehensive-error-mapping-table)
8. [Alert Severity Tiers & Monitoring Guide](#alert-severity-tiers--monitoring-guide)
9. [Client-Side Error Handling Best Practices](#client-side-error-handling-best-practices)

---

## Overview & Error Architecture

PrediFi spans multiple runtime environments:
- **On-chain Layer (Soroban VM)**: Smart contracts compile to WASM (`no_std`). Contract errors are emitted as discrete numeric codes matching the `#[contracterror]` enum, visible in transaction simulation and final Horizon transaction result XDRs as `Error(Contract, #<code>)`.
- **Backend Service Layer (Rust / Axum)**: Relies on `AppError` to wrap lower-level database (PostgreSQL / SQLx), Redis, RPC, and serialization failures into structured JSON error envelopes with HTTP status codes and unique tracing request UUIDs.
- **Frontend / Client Layer (Next.js / TypeScript)**: Decodes on-chain simulation failures and backend JSON errors to display actionable feedback to end users.

---

## Smart Contract Errors (`predifi-errors` Crate)

The `predifi-errors` crate provides the canonical error enum `PrediFiError` shared across contracts. It organizes errors into functional categories with numeric gap-based ranges.

### 1. Initialization & Configuration (Codes 1–2)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **1** | `NotInitialized` | `INIT_NOT_INITIALIZED` | `initialization` | Contract is not initialized. Call init before this operation. | An invocation targeted a contract method that requires initialized state, but `init()` has not been called. | Run the contract initialization invocation with valid admin/parameters before interacting. |
| **2** | `AlreadyInitializedOrConfigNotSet` | `INIT_ALREADY_INITIALIZED_OR_CONFIG_NOT_SET` | `initialization` | Contract already initialized or required config is missing. | An attempt was made to re-invoke `init()` on an already configured contract instance, or config is missing. | Verify contract instance status. Do not re-initialize an active deployment. |

### 2. Authorization & Access Control (Codes 10–11)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **10** | `Unauthorized` | `AUTH_UNAUTHORIZED` | `authorization` | Caller is not authorized to perform this action. | The caller's address lacks the required role (e.g. Admin 0, Operator 1, Oracle 3) or caller signature verification failed. | Ensure the transaction is signed by an account possessing the appropriate role in `access-control`. |
| **11** | `InsufficientPermissions` | `AUTH_INSUFFICIENT_PERMISSIONS` | `authorization` | Caller role is missing or does not grant required permission. | Caller has a registered identity, but the specific role or permission bitmask is absent. | Admin must assign the requisite role to the address via `assign_role` on `access-control`. |

### 3. Pool State & Lifecycle (Codes 20–26)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **20** | `PoolNotFound` | `POOL_NOT_FOUND` | `pool_state` | Pool ID does not exist. | The specified `pool_id` has not been registered or was pruned from storage. | Confirm the `pool_id` against the backend indexer or explorer. |
| **21** | `PoolAlreadyResolved` | `POOL_ALREADY_RESOLVED` | `pool_state` | Pool is already resolved. | An operator or oracle attempted to resolve a pool that is already in `MarketState::Resolved`. | Resolution is one-way and irreversible. No further resolution is allowed. |
| **22** | `PoolNotResolved` | `POOL_NOT_RESOLVED` | `pool_state` | Pool is not resolved yet. | A user attempted to invoke `claim_winnings` before the pool has been resolved. | Wait until pool reaches `MarketState::Resolved` after the resolution window. |
| **23** | `PoolExpiryError` | `POOL_EXPIRY_ERROR` | `pool_state` | Pool expiry state is invalid for this operation. | Operation was submitted outside of the allowed lifecycle window. | Check `end_time` and current ledger timestamp before submitting. |
| **24** | `InvalidPoolState` | `POOL_INVALID_STATE` | `pool_state` | Invalid pool state for the requested operation. | Attempting an action incompatible with current `MarketState` (e.g., placing predictions on a canceled/resolved pool). | Verify pool state (`Active`, `Resolved`, `Canceled`, `Disputed`) before calling. |
| **25** | `InvalidOutcome` | `POOL_INVALID_OUTCOME` | `pool_state` | Invalid outcome or outcome index out of bounds. | Selected outcome index is outside `0 .. options_count - 1`. | Pass an outcome index within the valid range for the pool. |
| **26** | `StateError` | `POOL_STATE_ERROR` | `pool_state` | State inconsistency or invalid options count detected. | Contract detected an internal ledger inconsistency or invalid option bounds. | Critical integrity alert; inspect contract storage and pause if necessary. |

### 4. Prediction & Betting (Codes 40–44)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **40** | `PredictionNotFound` | `PREDICTION_NOT_FOUND` | `prediction` | No prediction found for this user and pool. | A query or claim was made for a user who never placed a stake on the pool. | Check the user's prediction history on that `pool_id`. |
| **41** | `PredictionAlreadyExists` | `PREDICTION_ALREADY_EXISTS` | `prediction` | User already placed a prediction in this pool. | The contract enforces a single prediction entry per user per pool. | Additional stakes must be placed from a separate address or wait for multi-bet support. |
| **42** | `InvalidPredictionAmount` | `PREDICTION_INVALID_AMOUNT` | `prediction` | Invalid prediction amount (zero, negative, or invalid). | Stake amount is $\le 0$ or violates numeric formatting constraints. | Provide a positive stake amount in stroops exceeding the minimum stake threshold. |
| **43** | `PredictionTooLate` | `PREDICTION_TOO_LATE` | `prediction` | Prediction window has closed for this pool. | The current ledger timestamp is $\ge$ `pool.end_time`. | Place predictions before the pool's designated staking close deadline. |
| **44** | `InsufficientBalanceOrStakeLimit` | `PREDICTION_INSUFFICIENT_BALANCE_OR_STAKE_LIMIT` | `prediction` | Insufficient balance, below min stake, or above max stake limit. | User lacks token allowance/balance, or amount violates `min_stake`, `max_stake`, or `max_total_stake`. | Check wallet balance, token approval allowance, and verify pool stake bounds. |

### 5. Claiming & Rewards (Codes 60–62)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **60** | `AlreadyClaimed` | `CLAIM_ALREADY_CLAIMED` | `claiming` | Winnings already claimed for this pool. | User attempted to claim payout or refund twice for the same prediction. | Each winning prediction can only be claimed once. |
| **61** | `NotAWinner` | `CLAIM_NOT_A_WINNER` | `claiming` | User is not in a winning outcome for this pool. | The user's staked outcome does not match the winning outcome decided at resolution. | Only predictions matching the winning outcome are eligible for prize payouts. |
| **62** | `RewardError` | `CLAIM_REWARD_ERROR` | `claiming` | Reward calculation failed, winning stake is zero, or payout exceeds pool. | Mathematical anomaly, or zero winning stake pool requiring refund path. | If all winning stakes are zero, the pool enters refund mode via `claim_refund`. |

### 6. Timestamps & Time Constraints (Codes 80–81)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **80** | `InvalidTimestamp` | `TIME_INVALID_TIMESTAMP` | `timestamp` | Invalid timestamp or time constraints not met. | Timestamp is in the past, zero, or exceeds upper scheduling boundaries. | Specify a valid future timestamp within permissible bounds. |
| **81** | `TimeConstraintError` | `TIME_CONSTRAINT_ERROR` | `timestamp` | End time or resolution time constraints are not met. | The mandatory resolution delay or timelock period has not elapsed yet. | Wait for the resolution delay / timelock to expire before triggering resolution. |

### 7. Validation & Data Formatting (Codes 90–94)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **90** | `InvalidData` | `VALIDATION_INVALID_DATA` | `validation` | Input data failed validation. | Generic input validation failure (e.g. empty string, zero length array). | Ensure input fields comply with required schemas and limits. |
| **91** | `InvalidAddressOrToken` | `VALIDATION_INVALID_ADDRESS_OR_TOKEN` | `validation` | Provided address or token contract is invalid. | Malformed Stellar address, unregistered token, or non-whitelisted asset. | Supply a valid, whitelisted SEP-41 token contract address. |
| **92** | `InvalidPagination` | `VALIDATION_INVALID_PAGINATION` | `validation` | Invalid pagination offset or limit. | `limit == 0`, `limit > MAX_PAGE_SIZE`, or offset calculation overflow. | Use pagination limit between 1 and 100 with non-negative offset. |
| **93** | `InvalidFeeBps` | `VALIDATION_INVALID_FEE_BPS` | `validation` | Invalid fee basis points (max 10000). | Fee configuration exceeds 10,000 basis points ($100\%$). | Fee basis points must be between 0 and 10,000. |
| **94** | `MetadataError` | `VALIDATION_METADATA_ERROR` | `validation` | Metadata, label invalid/too long, or duplicate labels detected. | Metadata URL exceeds 512 bytes or option descriptions contain duplicates. | Shorten metadata URL and ensure outcome option strings are unique. |

### 8. Arithmetic & Calculations (Codes 110–112)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **110** | `ArithmeticError` | `MATH_ARITHMETIC_ERROR` | `arithmetic` | Arithmetic overflow, underflow, or division-by-zero occurred. | Unchecked math failure during stake accumulation or payout calculation. | Validate numerical input sizes and non-zero denominators. |
| **111** | `FeeExceedsAmount` | `MATH_FEE_EXCEEDS_AMOUNT` | `arithmetic` | Calculated fee exceeds total amount. | Calculated protocol fee is larger than the gross payout amount. | Ensure fee basis points and calculation logic preserve net payout. |
| **112** | `InvalidAmount` | `MATH_INVALID_AMOUNT` | `arithmetic` | Input amount is invalid or would cause arithmetic overflow. | Numerical parameter exceeds maximum safe integer limits ($i128$). | Restrict stakes and token inputs to safe integer ranges. |

### 9. Storage & State Integrity (Codes 120–122)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **120** | `StorageError` | `STORAGE_ERROR` | `storage` | Required storage key missing or storage is corrupted. | Missing persistent ledger entry that was expected to exist. | Check ledger TTL and renew with `extend_persistent` if archived. |
| **121** | `ConsistencyError` | `STORAGE_CONSISTENCY_ERROR` | `storage` | Pool stake or index inconsistency detected. | Sum of outcome stakes does not equal pool total stake (Invariant INV-1 violated). | Critical system bug; pause contract and conduct storage inspection. |
| **122** | `BalanceMismatch` | `STORAGE_BALANCE_MISMATCH` | `storage` | Contract token balance does not match internal accounting. | Contract vault token balance is less than required reserve for active stakes. | Critical security alert; inspect for unauthorized withdrawals. |

### 10. Tokens & Treasury (Codes 150–151)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **150** | `TokenError` | `TOKEN_ERROR` | `token` | Token transfer/approval or token contract call failed. | Soroban cross-contract token transfer rejected (insufficient funds/allowance). | Verify caller has granted sufficient token allowance to the PrediFi contract. |
| **151** | `WithdrawalOrTreasuryError` | `TOKEN_WITHDRAWAL_OR_TREASURY_ERROR` | `token` | Withdrawal or treasury transfer failed. | Treasury contract rejected fee disbursement or balance is insufficient. | Verify treasury account state and recipient authorization. |

### 11. Oracles & Automated Resolution (Codes 160–161)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **160** | `OracleError` | `ORACLE_ERROR` | `oracle` | Oracle is not configured, returned invalid data, or data is stale. | Pyth feed unavailable, price older than `max_price_age`, or confidence interval too wide. | Update oracle price feed with recent data before triggering resolution. |
| **161** | `ResolutionError` | `ORACLE_RESOLUTION_ERROR` | `oracle` | Pool resolution failed due to duplicate attempt, mismatch, or unauthorized resolver. | Resolution quorum not met, consensus conflict, or unauthorized caller. | Gather necessary operator votes or satisfy oracle threshold. |

### 12. Administration, Governance & Rate Limiting (Codes 180–200)

| Code | Variant | Label | Category | Message | Cause | Suggested Resolution |
|------|---------|-------|----------|---------|-------|----------------------|
| **180** | `AdminError` | `ADMIN_ERROR` | `admin` | Administrative operation failed (pause/emergency/version/upgrade). | Unauthorized upgrade, invalid bytecode hash, or timelock active. | Check admin role authorization and timelock parameters. |
| **190** | `RateLimitOrSuspiciousActivity` | `RATE_LIMIT_OR_SUSPICIOUS_ACTIVITY` | `rate_limiting` | Rate limit exceeded, cooldown active, or suspicious activity detected. | User submitted transactions faster than `prediction_cooldown_seconds`. | Wait for cooldown interval to expire before submitting another prediction. |
| **200** | `RequiredResolutionsExceedOperators` | `POOL_REQUIRED_RESOLUTIONS_EXCEED_OPERATORS` | `pool_configuration` | Required resolutions exceeds active operators; pool can never resolve. | Pool creation specified quorum higher than active registered operators. | Lower `required_resolutions` or register additional operators in `access-control`. |

---

## Contract-Specific Errors (`predifi-contract`)

In addition to the shared enum, `predifi-contract` defines specialized domain variants for fine-grained failure diagnosis:

| Code | Variant | Category | Meaning | Suggested Resolution |
|------|---------|----------|---------|----------------------|
| **27** | `PoolDisputed` | `pool_state` | Pool has been flagged as disputed by a moderator. | Await resolution or refund decision by arbitration. |
| **45** | `InsufficientStake` | `prediction` | Stake is below the global protocol minimum (`DEFAULT_GLOBAL_MIN_STAKE`). | Increase stake to meet the protocol minimum. |
| **70** | `PoolCanceled` | `pool_state` | Pool was canceled by creator or emergency admin. | Claim full stake refund via `claim_refund`. |
| **82** | `StakingStillOpen` | `timestamp` | `close_staking` invoked before `pool.end_time` passed. | Wait until pool end time has expired. |
| **83** | `ContractPaused` | `admin` | Contract is paused; state-mutating operations blocked. | Wait until admin resumes contract operations via `unpause`. |
| **84** | `TimeConstraintError` | `timestamp` | Claim or resolution window constraint not satisfied. | Verify the time window for the requested operation. |
| **95** | `FeeChangePending` | `admin` | A timelocked fee change is already pending. | Apply or cancel the pending proposal before queuing another. |
| **96** | `NoFeeChangePending` | `admin` | `apply_fee_bps` invoked but no proposal is queued. | Propose a fee change via `set_fee_bps` first. |
| **97** | `TimelockNotExpired` | `admin` | Attempted to apply fee change before 7-day timelock. | Wait until `effective_at` ledger timestamp is reached. |
| **100** | `OracleNotInitialized` | `oracle` | Pyth oracle configuration not initialized. | Admin must invoke `init_oracle` first. |
| **101** | `PriceFeedNotFound` | `oracle` | Referenced `feed_pair` does not exist in registry. | Register feed pair or update price feed first. |
| **102** | `PriceDataInvalid` | `oracle` | Price feed data expired or confidence exceeds threshold. | Publish fresh price feed data with tight confidence interval. |
| **103** | `PriceConditionNotSet` | `oracle` | Pool has no linked price condition for price resolution. | Set condition via `set_price_condition` or use operator resolution. |
| **104** | `MaxTotalStakeExceeded` | `prediction` | New stake would cause pool to exceed `max_total_stake`. | Reduce stake size or place prediction on another pool. |
| **105** | `ResolutionConflict` | `oracle` | Oracles or operators disagreed on the outcome. | Moderator intervention required to resolve consensus dispute. |
| **106** | `OracleAlreadyVoted` | `oracle` | Oracle/operator has already cast a vote for this pool. | Each operator may only submit one resolution vote per pool. |
| **107** | `StakeBelowMinimum` | `prediction` | Stake is below the pool's configured `min_stake`. | Increase stake to satisfy pool minimum. |
| **108** | `StakeAboveMaximum` | `prediction` | Stake exceeds the pool's configured `max_stake`. | Decrease stake to stay within pool maximum limit. |
| **109** | `MetadataUrlInvalid` | `validation` | Metadata URL exceeds 512 bytes limit. | Shorten IPFS / HTTPS metadata URL string. |
| **111** | `MaxPredictionsExceeded` | `prediction` | User exceeded `max_predictions_per_user` for the pool. | User has reached maximum allowed prediction allocations. |
| **113** | `PriceStale` | `oracle` | Price is older than the configured `max_price_age`. | Refresh price feed with recent observation before resolving. |
| **130** | `OutcomeDescriptionTooLong` | `validation` | Outcome label exceeds `MAX_OUTCOME_DESCRIPTION_LEN`. | Shorten description label string. |
| **131** | `OutcomeDescriptionEmpty` | `validation` | Outcome label is empty or whitespace only. | Provide non-empty description for each outcome option. |
| **132** | `DeadlineInPast` | `timestamp` | Specified deadline timestamp is in the past. | Supply future timestamp relative to current ledger time. |
| **133** | `InitialLiquidityBelowSafetyMargin` | `pool_state` | Initial liquidity does not satisfy safety margin. | Increase creator liquidity deposit for pool backing. |
| **134** | `EmergencyCancelPending` | `admin` | Multisig cancellation threshold not yet reached. | Collect remaining required admin approvals. |
| **135** | `EmergencyCancelAlreadyApproved` | `admin` | Caller already approved this cancellation proposal. | Duplicate approval detected; await other signers. |
| **201** | `InvalidTargetPrice` | `oracle` | `target_price` is zero or negative. | Provide strictly positive target price. |

---

## Native Configuration Errors (`native_errors.rs`)

When compiling the `predifi-errors` crate with the `std` feature enabled for the backend service, configuration errors are represented by `ConfigError`:

| Variant | Cause | Suggested Resolution |
|---------|-------|----------------------|
| `ConfigError::InvalidNumber { key, value, reason }` | An environment variable was set but failed to parse into the target numeric type (e.g. `RPC_HEALTH_TIMEOUT_SECS="abc"`). | Verify environment variable formatting in `.env` or deployment manifests to ensure valid numbers. |
| `ConfigError::InvalidValue { key, reason }` | A configuration value violates a semantic constraint (e.g. invalid URL, negative duration, or conflicting ports). | Adjust the environment setting to satisfy the documented constraint. |

---

## Backend Application Errors (`errors.rs`)

The backend API uses `AppError` to capture all system, database, and business logic faults, mapping them cleanly to HTTP status codes and API responses:

```rust
pub enum AppError {
    NotFound(String),
    Conflict(String),
    InvalidInput(String),
    ServiceUnavailable(String),
    Internal(String),
}
```

### PostgreSQL SQLSTATE Mapping Matrix

When executing SQL queries through SQLx, raw database errors are intercepted and translated into `AppError` variants based on their standard 5-character SQLSTATE code:

| SQLSTATE | Postgres Error Name | Mapped `AppError` | HTTP Status | Response Code | Description |
|----------|---------------------|-------------------|-------------|---------------|-------------|
| **23505** | `unique_violation` | `AppError::Conflict` | 409 Conflict | `CONFLICT` | Attempted to insert duplicate unique key (e.g. pool ID or transaction hash). |
| **23503** | `foreign_key_violation` | `AppError::Conflict` | 409 Conflict | `CONFLICT` | Referenced foreign entity does not exist (e.g. prediction on non-existent pool). |
| **23502** | `not_null_violation` | `AppError::InvalidInput` | 400 Bad Request | `INVALID_INPUT` | Required non-nullable database column missing in payload. |
| - | `sqlx::Error::RowNotFound` | `AppError::NotFound` | 404 Not Found | `NOT_FOUND` | Queried record does not exist in database tables. |
| - | `sqlx::Error::PoolTimedOut` | `AppError::ServiceUnavailable` | 503 Unavailable | `SERVICE_UNAVAILABLE` | Database connection pool exhausted or database server unreachable. |
| *Other* | Any other error | `AppError::Internal` | 500 Server Error | `INTERNAL_ERROR` | Internal server or unhandled SQL execution failure. |

---

## Backend API Response Envelopes & Error Codes (`response.rs`)

All error responses from the PrediFi API conform to a uniform envelope:

```json
{
  "status": "error",
  "error": {
    "code": "STRING_ERROR_CODE",
    "message": "Human-readable explanation of the error",
    "request_id": "a5c7f891-b34e-4f70-9832-6a7c88b90123"
  }
}
```

### Response Error Code Reference

| Error Code | HTTP Status | Meaning | Typical Trigger |
|------------|-------------|---------|-----------------|
| `INVALID_INPUT` | 400 | Request parameters or body failed validation. | Missing query param, out-of-bounds limit, malformed G... address. |
| `UNAUTHORIZED` | 401 | Missing or invalid Bearer JWT authentication. | Expired access token, malformed signature, missing header. |
| `FORBIDDEN` | 403 | Caller lacks required authorization. | Non-admin attempting to alter log level or creator routes. |
| `NOT_FOUND` | 404 | Requested resource could not be found. | Pool ID or user address not present in database. |
| `CONFLICT` | 409 | Request conflicts with current resource state. | Duplicate event ingestion or state machine conflict. |
| `RATE_LIMIT_EXCEEDED` | 429 | IP address exceeded token bucket quota. | High-frequency API polling or scraping. |
| `INTERNAL_ERROR` | 500 | Unhandled internal server exception. | Unexpected query failure or unhandled panic. |
| `SERVICE_UNAVAILABLE` | 503 | Core dependency temporarily unavailable. | Database down, Redis unreachable, or Stellar RPC lagging. |
| `DATABASE_UNAVAILABLE` | 503 | Database pool uninitialized or disconnected. | Backend running in stateless health-only mode. |

---

## Comprehensive Error Mapping Table

This master table correlates errors across all system layers: **Contract Error Code**, **Backend HTTP Status**, **Backend Error Code**, **Root Cause**, and **Suggested Resolution**.

| Error Name / Variant | Contract Code | HTTP Status | Backend Error Code | Cause | Suggested Resolution |
|----------------------|---------------|-------------|--------------------|-------|----------------------|
| `NotInitialized` | **#1** | 500 | `INTERNAL_ERROR` | Contract deployed but `init()` not invoked. | Deployer must run contract initialization transaction. |
| `AlreadyInitializedOrConfigNotSet` | **#2** | 409 | `CONFLICT` | Re-initialization attempt or missing config. | Verify contract deployment status; do not re-run init. |
| `Unauthorized` | **#10** | 401 | `UNAUTHORIZED` | Missing role or invalid cryptographic signature. | Ensure caller signs transaction with authorized key. |
| `InsufficientPermissions` | **#11** | 403 | `FORBIDDEN` | Caller has account but lacks role permission. | Grant role via `assign_role` on `access-control`. |
| `PoolNotFound` | **#20** | 404 | `NOT_FOUND` | Specified pool ID does not exist. | Verify pool ID with explorer or backend API. |
| `PoolAlreadyResolved` | **#21** | 409 | `CONFLICT` | Attempt to resolve an already resolved pool. | Check pool state before submitting resolution. |
| `PoolNotResolved` | **#22** | 400 | `INVALID_INPUT` | Attempt to claim winnings before resolution. | Wait for pool to transition to `Resolved` state. |
| `PoolExpiryError` | **#23** | 400 | `INVALID_INPUT` | Action outside permitted lifecycle time window. | Verify ledger timestamp against pool end time. |
| `InvalidPoolState` | **#24** | 409 | `CONFLICT` | State machine transition disallowed. | Verify pool is in `Active` status before betting. |
| `InvalidOutcome` | **#25** | 400 | `INVALID_INPUT` | Outcome index exceeds options count. | Choose an outcome between `0` and `options_count - 1`. |
| `StateError` | **#26** | 500 | `INTERNAL_ERROR` | Internal ledger state corruption. | Contract admin must investigate ledger storage. |
| `PoolDisputed` | **#27** | 409 | `CONFLICT` | Pool is currently under dispute review. | Await arbitrator review or emergency cancel. |
| `PredictionNotFound` | **#40** | 404 | `NOT_FOUND` | No prediction registered for this user/pool. | Place a prediction before querying or claiming. |
| `PredictionAlreadyExists` | **#41** | 409 | `CONFLICT` | User already has an active stake in the pool. | Users may only hold one prediction per pool. |
| `InvalidPredictionAmount` | **#42** | 400 | `INVALID_INPUT` | Stake amount $\le 0$ or invalid format. | Supply positive stake amount in valid units. |
| `PredictionTooLate` | **#43** | 400 | `INVALID_INPUT` | Staking window closed (`timestamp >= end_time`). | Predictions must be placed before `end_time`. |
| `InsufficientBalanceOrStakeLimit` | **#44** | 400 | `INVALID_INPUT` | Insufficient token balance or stake bounds violated. | Verify token balance and stay within min/max bounds. |
| `InsufficientStake` | **#45** | 400 | `INVALID_INPUT` | Stake is below protocol minimum (`DEFAULT_GLOBAL_MIN_STAKE`). | Increase stake amount to meet platform minimum. |
| `AlreadyClaimed` | **#60** | 409 | `CONFLICT` | Payout or refund already withdrawn by user. | Winnings can only be claimed once per prediction. |
| `NotAWinner` | **#61** | 400 | `INVALID_INPUT` | User prediction did not match winning outcome. | No payout is available for non-winning outcomes. |
| `RewardError` | **#62** | 500 | `INTERNAL_ERROR` | Reward calculation error or zero winning stake. | Check if pool has entered zero-winner refund mode. |
| `PoolCanceled` | **#70** | 409 | `CONFLICT` | Pool was canceled prior to resolution. | Call `claim_refund` to reclaim initial stake. |
| `InvalidTimestamp` | **#80** | 400 | `INVALID_INPUT` | Timestamp in past or invalid boundary. | Provide valid future timestamp. |
| `TimeConstraintError` | **#81 / #84** | 400 | `INVALID_INPUT` | Resolution delay or claim window constraint failed. | Wait for mandatory resolution delay to elapse. |
| `StakingStillOpen` | **#82** | 400 | `INVALID_INPUT` | `close_staking` invoked before pool end time. | Await expiration of pool `end_time`. |
| `ContractPaused` | **#83** | 503 | `SERVICE_UNAVAILABLE` | Contract paused for maintenance or emergency. | Wait for administrator to unpause contract. |
| `InvalidData` | **#90** | 400 | `INVALID_INPUT` | Malformed payload or validation failed. | Fix input data according to schema rules. |
| `TokenNotWhitelisted` | **#91** | 400 | `INVALID_INPUT` | Token contract not on betting whitelist. | Admin must whitelist token via `add_token_to_whitelist`. |
| `InvalidPagination` | **#92** | 400 | `INVALID_INPUT` | Offset or limit out of allowable bounds. | Use limit between 1 and 100; non-negative offset. |
| `InvalidFeeBps` | **#93** | 400 | `INVALID_INPUT` | Fee BPS exceeds 10,000 (100%). | Supply fee BPS between 0 and 10,000. |
| `MetadataUrlInvalid` | **#94 / #109** | 400 | `INVALID_INPUT` | Metadata URL too long or invalid address. | Keep metadata URL under 512 bytes. |
| `FeeChangePending` | **#95** | 409 | `CONFLICT` | Fee proposal already pending timelock. | Cancel or apply pending proposal before creating new one. |
| `NoFeeChangePending` | **#96** | 400 | `INVALID_INPUT` | `apply_fee_bps` called with no proposal active. | Queue proposal via `set_fee_bps` first. |
| `TimelockNotExpired` | **#97** | 400 | `INVALID_INPUT` | 7-day governance timelock has not elapsed. | Await expiration of the 7-day timelock. |
| `OracleNotInitialized` | **#100** | 500 | `INTERNAL_ERROR` | Pyth oracle adapter uninitialized. | Admin must run `init_oracle`. |
| `PriceFeedNotFound` | **#101** | 404 | `NOT_FOUND` | Price feed symbol not registered in oracle. | Register or publish feed pair with oracle. |
| `PriceDataInvalid` | **#102** | 400 | `INVALID_INPUT` | Oracle price confidence ratio too low. | Publish updated price feed with tight confidence interval. |
| `PriceConditionNotSet` | **#103** | 400 | `INVALID_INPUT` | Pool lacks configured price condition. | Bind feed condition or use operator resolution. |
| `MaxTotalStakeExceeded` | **#104** | 400 | `INVALID_INPUT` | Bet would breach pool maximum total stake. | Reduce stake size to fit remaining capacity. |
| `ResolutionConflict` | **#105** | 409 | `CONFLICT` | Disagreement among oracle/operator votes. | Moderator dispute intervention required. |
| `OracleAlreadyVoted` | **#106** | 409 | `CONFLICT` | Operator already submitted resolution vote. | Only one vote permitted per operator per pool. |
| `StakeBelowMinimum` | **#107** | 400 | `INVALID_INPUT` | Stake is below pool `min_stake`. | Increase stake to satisfy pool minimum. |
| `StakeAboveMaximum` | **#108** | 400 | `INVALID_INPUT` | Stake exceeds pool `max_stake`. | Decrease stake to satisfy pool maximum. |
| `ArithmeticError` | **#110** | 500 | `INTERNAL_ERROR` | Overflow, underflow, or division by zero. | Check input magnitudes; notify core developers. |
| `FeeExceedsAmount` | **#111** | 500 | `INTERNAL_ERROR` | Fee calculation larger than total amount. | Verify protocol fee basis points configuration. |
| `MaxPredictionsExceeded` | **#111** | 400 | `INVALID_INPUT` | User hit per-pool prediction limit. | Max predictions reached for this wallet. |
| `InvalidAmount` | **#112** | 400 | `INVALID_INPUT` | Amount would cause arithmetic overflow. | Restrict amount to safe integer bounds. |
| `PriceStale` | **#113** | 400 | `INVALID_INPUT` | Oracle price older than `max_price_age`. | Refresh price feed with recent observation. |
| `StorageError` | **#120** | 500 | `INTERNAL_ERROR` | Storage key missing or archived. | Renew storage TTL via `renew_storage_ttl`. |
| `ConsistencyError` | **#121** | 500 | `INTERNAL_ERROR` | Total stake does not match sum of stakes. | Critical alert; pause contract and inspect storage. |
| `BalanceMismatch` | **#122** | 500 | `INTERNAL_ERROR` | Token balance is lower than total stakes. | Critical security alert; inspect contract vault. |
| `OutcomeDescriptionTooLong` | **#130** | 400 | `INVALID_INPUT` | Option label exceeds maximum length. | Shorten outcome description string. |
| `OutcomeDescriptionEmpty` | **#131** | 400 | `INVALID_INPUT` | Option label is empty or blank. | Provide meaningful description text for outcome. |
| `DeadlineInPast` | **#132** | 400 | `INVALID_INPUT` | Deadline is in the past. | Supply future ledger timestamp. |
| `InitialLiquidityBelowSafetyMargin` | **#133** | 400 | `INVALID_INPUT` | Insufficient initial creator liquidity. | Increase creator liquidity deposit. |
| `EmergencyCancelPending` | **#134** | 409 | `CONFLICT` | Emergency cancel requires more admin signers. | Collect remaining required multisig approvals. |
| `EmergencyCancelAlreadyApproved` | **#135** | 409 | `CONFLICT` | Admin caller already approved cancellation. | Await approval from other designated admins. |
| `TokenError` | **#150** | 400 | `INVALID_INPUT` | Token contract transfer failed. | Verify wallet token balance and approval allowance. |
| `WithdrawalOrTreasuryError` | **#151** | 500 | `INTERNAL_ERROR` | Treasury fee transfer failed. | Verify treasury address and contract token balance. |
| `OracleError` | **#160** | 503 | `SERVICE_UNAVAILABLE` | Pyth oracle unresponsive or stale data. | Update price feed or wait for oracle refresh. |
| `ResolutionError` | **#161** | 400 | `INVALID_INPUT` | Quorum failure or resolver unauthorized. | Verify resolver authority in `access-control`. |
| `AdminError` | **#180** | 500 | `INTERNAL_ERROR` | Administrative operation rejected. | Check admin role authorization and timelock. |
| `RateLimitOrSuspiciousActivity` | **#190** | 429 | `RATE_LIMIT_EXCEEDED` | Cooldown between bets active or spam detected. | Wait for `prediction_cooldown_seconds` to elapse. |
| `RequiredResolutionsExceedOperators` | **#200** | 400 | `INVALID_INPUT` | Resolution quorum exceeds active operators. | Lower quorum threshold or register more operators. |
| `InvalidTargetPrice` | **#201** | 400 | `INVALID_INPUT` | Target price is non-positive. | Supply strictly positive target price. |

---

## Alert Severity Tiers & Monitoring Guide

Contract errors are grouped into three severity tiers to drive automated alerting across PagerDuty, Slack, and Grafana.

### 🔴 HIGH Severity — Immediate Paging (24/7)

Potential security exploit, invariant breach, or critical system compromise.

- **Contract Error Codes**: `10`, `11`, `120`, `121`, `122`, `160`, `161`, `180`, `190`
- **Horizon XDR Regex Rule**:
  ```regex
  Error\(Contract, #(10|11|120|121|122|160|161|180|190)\)
  ```
- **Automated Response**: Triggers on-call engineer page via PagerDuty; if multiple `ConsistencyError` (#121) or `BalanceMismatch` (#122) appear within 60 seconds, automated circuit breaker triggers `pause()` on `predifi-contract`.

### 🟡 MEDIUM Severity — 15-Minute Alert Window

User-impacting operational issues, calculation discrepancies, or transfer failures.

- **Contract Error Codes**: `60`, `62`, `110`, `111`, `113`, `150`, `151`
- **Monitoring Rule**: Alert if error rate exceeds 10 occurrences per minute for 5 consecutive minutes.
- **Automated Response**: Dispatches notification to `#alerts-predifi-ops` Slack channel.

### 🟢 LOW Severity — Business Hours Dashboard

Standard client validation errors (e.g. pool not found, bet after deadline, insufficient balance).

- **Contract Error Codes**: `1`, `2`, `20–26`, `40–44`, `61`, `80–84`, `90–94`, `104–108`, `200`
- **Monitoring Rule**: Track aggregate volume and error distributions on Grafana dashboards. Alert only if global error rate spikes $> 100$ per minute (indicating frontend bug or coordinated spam).

---

## Client-Side Error Handling Best Practices

### 1. Decoding Soroban Simulation Failures

When calling `simulateTransaction` using the Stellar JavaScript SDK, decode contract errors before broadcasting transactions:

```typescript
import { rpc } from "@stellar/stellar-sdk";

function extractContractErrorCode(simResponse: rpc.Api.SimulateTransactionResponse): number | null {
  if (rpc.Api.isSimulationError(simResponse)) {
    const match = simResponse.error.match(/Error\(Contract, #(\d+)\)/);
    if (match) {
      return parseInt(match[1], 10);
    }
  }
  return null;
}
```

### 2. User-Friendly Error Notifications

Map raw codes to intuitive UI messages:

```typescript
const ERROR_MESSAGES: Record<number, string> = {
  43: "This market has closed. Predictions can no longer be placed.",
  44: "Insufficient token balance or amount violates the pool's stake limits.",
  60: "You have already claimed your winnings for this market.",
  61: "Your prediction was not on the winning outcome.",
  83: "The platform is currently paused for scheduled maintenance.",
  190: "Please wait a moment before placing another prediction (cooldown active)."
};

export function getErrorMessage(code: number): string {
  return ERROR_MESSAGES[code] || `Contract transaction failed (Error #${code}).`;
}
```

### 3. Backend HTTP Error Envelope Handling

```typescript
interface ApiErrorEnvelope {
  status: "error";
  error: {
    code: string;
    message: string;
    request_id: string;
  };
}

async function handleApiResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const errorBody: ApiErrorEnvelope = await response.json().catch(() => ({
      status: "error",
      error: {
        code: "HTTP_" + response.status,
        message: response.statusText,
        request_id: "unavailable"
      }
    }));
    
    console.error(`[API Error ${errorBody.error.code}] ${errorBody.error.message} (Trace: ${errorBody.error.request_id})`);
    throw new Error(errorBody.error.message);
  }
  
  return response.json();
}
```

---

**Document Maintained By**: Web3Novalabs Core Engineering  
**Source Code References**: [predifi-errors crate](file:///Users/victor/Desktop/Pvsaint/predifi/contract/contracts/predifi-errors/src/errors.rs), [native_errors.rs](file:///Users/victor/Desktop/Pvsaint/predifi/contract/contracts/predifi-errors/src/native_errors.rs), [backend errors.rs](file:///Users/victor/Desktop/Pvsaint/predifi/backend/src/errors.rs), [response.rs](file:///Users/victor/Desktop/Pvsaint/predifi/backend/src/response.rs)
