# PrediFi Errors

A comprehensive error handling crate for PrediFi smart contracts on Soroban.

## Overview

This crate provides a well-structured error system with granular error codes, categorization, and frontend-friendly utilities. The error codes use gap-based numbering to allow future additions without breaking existing client-side error mappings.

## Features

- **95+ Specific Error Variants**: Covering all aspects of the prediction market protocol
- **Gap-Based Numbering**: Error codes organized in ranges (e.g., 1-5, 10-15) for future extensibility
- **Error Categorization**: Logical grouping of errors for analytics and debugging
- **Recoverability Detection**: Distinguish between user-fixable and system errors
- **Display Trait**: Human-readable error messages for logging
- **Frontend Integration**: Easy error code extraction for UI error handling

## Error Categories

| Range | Category | Description |
|-------|----------|-------------|
| 1-5 | Initialization | Contract setup and configuration |
| 10-15 | Authorization | Access control and permissions |
| 20-30 | Pool State | Pool lifecycle management |
| 40-50 | Prediction | Betting and prediction placement |
| 60-70 | Claiming | Reward claiming |
| 80-85 | Timestamp | Time validation |
| 90-100 | Validation | General data validation |
| 110-118 | Arithmetic | Mathematical operations |
| 120-129 | Storage | Data persistence |
| 130-145 | Granular Validation | Specific input validation |
| 150-159 | Token | Token transfers and interactions |
| 160-169 | Oracle | Oracle and resolution |
| 170-179 | Reward | Reward calculations |
| 180-189 | Admin | Emergency and admin operations |
| 190-199 | Rate Limiting | Spam prevention |

## Usage

### Basic Error Handling

```rust
use predifi_errors::PrediFiError;

fn validate_pool_state(pool: &Pool) -> Result<(), PrediFiError> {
    if pool.is_resolved {
        return Err(PrediFiError::PoolAlreadyResolved);
    }
    
    if pool.end_time < env.ledger().timestamp() {
        return Err(PrediFiError::PoolExpired);
    }
    
    Ok(())
}
```

### Error Metadata

```rust
let error = PrediFiError::InvalidPredictionAmount;

// Get numeric error code (for frontend)
let code = error.code(); // 42

// Get error category (for analytics)
let category = error.category(); // "prediction"

// Get human-readable message (for logging)
let message = error.as_str(); // "Invalid prediction amount"

// Check if user can recover (for UI feedback)
let recoverable = error.is_recoverable(); // true
```

### Frontend Integration

```typescript
// Example TypeScript error handling
interface ErrorResponse {
  code: number;
  category: string;
  message: string;
  recoverable: boolean;
}

function handleContractError(errorCode: number): string {
  const errorMap: Record<number, string> = {
    42: "Please enter a valid prediction amount",
    43: "The pool has closed for predictions",
    44: "Insufficient balance to place this prediction",
    // ... more mappings
  };
  
  return errorMap[errorCode] || "An unexpected error occurred";
}
```

## Error Handling Best Practices

### 1. Use Specific Errors

```rust
// ❌ Too generic
return Err(PrediFiError::InvalidData);

// ✅ Specific and actionable
return Err(PrediFiError::InvalidPredictionAmount);
```

### 2. Validate Early

```rust
pub fn place_prediction(
    env: Env,
    user: Address,
    pool_id: u64,
    outcome: u32,
    amount: i128,
) -> Result<(), PrediFiError> {
    // Validate inputs first
    if amount <= 0 {
        return Err(PrediFiError::AmountIsZero);
    }
    
    // Then check state
    let pool = get_pool(&env, pool_id)?;
    if pool.is_resolved {
        return Err(PrediFiError::PoolAlreadyResolved);
    }
    
    // Finally perform operation
    // ...
}
```

### 3. Handle Arithmetic Safely

```rust
// Use checked arithmetic and return specific errors
let total = stake_a
    .checked_add(stake_b)
    .ok_or(PrediFiError::AdditionOverflow)?;

let fee = amount
    .checked_mul(fee_bps as i128)
    .ok_or(PrediFiError::MultiplicationOverflow)?
    .checked_div(10000)
    .ok_or(PrediFiError::DivisionByZero)?;
```

### 4. Maintain State Consistency

```rust
// Check for state inconsistencies
if pool.total_stake != pool.outcome_stakes.iter().sum() {
    return Err(PrediFiError::StakeInconsistency);
}
```

## Adding New Errors

When adding new error variants:

1. Choose an appropriate range based on the error category
2. Use the next available number in that range
3. Add a descriptive doc comment
4. Update the `as_str()` method with a clear message
5. Update the `category()` method if needed
6. Consider if the error is recoverable and update `is_recoverable()` if needed

Example:

```rust
pub enum PrediFiError {
    // ... existing errors ...
    
    // -- New Category (200-209) ----------------------------------------
    /// Description of the new error.
    NewErrorVariant = 200,
}

impl PrediFiError {
    pub fn as_str(&self) -> &'static str {
        match self {
            // ... existing matches ...
            Self::NewErrorVariant => "Clear error message",
        }
    }
    
    pub const fn category(&self) -> &'static str {
        match self {
            // ... existing matches ...
            Self::NewErrorVariant => "new_category",
        }
    }
}
```

## Testing

```rust
#[test]
fn test_error_codes() {
    assert_eq!(PrediFiError::NotInitialized.code(), 1);
    assert_eq!(PrediFiError::Unauthorized.code(), 10);
    assert_eq!(PrediFiError::PoolNotFound.code(), 20);
}

#[test]
fn test_error_categories() {
    assert_eq!(PrediFiError::NotInitialized.category(), "initialization");
    assert_eq!(PrediFiError::Unauthorized.category(), "authorization");
    assert_eq!(PrediFiError::ArithmeticOverflow.category(), "arithmetic");
}

#[test]
fn test_error_recoverability() {
    assert!(!PrediFiError::StorageCorrupted.is_recoverable());
    assert!(PrediFiError::InvalidPredictionAmount.is_recoverable());
}
```

## License

This crate is part of the PrediFi project.
