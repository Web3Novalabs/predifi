# Contract Reference

Complete reference for all PrediFi contract methods, events, and data structures.

---

## Access-Control Contract

The access-control contract provides role-based access control (RBAC) for the
PrediFi protocol. It is deployed separately and its address is passed to
`predifi-contract::init` as the `access_control` argument.

### Roles

| Role | Value | Used by |
|---|---|---|
| Admin | 0 | `pause`, `unpause`, `set_fee_bps`, `set_treasury`, `set_resolution_delay`, `set_referral_cut_bps`, token whitelist, `withdraw_treasury`, `upgrade_contract`, `migrate_state` |
| Operator | 1 | `resolve_pool`, `cancel_pool`, `set_stake_limits` |
| Moderator | 2 | Reserved for future dispute-resolution (#595) |
| Oracle | 3 | `oracle_resolve` (OracleCallback trait) |
| User | 4 | Basic protocol participants |

### Bootstrap sequence

1. Deploy the access-control contract (no constructor arguments).
2. Call `init(admin_address)` — stores the admin and auto-grants the Admin role (0).
3. Call `assign_role` to grant Operator / Oracle roles as needed.
4. Pass the contract address to `predifi_contract::init` as `access_control`.

---

### `init`

```rust
pub fn init(env: &Env, admin: Address)
```

Initialises the contract and sets the first administrator. Can only be called once; a second call panics with `AlreadyInitializedOrConfigNotSet`.

---

### `assign_role`

```rust
pub fn assign_role(env: &Env, admin_caller: Address, user: Address, role: Role)
    -> Result<(), PrediFiError>
```

Grants `role` to `user`. Only the current admin may call this.

---

### `revoke_role`

```rust
pub fn revoke_role(env: &Env, admin_caller: Address, user: Address, role: Role)
    -> Result<(), PrediFiError>
```

Removes `role` from `user`. Returns `AdminError` if called with `Role::Admin` against the current (sole) admin — transfer admin first.

---

### `transfer_role`

```rust
pub fn transfer_role(env: &Env, admin_caller: Address, from: Address, to: Address, role: Role)
    -> Result<(), PrediFiError>
```

Atomically moves `role` from `from` to `to`.

---

### `revoke_all_roles`

```rust
pub fn revoke_all_roles(env: &Env, admin_caller: Address, user: Address)
    -> Result<(), PrediFiError>
```

Removes every role (0–4) from `user`. Returns `AdminError` if `user` is the current admin.

---

### `has_role`

```rust
pub fn has_role(env: &Env, user: Address, role: Role) -> bool
```

Returns `true` if `user` holds `role`. Also extends the role entry's TTL on a positive match.

---

### `has_any_role`

```rust
pub fn has_any_role(env: &Env, user: &Address, roles: &Vec<Role>) -> bool
```

Returns `true` if `user` holds at least one of the roles in `roles`.

---

### `is_admin`

```rust
pub fn is_admin(env: &Env, user: &Address) -> bool
```

Returns `true` if `user` is the current admin.

---

### `get_admin`

```rust
pub fn get_admin(env: &Env) -> Address
```

Returns the current admin address. Panics with `"NotInitialized"` if `init` has not been called.

---

### `get_proposed_admin`

```rust
pub fn get_proposed_admin(env: &Env) -> Option<Address>
```

Returns the pending proposed admin, or `None` if no transfer is in progress.

---

### `get_operator_count`

```rust
pub fn get_operator_count(env: &Env) -> u32
```

Returns how many addresses currently hold the Operator role. Used by `predifi-contract` to validate `required_resolutions`.

---

### Admin transfer (two-step)

> **`transfer_admin` has been removed.** The only supported path is the
> two-step flow below, which requires the new admin's signature and therefore
> prevents transferring admin rights to a mistyped or uncontrolled address.

**Step 1 — current admin proposes a successor:**

```rust
pub fn propose_new_admin(env: &Env, current_admin: Address, new_admin: Address)
    -> Result<(), PrediFiError>
```

**Step 2 — proposed admin accepts:**

```rust
pub fn accept_admin_role(env: &Env, new_admin: Address) -> Result<(), PrediFiError>
```

On acceptance the old admin loses the Admin role, the new admin gains it, and the pending proposal is cleared.

---

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

| Parameter        | Type      | Description                            |
| ---------------- | --------- | -------------------------------------- |
| `access_control` | `Address` | Access control contract address        |
| `treasury`       | `Address` | Treasury address for fee collection    |
| `fee_bps`        | `u32`     | Fee in basis points (max 10000 = 100%) |

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

| Parameter      | Type      | Description                                  |
| -------------- | --------- | -------------------------------------------- |
| `end_time`     | `u64`     | Unix timestamp after which predictions close |
| `token`        | `Address` | Stellar token contract for staking           |
| `description`  | `String`  | Event description (max 256 bytes)            |
| `metadata_url` | `String`  | Extended metadata URL (max 512 bytes)        |

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

Place a prediction on an active pool. Optionally specify a referrer to earn a share of the protocol fee when the referred user claims winnings.

```rust
pub fn place_prediction(
    env: Env,
    user: Address,
    pool_id: u64,
    amount: i128,
    outcome: u32,
    referrer: Option<Address>
)
```

**Parameters:**

| Parameter  | Type              | Description                                                                                                                 |
| ---------- | ----------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `user`     | `Address`         | User placing the prediction                                                                                                 |
| `pool_id`  | `u64`             | Pool to predict on                                                                                                          |
| `amount`   | `i128`            | Prediction amount (in token's smallest unit)                                                                                |
| `outcome`  | `u32`             | Outcome index (0, 1, 2, etc.)                                                                                               |
| `referrer` | `Option<Address>` | Optional address that referred this user; stored on first prediction for (user, pool) and used for referral payout on claim |

**Returns:** None

**Events:** `PredictionPlacedEvent`

**Validations:**

- Pool must exist
- Pool must not be resolved
- Current time < pool.end_time
- Amount > 0
- User must have sufficient token balance
- If `referrer` is set: cannot be the user or the contract address

**Token Transfer:**

- Transfers `amount` tokens from user to contract

**Example:**

```rust
contract.place_prediction(
    env,
    user_address,
    pool_id,
    1000000000, // 100 tokens
    1, // Outcome: "Yes"
    Some(referrer_address) // or None
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

| Parameter  | Type      | Description                         |
| ---------- | --------- | ----------------------------------- |
| `operator` | `Address` | Operator address (must have role 1) |
| `pool_id`  | `u64`     | Pool to resolve                     |
| `outcome`  | `u32`     | Winning outcome index               |

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

Claim winnings from a resolved pool. Protocol fee is deducted from the pool before distribution; if the user was referred, a portion of that fee is paid to the referrer.

```rust
pub fn claim_winnings(
    env: Env,
    user: Address,
    pool_id: u64
) -> Result<i128, PredifiError>
```

**Parameters:**

| Parameter | Type      | Description            |
| --------- | --------- | ---------------------- |
| `user`    | `Address` | User claiming winnings |
| `pool_id` | `u64`     | Pool to claim from     |

**Returns:** `Result<i128, PredifiError>` - Amount claimed (0 if didn't win)

**Events:** `WinningsClaimedEvent`, `ReferralPaidEvent` (when user had a referrer and wins)

**Validations:**

- Pool must be resolved
- User must not have already claimed
- User must have placed a prediction

**Reward Calculation:**

- Protocol fee = `total_pool_stake × fee_bps / 10_000`
- Payout pool = `total_pool_stake - protocol_fee`
- `winnings = (user_stake / winning_outcome_total_stake) × payout_pool`
- If user has a referrer: referrer receives `referral_cut_bps` of the user's share of the protocol fee (the rest remains for treasury withdrawal).

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

| Parameter | Type      | Description               |
| --------- | --------- | ------------------------- |
| `user`    | `Address` | User address              |
| `offset`  | `u32`     | Pagination offset         |
| `limit`   | `u32`     | Maximum results to return |

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

| Parameter | Type      | Description                      |
| --------- | --------- | -------------------------------- |
| `admin`   | `Address` | Admin address (must have role 0) |

**Events:** `PauseEvent`

---

### `unpause`

Resume contract operations. Requires Admin role (0).

```rust
pub fn unpause(env: Env, admin: Address)
```

**Parameters:**

| Parameter | Type      | Description                      |
| --------- | --------- | -------------------------------- |
| `admin`   | `Address` | Admin address (must have role 0) |

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

| Parameter | Type      | Description                         |
| --------- | --------- | ----------------------------------- |
| `admin`   | `Address` | Admin address                       |
| `fee_bps` | `u32`     | New fee in basis points (max 10000) |

**Events:** `FeeUpdateEvent`

---

### `set_referral_cut_bps`

Set the referral cut in basis points (e.g. 5000 = 50% of the referrer's fee share). Requires Admin role (0). Must be ≤ 10_000.

```rust
pub fn set_referral_cut_bps(
    env: Env,
    admin: Address,
    referral_cut_bps: u32
) -> Result<(), PredifiError>
```

**Parameters:**

| Parameter          | Type      | Description                                                                        |
| ------------------ | --------- | ---------------------------------------------------------------------------------- |
| `admin`            | `Address` | Admin address                                                                      |
| `referral_cut_bps` | `u32`     | Share of protocol fee (attributable to referred user) paid to referrer (max 10000) |

**Referral view functions:**

- **`get_referral_cut_bps(env: Env) -> u32`** — Returns the current referral cut in basis points (default 5000).
- **`get_referred_volume(env: Env, referrer: Address, pool_id: u64) -> i128`** — Returns the total stake amount referred by `referrer` on the given pool (for analytics and payout context).

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

| Parameter  | Type      | Description          |
| ---------- | --------- | -------------------- |
| `admin`    | `Address` | Admin address        |
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

Emitted when a pool is resolved and the outcome is determined. This occurs when:

- An operator calls `resolve_pool()` with a valid outcome after the resolution delay
- An oracle calls `oracle_resolve()` and the voting threshold is reached
- `resolve_pool_from_price()` is called and the price condition is evaluated

The `operator` field indicates the address (operator or oracle) that triggered the resolution, or the contract itself if resolved automatically via price feed.

```rust
pub struct PoolResolvedEvent {
    pub pool_id: u64,     // Unique identifier for the resolved pool
    pub operator: Address, // Address that triggered the resolution
    pub outcome: u32,      // The determined winning outcome (0-based index)
}
```

**Example**: When outcome 1 is selected for pool 12345 by operator addr123:

```rust
PoolResolvedEvent {
    pool_id: 12345,
    operator: addr123,
    outcome: 1,
}
```

### `PoolResolvedDiagEvent`

Emitted alongside `PoolResolvedEvent` with enriched numeric context for monitoring and payout calculations. This diagnostic event allows off-chain systems to:

- Calculate implied payouts
- Flag anomalies (e.g., `winning_stake == 0` indicates no winners)
- Track liquidity and stake distribution at resolution time

This event is **always** published immediately after `PoolResolvedEvent`.

```rust
pub struct PoolResolvedDiagEvent {
    pub pool_id: u64,           // Same pool as PoolResolvedEvent
    pub outcome: u32,            // Same outcome as PoolResolvedEvent
    pub total_stake: i128,       // Total stake across all outcomes
    pub winning_stake: i128,     // Stake on the winning outcome (0 ⟹ no winners)
    pub timestamp: u64,          // Ledger timestamp at resolution
}
```

**Example**: Diagnostic data for the same resolution:

```rust
PoolResolvedDiagEvent {
    pool_id: 12345,
    outcome: 1,
    total_stake: 5000,          // 5000 total tokens staked
    winning_stake: 2000,        // 2000 on outcome 1
    timestamp: 1234567890,
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

### `RewardClaimedEvent`

Emitted when a user claims a payout from the contract. This event is published for both winning claims and refund claims.

```rust
pub struct RewardClaimedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
    pub claim_type: String,
}
```

---

## Error Codes

See [Troubleshooting](./troubleshooting.md) for complete error reference.

| Code | Error             | Description                   |
| ---- | ----------------- | ----------------------------- |
| 10   | `Unauthorized`    | Caller lacks required role    |
| 22   | `PoolNotResolved` | Pool not yet resolved         |
| 60   | `AlreadyClaimed`  | User already claimed winnings |

---

## Next Steps

- Start with [Quickstart](./quickstart.md)
- Understand [Prediction Lifecycle](./prediction-lifecycle.md)
- Learn about [Oracles](./oracles.md)
- Review [Troubleshooting](./troubleshooting.md)
