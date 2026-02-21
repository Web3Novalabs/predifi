# Contract Reference

Complete reference for all PrediFi contract methods, events, and data structures.

## Core Functions

### `init`

Initialize the contract with configuration parameters.

```rust
pub fn init(
    env: Env,
    access_control: Address,
    treasury: Address,
    fee_bps: u32
)
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `access_control` | `Address` | Access control contract address |
| `treasury` | `Address` | Treasury address for fee collection |
| `fee_bps` | `u32` | Fee in basis points (max 10000 = 100%) |

**Returns:** None

**Events:** `InitEvent`

**Notes:**
- Idempotent - safe to call multiple times
- Only sets config if not already initialized

---

### `create_pool`

Create a new prediction market pool.

```rust
pub fn create_pool(
    env: Env,
    end_time: u64,
    token: Address,
    description: String,
    metadata_url: String
) -> u64
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `end_time` | `u64` | Unix timestamp after which predictions close |
| `token` | `Address` | Stellar token contract for staking |
| `description` | `String` | Event description (max 256 bytes) |
| `metadata_url` | `String` | Extended metadata URL (max 512 bytes) |

**Returns:** `u64` - New pool ID

**Events:** `PoolCreatedEvent`

**Validations:**
- `end_time` must be in the future
- `description` length ≤ 256 bytes
- `metadata_url` length ≤ 512 bytes

**Example:**

```rust
let pool_id = contract.create_pool(
    env,
    1735689600, // Dec 31, 2024
    token_address,
    String::from_str(&env, "Will BTC hit $100k?"),
    String::from_str(&env, "ipfs://QmXxx...")
);
```

---

### `place_prediction`

Place a prediction on an active pool.

```rust
pub fn place_prediction(
    env: Env,
    user: Address,
    pool_id: u64,
    amount: i128,
    outcome: u32
)
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `user` | `Address` | User placing the prediction |
| `pool_id` | `u64` | Pool to predict on |
| `amount` | `i128` | Prediction amount (in token's smallest unit) |
| `outcome` | `u32` | Outcome index (0, 1, 2, etc.) |

**Returns:** None

**Events:** `PredictionPlacedEvent`

**Validations:**
- Pool must exist
- Pool must not be resolved
- Current time < pool.end_time
- Amount > 0
- User must have sufficient token balance

**Token Transfer:**
- Transfers `amount` tokens from user to contract

**Example:**

```rust
contract.place_prediction(
    env,
    user_address,
    pool_id,
    1000000000, // 100 tokens
    1 // Outcome: "Yes"
);
```

---

### `resolve_pool`

Resolve a pool with the winning outcome. Requires Operator role (1).

```rust
pub fn resolve_pool(
    env: Env,
    operator: Address,
    pool_id: u64,
    outcome: u32
) -> Result<(), PredifiError>
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `operator` | `Address` | Operator address (must have role 1) |
| `pool_id` | `u64` | Pool to resolve |
| `outcome` | `u32` | Winning outcome index |

**Returns:** `Result<(), PredifiError>`

**Events:** `PoolResolvedEvent`

**Validations:**
- Operator must have role 1
- Pool must exist
- Pool must not already be resolved

**Errors:**
- `Unauthorized` - Operator lacks required role
- `PoolNotFound` - Pool doesn't exist
- `PoolAlreadyResolved` - Pool already resolved

**Example:**

```rust
contract.resolve_pool(
    env,
    operator_address,
    pool_id,
    1 // Winning outcome
)?;
```

---

### `claim_winnings`

Claim winnings from a resolved pool.

```rust
pub fn claim_winnings(
    env: Env,
    user: Address,
    pool_id: u64
) -> Result<i128, PredifiError>
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `user` | `Address` | User claiming winnings |
| `pool_id` | `u64` | Pool to claim from |

**Returns:** `Result<i128, PredifiError>` - Amount claimed (0 if didn't win)

**Events:** `WinningsClaimedEvent`

**Validations:**
- Pool must be resolved
- User must not have already claimed
- User must have placed a prediction

**Reward Calculation:**

```
winnings = (user_stake / winning_outcome_total_stake) × total_pool_stake
```

**Errors:**
- `PoolNotResolved` - Pool not yet resolved
- `AlreadyClaimed` - User already claimed winnings

**Example:**

```rust
let winnings = contract.claim_winnings(
    env,
    user_address,
    pool_id
)?;

if winnings > 0 {
    // User won and received winnings
}
```

---

### `get_user_predictions`

Get a paginated list of a user's predictions.

```rust
pub fn get_user_predictions(
    env: Env,
    user: Address,
    offset: u32,
    limit: u32
) -> Vec<UserPredictionDetail>
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `user` | `Address` | User address |
| `offset` | `u32` | Pagination offset |
| `limit` | `u32` | Maximum results to return |

**Returns:** `Vec<UserPredictionDetail>`

**Example:**

```rust
let predictions = contract.get_user_predictions(
    env,
    user_address,
    0,  // offset
    10  // limit
);
```

---

## Admin Functions

### `pause`

Pause all contract operations. Requires Admin role (0).

```rust
pub fn pause(env: Env, admin: Address)
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `admin` | `Address` | Admin address (must have role 0) |

**Events:** `PauseEvent`

---

### `unpause`

Resume contract operations. Requires Admin role (0).

```rust
pub fn unpause(env: Env, admin: Address)
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `admin` | `Address` | Admin address (must have role 0) |

**Events:** `UnpauseEvent`

---

### `set_fee_bps`

Update protocol fee. Requires Admin role (0).

```rust
pub fn set_fee_bps(
    env: Env,
    admin: Address,
    fee_bps: u32
) -> Result<(), PredifiError>
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `admin` | `Address` | Admin address |
| `fee_bps` | `u32` | New fee in basis points (max 10000) |

**Events:** `FeeUpdateEvent`

---

### `set_treasury`

Update treasury address. Requires Admin role (0).

```rust
pub fn set_treasury(
    env: Env,
    admin: Address,
    treasury: Address
) -> Result<(), PredifiError>
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `admin` | `Address` | Admin address |
| `treasury` | `Address` | New treasury address |

**Events:** `TreasuryUpdateEvent`

---

## Data Structures

### `Pool`

```rust
pub struct Pool {
    pub end_time: u64,
    pub resolved: bool,
    pub outcome: u32,
    pub token: Address,
    pub total_stake: i128,
    pub description: String,
    pub metadata_url: String,
}
```

### `Prediction`

```rust
pub struct Prediction {
    pub amount: i128,
    pub outcome: u32,
}
```

### `UserPredictionDetail`

```rust
pub struct UserPredictionDetail {
    pub pool_id: u64,
    pub amount: i128,
    pub user_outcome: u32,
    pub pool_end_time: u64,
    pub pool_resolved: bool,
    pub pool_outcome: u32,
}
```

### `Config`

```rust
pub struct Config {
    pub fee_bps: u32,
    pub treasury: Address,
    pub access_control: Address,
}
```

---

## Events

### `PoolCreatedEvent`

Emitted when a new pool is created.

```rust
pub struct PoolCreatedEvent {
    pub pool_id: u64,
    pub end_time: u64,
    pub token: Address,
    pub metadata_url: String,
}
```

### `PredictionPlacedEvent`

Emitted when a user places a prediction.

```rust
pub struct PredictionPlacedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
    pub outcome: u32,
}
```

### `PoolResolvedEvent`

Emitted when a pool is resolved.

```rust
pub struct PoolResolvedEvent {
    pub pool_id: u64,
    pub operator: Address,
    pub outcome: u32,
}
```

### `WinningsClaimedEvent`

Emitted when a user claims winnings.

```rust
pub struct WinningsClaimedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
}
```

---

## Error Codes

See [Troubleshooting](./troubleshooting.md) for complete error reference.

| Code | Error | Description |
|------|-------|-------------|
| 10 | `Unauthorized` | Caller lacks required role |
| 22 | `PoolNotResolved` | Pool not yet resolved |
| 60 | `AlreadyClaimed` | User already claimed winnings |

---

## Next Steps

- Start with [Quickstart](./quickstart.md)
- Understand [Prediction Lifecycle](./prediction-lifecycle.md)
- Learn about [Oracles](./oracles.md)
- Review [Troubleshooting](./troubleshooting.md)
