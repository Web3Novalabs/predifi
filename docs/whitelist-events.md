# Whitelist Events

## Overview

This document describes the whitelist event emission system implemented
in the `predifi-contract`. When a user is added to or removed from a
private pool's whitelist, the contract emits a structured event so that
off-chain systems can track whitelist changes for auditing and analytics.

---

## Events

### `AddedToWhitelistEvent`
Emitted when a user is successfully added to a private pool's whitelist.

| Field | Type | Description |
|-------|------|-------------|
| pool_id | u64 | The ID of the private pool |
| user | Address | The Stellar address added to the whitelist |
| timestamp | u64 | Ledger timestamp when the event occurred |

### `RemovedFromWhitelistEvent`
Emitted when a user is successfully removed from a private pool's whitelist.

| Field | Type | Description |
|-------|------|-------------|
| pool_id | u64 | The ID of the private pool |
| user | Address | The Stellar address removed from the whitelist |
| timestamp | u64 | Ledger timestamp when the event occurred |

---

## Implementation

**File:** `contract/contracts/predifi-contract/src/lib.rs`

### `add_to_whitelist(env, creator, pool_id, user)`
Adds a user to a private pool's whitelist. Only callable by the pool
creator. Emits `AddedToWhitelistEvent` on success.

### `remove_from_whitelist(env, creator, pool_id, user)`
Removes a user from a private pool's whitelist. Only callable by the
pool creator. Emits `RemovedFromWhitelistEvent` on success.

---

## Security Assumptions

- Only the pool creator can add or remove users from the whitelist
- `creator.require_auth()` is called before any state changes
- Events are emitted only after successful state changes — no event
  is emitted if the operation fails
- The `timestamp` field uses `env.ledger().timestamp()` which is
  the on-chain ledger time — it cannot be manipulated by the caller
- Pool must be private (`pool.private == true`) for whitelist operations
  to be valid

---

## Abuse and Failure Paths

| Scenario | Behaviour |
|----------|-----------|
| Non-creator tries to add to whitelist | Panics with Unauthorized |
| Non-creator tries to remove from whitelist | Panics with Unauthorized |
| Pool is not private | Panics with assertion error |
| Pool does not exist | Panics with "Pool not found" |
| Contract is paused | Panics — require_not_paused check |

---

## Test Coverage

**File:** `contract/contracts/predifi-contract/src/test.rs`

| Test | What it verifies |
|------|-----------------|
| `test_whitelist_events_emitted` | Events emitted on add and remove |
| `test_unauthorized_add_to_whitelist_panics` | Non-creator cannot add |
| `test_unauthorized_remove_from_whitelist_panics` | Non-creator cannot remove |

---

## Example
```rust
// Add a user to a private pool's whitelist
client.add_to_whitelist(&creator, &pool_id, &user);
// AddedToWhitelistEvent { pool_id, user, timestamp } is emitted

// Remove a user from a private pool's whitelist
client.remove_from_whitelist(&creator, &pool_id, &user);
// RemovedFromWhitelistEvent { pool_id, user, timestamp } is emitted
```

---

## Related Files

- `contract/contracts/predifi-contract/src/lib.rs` — event definitions
  and function implementations
- `contract/contracts/predifi-contract/src/test.rs` — test suite