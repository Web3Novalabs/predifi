# PrediFi Errors Crate (`predifi-errors`)

`predifi-errors` is the centralized error handling crate for PrediFi smart contracts on Soroban. It provides standardized error enums, granular error codes, categorization, and helper methods shared between smart contracts, backend services, and frontend applications.

For a full reference of error codes and resolution strategies across the protocol, see [`docs/ERROR_HANDLING_REFERENCE.md`](../../../docs/ERROR_HANDLING_REFERENCE.md).

---

## Key Design Principles

### 1. Gap-Based Numbering Scheme
Error codes are assigned numeric integers grouped into distinct ranges with reserved numerical gaps (e.g., Initialization: 1–5, Authorization: 10–15, Pool State: 20–30, Prediction: 40–50). 
- **Extensibility**: Reserved numerical gaps allow developers to add new, highly specific error variants to existing categories without shifting or renumbering existing error codes.
- **Client Mapping Stability**: Because existing numbers remain unchanged when new variants are added, frontend applications and backend indexers relying on numeric error maps will not break.

### 2. 32-Variant Soroban Contract Error Limit
Soroban imposes a hard maximum limit of **32 variants** per single `#[contracterror]` enum compiled into a smart contract interface:
- **Architecture**: `PrediFiError` separates contract-level error codes and internal sub-errors logically.
- **Wasm Optimization**: High-frequency contract error codes fit within Soroban's native error payload limits, while non-contract helper utilities provide extended categorization (`as_str()`, `category()`, `is_recoverable()`) without bloating compiled Wasm size.

---

## Error Categories & Numbering Ranges

The numbering scheme in `errors.rs` is organized as follows:

| Range | Category | Description | Example Error Code |
|-------|----------|-------------|---------------------|
| **1 – 5** | Initialization | Contract setup and admin initialization | `NotInitialized` (1) |
| **10 – 15** | Authorization | Admin access control and signature checks | `Unauthorized` (10) |
| **20 – 30** | Pool State | Pool lifecycle (creation, pause, resolution) | `PoolNotFound` (20) |
| **40 – 50** | Prediction | Placing predictions and outcome validation | `PoolClosed` (41) |
| **60 – 70** | Claiming | Claim processing and reward distribution | `AlreadyClaimed` (60) |
| **80 – 85** | Timestamp | Expiration, deadline, and ledger time checks | `StartTimeInPast` (80) |
| **90 – 100** | Validation | Parameter bounds and input sanity checks | `InvalidOutcomeCount` (90) |
| **110 – 118** | Arithmetic | Math overflow, underflow, and div-by-zero | `ArithmeticOverflow` (110) |
| **120 – 129** | Storage | State retrieval and storage bump errors | `StorageCorrupted` (120) |
| **130 – 145** | Granular Validation | Specific field validation (amounts, fees) | `AmountIsZero` (130) |
| **150 – 159** | Token | Soroban SAC token transfers and balances | `TokenTransferFailed` (150) |
| **160 – 169** | Oracle | Oracle registration, price feeds, and feed age | `OracleNotConfigured` (160) |
| **170 – 179** | Reward | Reward calculation and pool payout bounds | `InvalidRewardShare` (170) |
| **180 – 189** | Admin | Emergency pause, whitelist, and config management | `EmergencyPauseActive` (180) |
| **190 – 199** | Rate Limiting | Anti-spam and request frequency limits | `RateLimitExceeded` (190) |

---

## Guidelines: How to Add a New Error Code

When adding a new error variant to `predifi-errors`:

1. **Identify the Category**: Determine which functional category the error belongs to (e.g., Oracle, Claiming, Validation).
2. **Find the Next Gap**: Check `errors.rs` in that category range and assign the next unused integer in that range. **Never alter existing integer values.**
3. **Add Doc Comments**: Include a descriptive `///` doc comment explaining when the error is raised.
4. **Implement Helper Methods**:
   - Update `as_str(&self)` with a human-readable message.
   - Update `category(&self)` with the appropriate category string.
   - Update `is_recoverable(&self)` to indicate whether a user can fix the input and retry.
5. **Verify Tests**: Run `cargo test -p predifi-errors` to confirm unit tests pass.

---

## Usage Example

```rust
use predifi_errors::PrediFiError;

// Throw error in contract
pub fn validate_stake(amount: i128) -> Result<(), PrediFiError> {
    if amount <= 0 {
        return Err(PrediFiError::AmountIsZero); // Code 130
    }
    Ok(())
}

// Extract metadata in backend / tests
let err = PrediFiError::AmountIsZero;
assert_eq!(err.code(), 130);
assert_eq!(err.category(), "granular_validation");
assert!(err.is_recoverable());
```

---

## Reference Document

For comprehensive protocol error mapping and troubleshooting procedures, see [`docs/ERROR_HANDLING_REFERENCE.md`](../../../docs/ERROR_HANDLING_REFERENCE.md).
