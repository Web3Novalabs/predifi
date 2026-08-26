# Troubleshooting

Common errors, solutions, and debugging tips for PrediFi integration.

## Error Code Reference

PrediFi uses comprehensive error codes for precise error handling. All errors implement the `PredifiError` enum.

### Error Categories

| Range | Category | Description |
|-------|----------|-------------|
| 1-5 | Initialization | Contract setup errors |
| 10-15 | Authorization | Access control errors |
| 20-30 | Pool State | Pool lifecycle errors |
| 40-50 | Prediction | Betting errors |
| 60-70 | Claiming | Reward claim errors |
| 80-85 | Timestamp | Time validation errors |
| 90-100 | Validation | Input validation errors |
| 110-118 | Arithmetic | Math operation errors |
| 120-129 | Storage | Data persistence errors |
| 150-159 | Token | Token transfer errors |
| 160-169 | Oracle | Oracle resolution errors |
| 180-189 | Admin | Admin operation errors |

---

## Common Errors

### Initialization Errors

#### `NotInitialized` (Code: 1)

**Message:** "Contract has not been initialized yet"

**Cause:** Contract `init()` function hasn't been called.

**Solution:**

```rust
// Call init before using contract
contract.init(
    env,
    access_control_address,
    treasury_address,
    100 // 1% fee (100 basis points)
);
```

---

### Authorization Errors

#### `Unauthorized` (Code: 10)

**Message:** "The caller is not authorized to perform this action"

**Cause:** User lacks required role for the operation.

**Solution:**

- For `resolve_pool()`: Ensure caller has Operator role (1)
- For admin functions: Ensure caller has Admin role (0)
- Check access control contract for role assignments

**Example:**

```rust
// Check if user has operator role
let has_role = access_control.has_role(user, 1);
if !has_role {
    return Err(PredifiError::Unauthorized);
}
```

---

### Pool State Errors

#### `PoolNotFound` (Code: 20)

**Message:** "The specified pool was not found"

**Cause:** Pool ID doesn't exist.

**Solution:**

```typescript
// Verify pool exists before operations
const pool = await contract.call('get_pool', {
  pool_id: nativeToScVal(poolId, { type: 'u64' })
});

if (!pool) {
  throw new Error('Pool not found');
}
```

#### `PoolAlreadyResolved` (Code: 21)

**Message:** "The pool has already been resolved"

**Cause:** Attempting to resolve or modify an already resolved pool.

**Solution:** Check pool state before operations:

```rust
let pool = get_pool(&env, pool_id)?;
if pool.resolved {
    return Err(PredifiError::PoolAlreadyResolved);
}
```

#### `PoolNotResolved` (Code: 22)

**Message:** "The pool has not been resolved yet"

**Cause:** Attempting to claim winnings from unresolved pool.

**Solution:** Wait for pool resolution:

```typescript
// Check if pool is resolved
const pool = await getPool(poolId);
if (!pool.resolved) {
  console.log('Pool not yet resolved. Waiting...');
  return;
}

// Now safe to claim
await claimWinnings(poolId);
```

---

### Prediction Errors

#### `InvalidPredictionAmount` (Code: 42)

**Message:** "The prediction amount is invalid (e.g., zero or negative)"

**Cause:** Amount is zero or negative.

**Solution:**

```rust
// Validate amount before calling
if amount <= 0 {
    return Err(PredifiError::InvalidPredictionAmount);
}
```

#### `PredictionTooLate` (Code: 43)

**Message:** "Cannot place prediction after pool end time"

**Cause:** Pool's `end_time` has passed.

**Solution:**

```typescript
// Check pool end time
const pool = await getPool(poolId);
const now = Date.now() / 1000; // Unix timestamp

if (now >= pool.end_time) {
  throw new Error('Pool has closed for predictions');
}
```

#### `InsufficientBalanceOrStakeLimit` (Code: 44)

**Message:** "The user has insufficient balance or stake limit violation"

**Cause:** User doesn't have enough tokens or exceeds stake limit.

**Solution:**

```typescript
// Check balance before prediction
const balance = await tokenContract.balance(userAddress);
if (balance < amount) {
  throw new Error('Insufficient balance');
}

// Check stake limits if applicable
const totalStake = await getUserTotalStake(userAddress);
if (totalStake + amount > MAX_STAKE) {
  throw new Error('Stake limit exceeded');
}
```

---

### Claiming Errors

#### `AlreadyClaimed` (Code: 60)

**Message:** "The user has already claimed winnings for this pool"

**Cause:** User already claimed winnings from this pool.

**Solution:**

```typescript
// Check if already claimed before calling
const hasClaimed = await checkIfClaimed(userAddress, poolId);
if (hasClaimed) {
  console.log('Already claimed');
  return;
}

// Safe to claim
await claimWinnings(poolId);
```

#### `NotAWinner` (Code: 61)

**Message:** "The user did not win this pool"

**Cause:** User's prediction outcome doesn't match pool outcome.

**Note:** This doesn't throw an error - `claim_winnings()` returns 0 for losers.

**Solution:**

```rust
let winnings = contract.claim_winnings(env, user, pool_id)?;
if winnings == 0 {
    // User didn't win or already claimed
}
```

---

### Timestamp Errors

#### `InvalidTimestamp` (Code: 80)

**Message:** "The provided timestamp is invalid or time constraints not met"

**Cause:** Timestamp validation failed (e.g., end_time in the past).

**Solution:**

```rust
// Validate timestamp
let current_time = env.ledger().timestamp();
if end_time <= current_time {
    return Err(PredifiError::InvalidTimestamp);
}
```

---

### Validation Errors

#### `InvalidData` (Code: 90)

**Message:** "The provided data is invalid"

**Cause:** General data validation failure.

**Solution:** Check all input parameters match expected types and constraints.

#### `InvalidAddressOrToken` (Code: 91)

**Message:** "The provided address or token is invalid"

**Cause:** Invalid Stellar address or token contract.

**Solution:**

```typescript
// Validate address format
function isValidAddress(address: string): boolean {
  // Stellar addresses are 56 characters, start with G
  return /^G[A-Z0-9]{55}$/.test(address);
}

if (!isValidAddress(tokenAddress)) {
  throw new Error('Invalid token address');
}
```

---

### Arithmetic Errors

#### `ArithmeticError` (Code: 110)

**Message:** "An arithmetic overflow, underflow, or division by zero occurred"

**Cause:** Math operation failed (overflow, underflow, or division by zero).

**Solution:** Use checked arithmetic:

```rust
// Use checked operations
let total = stake_a
    .checked_add(stake_b)
    .ok_or(PredifiError::ArithmeticError)?;

let winnings = amount
    .checked_mul(pool.total_stake)
    .ok_or(PredifiError::ArithmeticError)?
    .checked_div(winning_stake)
    .ok_or(PredifiError::ArithmeticError)?;
```

---

### Token Errors

#### `TokenError` (Code: 150)

**Message:** "Token transfer, approval, or contract call failed"

**Cause:** Token contract call failed (insufficient balance, approval, etc.).

**Solution:**

```typescript
// Check balance and approval before transfer
const balance = await token.balance(userAddress);
if (balance < amount) {
  throw new Error('Insufficient balance');
}

// Ensure contract has approval (if needed)
await token.approve(userAddress, contractAddress, amount);
```

---

### Oracle Errors

#### `OracleError` (Code: 160)

**Message:** "Oracle error or stale data detected"

**Cause:** Oracle data is unavailable or stale.

**Solution:**

```typescript
// Verify oracle data freshness
const oracleData = await queryOracle(poolId);
const dataAge = Date.now() - oracleData.timestamp;

if (dataAge > MAX_DATA_AGE) {
  throw new Error('Oracle data is stale');
}
```

#### `ResolutionError` (Code: 161)

**Message:** "Resolution error or unauthorized resolver"

**Cause:** Resolution attempt failed or unauthorized.

**Solution:** Ensure operator has role 1 and pool is ready for resolution.

---

## RPC & Network Issues

### Transaction Timeout

**Symptom:** Transaction hangs or times out.

**Solutions:**

1. **Increase timeout:**
```typescript
const tx = new TransactionBuilder(account, {
  timeout: 60 // Increase from default 30
})
```

2. **Check network status:**
```typescript
const server = new Server('https://horizon-testnet.stellar.org');
const health = await server.health();
console.log('Network status:', health);
```

3. **Retry with exponential backoff:**
```typescript
async function retryWithBackoff(fn, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      await sleep(2 ** i * 1000); // Exponential backoff
    }
  }
}
```

### Connection Errors

**Symptom:** Cannot connect to Stellar network.

**Solutions:**

- Verify network endpoint is correct
- Check firewall/proxy settings
- Try alternative Horizon server
- Verify internet connection

### Gas/Fee Estimation

**Symptom:** Transaction fails with insufficient fee.

**Solution:**

```typescript
// Get recommended fee
const feeStats = await server.feeStats();
const recommendedFee = feeStats.fee_charged.mode;

const tx = new TransactionBuilder(account, {
  fee: recommendedFee.toString()
});
```

---

## Debugging Tips

### 1. Enable Verbose Logging

```typescript
// Enable detailed logging
const server = new Server('https://horizon-testnet.stellar.org', {
  allowHttp: true
});

server.on('request', (req) => {
  console.log('Request:', req);
});

server.on('response', (res) => {
  console.log('Response:', res);
});
```

### 2. Check Contract State

```typescript
// Verify contract is initialized
const config = await contract.call('get_config');
console.log('Config:', config);

// Check if paused
const paused = await contract.call('is_paused');
console.log('Paused:', paused);
```

### 3. Validate Pool State

```typescript
// Get full pool state
const pool = await getPool(poolId);
console.log('Pool state:', {
  id: poolId,
  endTime: new Date(pool.end_time * 1000),
  resolved: pool.resolved,
  outcome: pool.outcome,
  totalStake: pool.total_stake
});
```

### 4. Monitor Events

```typescript
// Listen for contract events
const events = await server.effects()
  .forAccount(contractAddress)
  .order('desc')
  .limit(10)
  .call();

events.records.forEach(event => {
  console.log('Event:', event);
});
```

---

## Getting Help

If you encounter issues not covered here:

1. **Check Error Codes:** Review the [Error Code Reference](#error-code-reference) above
2. **Review Documentation:** See [Contract Reference](./contract-reference.md)
3. **Community Support:** Join [Telegram](https://t.me/predifi_onchain_build/1)
4. **Open an Issue:** [GitHub Issues](https://github.com/Web3Novalabs/predifi/issues)

---

## Next Steps

- Review [Quickstart](./quickstart.md) for basic usage
- Explore [Contract Reference](./contract-reference.md) for API details
- Understand [Prediction Lifecycle](./prediction-lifecycle.md) for flow
