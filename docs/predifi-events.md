# PrediFi Contract Events Reference

## Overview

This document is the authoritative reference for **every event defined in
[`contract/contracts/predifi-contract/src/events.rs`](../contract/contracts/predifi-contract/src/events.rs)**.

It is written for **off-chain indexer integrators**: services such as Horizon
streaming consumers, Soroban RPC event pollers, analytics pipelines, and SIEM /
alerting systems. Each entry lists the stable topic discriminator, the payload
fields with their exact Soroban types, and the contract entry point that emits
it — so integrators can build a decoder without reading the Rust source.

> **Stability contract:** the event **topic** (the string in
> `#[contractevent(topics = ["..."])]`) is the stable identifier that indexers
> must filter on. Treat payload field ordering as an implementation detail; look
> fields up **by name** (the payload is a `Map<Symbol, Val>` in the raw event).

---

## Topic Structure & Encoding

Every event is published with the `#[contractevent(topics = ["<name>"])]`
macro. At runtime:

- **Topic 0** is the event identifier `Symbol` (e.g. `pool_created`). This is
  the value an indexer filters on.
- **Data (topic 1)** is the serialized event struct, decoded as a
  `Map<Symbol, Val>` whose keys are the field names documented below.

The `Symbol` topic names in this contract are all ≤ 32 characters, so each maps
to a single Soroban `Symbol` topic element.

### Subscribing to events off-chain

```text
Horizon API:
GET /events?contract={contract_id}&topic=pool_created

Soroban RPC:
{
  "jsonrpc": "2.0",
  "method": "getEvents",
  "params": {
    "filter": {
      "contractIds": ["{contract_id}"],
      "topics": [["pool_created"]]
    }
  }
}
```

### Decoding a payload (pseudo-code)

```text
for event in events:
    topic0 = symbol(event.topics[0])          # e.g. "pool_created"
    data   = map<Symbol, Val>(event.data)     # field name -> value
    pool_id = u64(data["pool_id"])
    creator = address(data["creator"])
    ...
```

---

## Event Index

| # | Event struct | Topic | Category | Emitted by |
| --- | --- | --- | --- | --- |
| 1 | `InitEvent` | `init` | Admin | `init` |
| 2 | `PauseEvent` | `pause` | Admin | `pause` |
| 3 | `UnpauseEvent` | `unpause` | Admin | `unpause` |
| 4 | `FeeUpdateEvent` | `fee_update` | Admin | `apply_fee_bps` |
| 5 | `FeeTiersUpdateEvent` | `fee_tiers_update` | Admin | `set_fee_tiers` |
| 6 | `TreasuryUpdateEvent` | `treasury_update` | Admin | `set_treasury` |
| 7 | `ResolutionDelayUpdateEvent` | `resolution_delay_update` | Admin | `set_resolution_delay` |
| 8 | `ClaimWindowUpdateEvent` | `claim_window_update` | Admin | `set_claim_window` |
| 9 | `MinPoolDurationUpdateEvent` | `min_pool_duration_update` | Admin | `set_min_pool_duration` |
| 10 | `MinStakeUpdateEvent` | `min_stake_update` | Admin | `set_min_stake` |
| 11 | `TokenWhitelistAddedEvent` | `token_whitelist_added` | Admin | `add_token_to_whitelist`, `batch_add_tokens_to_whitelist` |
| 12 | `TokenWhitelistRemovedEvent` | `token_whitelist_removed` | Admin | `remove_token_from_whitelist`, `batch_remove_tokens_whitelist` |
| 13 | `UpgradeEvent` | `upgrade` | Admin | `upgrade_contract` |
| 14 | `ContractUpgradedEvent` | `contract_upgraded` | Admin | `upgrade_contract` |
| 15 | `PoolReadyForResolutionEvent` | `pool_ready` | Lifecycle | `mark_pool_ready` |
| 16 | `PoolCreatedEvent` | `pool_created` | Lifecycle | `create_pool` |
| 17 | `InitialLiquidityProvidedEvent` | `initial_liquidity_provided` | Lifecycle | `create_pool` |
| 18 | `PoolResolvedEvent` | `pool_resolved` | Lifecycle | `resolve_pool` |
| 19 | `OracleResolvedEvent` | `oracle_resolved` | Lifecycle | oracle resolution path |
| 20 | `PoolCanceledEvent` | `pool_canceled` | Lifecycle | `cancel_pool`, `emergency_cancel_pool` |
| 21 | `StakeLimitsUpdatedEvent` | `stake_limits_updated` | Lifecycle | `set_stake_limits` |
| 22 | `OutcomeStakesUpdatedEvent` | `outcome_stakes_updated` | Lifecycle | `place_prediction` |
| 23 | `PredictionPlacedEvent` | `prediction_placed` | Financial | `place_prediction` |
| 24 | `WinningsClaimedEvent` | `winnings_claimed` | Financial | `claim_winnings`, `batch_claim_winnings` |
| 25 | `ReferralPaidEvent` | `referral_paid` | Financial | `claim_winnings` |
| 26 | `RefundClaimedEvent` | `refund_claimed` | Financial | `claim_refund` |
| 27 | `TreasuryWithdrawnEvent` | `treasury_withdrawn` | Financial | `withdraw_treasury` |
| 28 | `UnauthorizedResolveAttemptEvent` | `unauthorized_resolution` | Monitoring | resolution guards |
| 29 | `UnauthorizedAdminAttemptEvent` | `unauthorized_admin_op` | Monitoring | admin guards |
| 30 | `SuspiciousDoubleClaimEvent` | `double_claim_attempt` | Monitoring | `claim_winnings` |
| 31 | `ContractPausedAlertEvent` | `contract_paused_alert` | Monitoring | any paused entry point |
| 32 | `HighValuePredictionEvent` | `high_value_prediction` | Monitoring | `place_prediction` |
| 33 | `PoolResolvedDiagEvent` | `pool_resolved_diag` | Monitoring | `resolve_pool` |
| 34 | `OracleInitEvent` | `oracle_init` | Oracle | `init_oracle` |
| 35 | `PriceFeedUpdatedEvent` | `price_feed_updated` | Oracle | `update_price_feed` |
| 36 | `PriceConditionSetEvent` | `price_condition_set` | Oracle | `set_price_condition` |
| 37 | `PriceResolvedEvent` | `price_resolved` | Oracle | `resolve_pool_from_price` |
| 38 | `ResolutionConflictEvent` | `resolution_conflict` | Oracle | multi-oracle resolution |
| 39 | `AddedToWhitelistEvent` | `added_to_whitelist` | Whitelist | `add_to_whitelist` |
| 40 | `RemovedFromWhitelistEvent` | `removed_from_whitelist` | Whitelist | `remove_from_whitelist` |

> **Note:** entries 28–33 duplicate payloads that are also declared locally in
> `lib.rs`; the **runtime publisher is the module-local struct** (see
> [Catalog vs. runtime definitions](#catalog-vs-runtime-definitions)).

---

## 1. Administrative & Governance Events

### `InitEvent` — topic `init`

Emitted exactly once when the contract is initialized.

| Emitting entry point | Module | When |
| --- | --- | --- |
| `init` | `admin.rs` | `init` succeeds |

| Field | Type | Description |
| --- | --- | --- |
| `access_control` | `Address` | Access-control contract managing roles |
| `treasury` | `Address` | Address receiving protocol fees |
| `fee_bps` | `u32` | Default protocol fee in basis points |
| `resolution_delay` | `u64` | Mandatory delay (seconds) after `end_time` before resolution |
| `min_pool_duration` | `u64` | Minimum allowed pool duration (seconds) |

> ⚠️ The **published** `admin.rs::InitEvent` also carries
> `max_predictions_per_user: u32`, which is **absent** from the `events.rs`
> catalog struct. See [discrepancies](#catalog-vs-runtime-definitions).

### `PauseEvent` — topic `pause`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `pause` | `admin.rs` | an admin pauses the contract |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that paused the contract |

### `UnpauseEvent` — topic `unpause`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `unpause` | `admin.rs` | an admin resumes the contract |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that resumed the contract |

### `FeeUpdateEvent` — topic `fee_update`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `apply_fee_bps` | `admin.rs` | a queued fee change is applied |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that applied the change |
| `fee_bps` | `u32` | New global fee in basis points |

### `FeeTiersUpdateEvent` — topic `fee_tiers_update`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `set_fee_tiers` | `admin.rs` | fee tiers are reconfigured |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that configured the tiers |
| `tiers_count` | `u32` | Number of tiers after the update |

### `TreasuryUpdateEvent` — topic `treasury_update`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `set_treasury` | `treasury.rs` | the treasury address changes |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that changed the treasury |
| `treasury` | `Address` | New treasury address |

### `ResolutionDelayUpdateEvent` — topic `resolution_delay_update`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `set_resolution_delay` | `admin.rs` | the resolution delay changes |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that changed the delay |
| `delay` | `u64` | New resolution delay (seconds) |

### `ClaimWindowUpdateEvent` — topic `claim_window_update`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `set_claim_window` | `admin.rs` | the claim window changes |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that changed the window |
| `claim_window_seconds` | `u64` | New claim window duration (seconds) |

### `MinPoolDurationUpdateEvent` — topic `min_pool_duration_update`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `set_min_pool_duration` | `admin.rs` | the minimum pool duration changes |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that changed the minimum |
| `duration` | `u64` | New minimum pool duration (seconds) |

### `MinStakeUpdateEvent` — topic `min_stake_update`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `set_min_stake` | `admin.rs` | the global minimum stake changes |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that changed the minimum |
| `min_stake` | `i128` | New global minimum stake |

### `TokenWhitelistAddedEvent` — topic `token_whitelist_added`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `add_token_to_whitelist` / `batch_add_tokens_to_whitelist` | `admin.rs` | a token is allowlisted |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that allowlisted the token |
| `token` | `Address` | The allowlisted token contract |

### `TokenWhitelistRemovedEvent` — topic `token_whitelist_removed`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `remove_token_from_whitelist` / `batch_remove_tokens_whitelist` | `admin.rs` | a token is removed from the allowlist |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that removed the token |
| `token` | `Address` | The removed token contract |

### `UpgradeEvent` — topic `upgrade`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `upgrade_contract` | `admin.rs` | an upgrade is requested |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin requesting the upgrade |
| `new_wasm_hash` | `BytesN<32>` | Hash of the new contract WASM |

### `ContractUpgradedEvent` — topic `contract_upgraded`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `upgrade_contract` | `admin.rs` | an upgrade completes |

| Field | Type | Description |
| --- | --- | --- |
| `old_version` | `u32` | Version before the upgrade |
| `new_version` | `u32` | Version after the upgrade |
| `upgraded_by` | `Address` | Admin that performed the upgrade |

---

## 2. Market & Lifecycle Events

### `PoolReadyForResolutionEvent` — topic `pool_ready`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `mark_pool_ready` | `pool.rs` | a pool becomes resolvable |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `timestamp` | `u64` | Ledger timestamp when emitted |

### `PoolCreatedEvent` — topic `pool_created`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `create_pool` | `pool.rs` | a prediction pool is created |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | Newly assigned pool identifier |
| `creator` | `Address` | Address that created the pool |
| `end_time` | `u64` | Unix timestamp at which staking closes |
| `token` | `Address` | Staking token (must be whitelisted) |
| `options_count` | `u32` | Number of outcomes |
| `metadata_url` | `String` | Off-chain metadata URL |
| `initial_liquidity` | `i128` | Liquidity seeded by the creator |
| `category` | `Symbol` | Pool category tag |
| `required_resolutions` | `u32` | Number of operator approvals required |
| `max_total_stake` | `i128` | Maximum total stake (`0` = unlimited) |
| `min_total_stake` | `i128` | Minimum total stake for resolution *(**catalog only** — see note)* |
| `outcome_descriptions` | `Vec<String>` | Human-readable outcome labels |

> ⚠️ `min_total_stake` exists **only** in the `events.rs` catalog struct. The
> runtime `pool.rs::PoolCreatedEvent` that is actually published omits it
> (its fields are `pool_id, creator, end_time, token, options_count,
> metadata_url, initial_liquidity, category, required_resolutions,
> max_total_stake, outcome_descriptions`). Indexers must decode defensively by
> field name.

### `InitialLiquidityProvidedEvent` — topic `initial_liquidity_provided`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `create_pool` | `pool.rs` | initial liquidity is deposited (only when `initial_liquidity > 0`) |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `creator` | `Address` | Creator providing liquidity |
| `amount` | `i128` | Liquidity amount |

### `PoolResolvedEvent` — topic `pool_resolved`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `resolve_pool` | `pool.rs` | a pool is resolved |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `operator` | `Address` | Operator that resolved the pool |
| `outcome` | `u32` | Winning outcome index |

### `OracleResolvedEvent` — topic `oracle_resolved`

| Emitting entry point | Module | When |
| --- | --- | --- |
| oracle resolution path | `oracle.rs` | an oracle resolves a pool |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `oracle` | `Address` | Oracle that resolved the pool |
| `outcome` | `u32` | Resolved outcome index |
| `proof` | `String` | Supporting proof / evidence string |

### `PoolCanceledEvent` — topic `pool_canceled`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `cancel_pool`, `emergency_cancel_pool` | `pool.rs` | a pool is canceled |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `caller` | `Address` | Address that triggered the cancellation |
| `reason` | `String` | Human-readable cancellation reason |
| `operator` | `Address` | Operator associated with the action |

### `StakeLimitsUpdatedEvent` — topic `stake_limits_updated`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `set_stake_limits` | `pool.rs` | per-pool stake limits change |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `operator` | `Address` | Operator that updated the limits |
| `min_stake` | `i128` | New minimum stake |
| `max_stake` | `i128` | New maximum stake |

### `OutcomeStakesUpdatedEvent` — topic `outcome_stakes_updated`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `place_prediction` | `prediction.rs` | outcome stake totals change |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `options_count` | `u32` | Number of outcomes |
| `total_stake` | `i128` | Total stake after the update |

---

## 3. Financial & Claim Events

### `PredictionPlacedEvent` — topic `prediction_placed`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `place_prediction` | `prediction.rs` | a prediction is placed |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `user` | `Address` | Predictor address |
| `amount` | `i128` | Stake amount |
| `outcome` | `u32` | Predicted outcome index |

### `WinningsClaimedEvent` — topic `winnings_claimed`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `claim_winnings`, `batch_claim_winnings` | `prediction.rs` | winnings are claimed |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `user` | `Address` | Claimant address |
| `amount` | `i128` | Amount paid out |

### `ReferralPaidEvent` — topic `referral_paid`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `claim_winnings` | `referral.rs` | referral rewards are paid |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `referrer` | `Address` | Referrer receiving the cut |
| `referred_user` | `Address` | Winning user that triggered the cut |
| `amount` | `i128` | Referral amount paid |

### `RefundClaimedEvent` — topic `refund_claimed`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `claim_refund` | `prediction.rs` | a refund is claimed |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `user` | `Address` | Claimant address |
| `amount` | `i128` | Refund amount |

### `TreasuryWithdrawnEvent` — topic `treasury_withdrawn`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `withdraw_treasury` | `treasury.rs` | treasury funds are withdrawn |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin authorizing the withdrawal |
| `token` | `Address` | Token withdrawn |
| `amount` | `i128` | Amount withdrawn |
| `recipient` | `Address` | Recipient of the funds |
| `remaining_balance` | `i128` | Treasury balance after withdrawal |
| `timestamp` | `u64` | Ledger timestamp when emitted |

---

## 4. Monitoring & Alert Events

These events are high-signal and designed for SIEM / Horizon streaming /
Grafana alerting.

### `UnauthorizedResolveAttemptEvent` — topic `unauthorized_resolution`

| Emitting entry point | Module | When |
| --- | --- | --- |
| resolution role guards | `lib.rs` | an unauthorized resolution is attempted |

| Field | Type | Description |
| --- | --- | --- |
| `caller` | `Address` | Unauthorized caller |
| `pool_id` | `u64` | Target pool |
| `timestamp` | `u64` | Ledger timestamp when emitted |

### `UnauthorizedAdminAttemptEvent` — topic `unauthorized_admin_op`

| Emitting entry point | Module | When |
| --- | --- | --- |
| admin role guards | `lib.rs` | an unauthorized admin operation is attempted |

| Field | Type | Description |
| --- | --- | --- |
| `caller` | `Address` | Unauthorized caller |
| `operation` | `Symbol` | Attempted operation identifier |
| `timestamp` | `u64` | Ledger timestamp when emitted |

### `SuspiciousDoubleClaimEvent` — topic `double_claim_attempt`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `claim_winnings` | `prediction.rs` | a duplicate claim is attempted |

| Field | Type | Description |
| --- | --- | --- |
| `user` | `Address` | User attempting the duplicate claim |
| `pool_id` | `u64` | The pool identifier |
| `timestamp` | `u64` | Ledger timestamp when emitted |

### `ContractPausedAlertEvent` — topic `contract_paused_alert`

| Emitting entry point | Module | When |
| --- | --- | --- |
| any paused entry point | `admin.rs` | a paused contract operation is attempted |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin associated with the paused state |
| `timestamp` | `u64` | Ledger timestamp when emitted |

### `HighValuePredictionEvent` — topic `high_value_prediction`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `place_prediction` | `prediction.rs` | a high-value prediction is placed |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `user` | `Address` | Predictor address |
| `amount` | `i128` | Stake amount |
| `outcome` | `u32` | Predicted outcome index |
| `threshold` | `i128` | Threshold that was exceeded |

### `PoolResolvedDiagEvent` — topic `pool_resolved_diag`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `resolve_pool` | `pool.rs` | resolution diagnostics are recorded |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `outcome` | `u32` | Resolved outcome index |
| `total_stake` | `i128` | Total stake in the pool |
| `winning_stake` | `i128` | Stake on the winning outcome |
| `timestamp` | `u64` | Ledger timestamp when emitted |

---

## 5. Oracle & Price-Feed Events

### `OracleInitEvent` — topic `oracle_init`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `init_oracle` | `oracle.rs` | an oracle is configured |

| Field | Type | Description |
| --- | --- | --- |
| `admin` | `Address` | Admin that configured the oracle |
| `pyth_contract` | `Address` | Price-feed (Pyth) contract address |
| `max_price_age` | `u64` | Maximum acceptable price age (seconds) |
| `min_confidence_ratio` | `u32` | Minimum confidence ratio in basis points |

### `PriceFeedUpdatedEvent` — topic `price_feed_updated`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `update_price_feed` | `oracle.rs` | a price feed changes |

| Field | Type | Description |
| --- | --- | --- |
| `oracle` | `Address` | Oracle publishing the price |
| `feed_pair` | `Symbol` | Feed pair identifier |
| `price` | `i128` | Published price |
| `confidence` | `i128` | Confidence interval |
| `timestamp` | `u64` | Publish timestamp |
| `expires_at` | `u64` | Expiry timestamp |

### `PriceConditionSetEvent` — topic `price_condition_set`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `set_price_condition` | `oracle.rs` | a price condition is configured |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `feed_pair` | `Symbol` | Feed pair identifier |
| `target_price` | `i128` | Target price |
| `operator` | `u32` | Comparison operator (encoded) |
| `tolerance_bps` | `u32` | Tolerance in basis points |

### `PriceResolvedEvent` — topic `price_resolved`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `resolve_pool_from_price` | `oracle.rs` | a price condition resolves |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `feed_pair` | `Symbol` | Feed pair identifier |
| `current_price` | `i128` | Price observed at resolution |
| `target_price` | `i128` | Configured target price |
| `outcome` | `u32` | Resolved outcome index |

### `ResolutionConflictEvent` — topic `resolution_conflict`

| Emitting entry point | Module | When |
| --- | --- | --- |
| multi-oracle resolution | `oracle.rs` | resolution sources disagree |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The pool identifier |
| `oracle` | `Address` | Oracle reporting the conflicting outcome |
| `outcome` | `u32` | Newly reported outcome |
| `existing_outcome` | `u32` | Previously recorded outcome |

---

## 6. Whitelist Events

### `AddedToWhitelistEvent` — topic `added_to_whitelist`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `add_to_whitelist` | `admin.rs` | a participant is allowlisted for a private pool |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The private pool identifier |
| `user` | `Address` | Address added to the whitelist |
| `added_by` | `Address` | Pool creator that performed the addition |
| `timestamp` | `u64` | Ledger timestamp when emitted |

### `RemovedFromWhitelistEvent` — topic `removed_from_whitelist`

| Emitting entry point | Module | When |
| --- | --- | --- |
| `remove_from_whitelist` | `admin.rs` | a participant is removed from a private pool's whitelist |

| Field | Type | Description |
| --- | --- | --- |
| `pool_id` | `u64` | The private pool identifier |
| `user` | `Address` | Address removed from the whitelist |
| `removed_by` | `Address` | Pool creator that performed the removal |
| `timestamp` | `u64` | Ledger timestamp when emitted |

---

## Catalog vs. Runtime Definitions

The `events.rs` module is the **catalog / source of truth for documentation**,
but the contract mirrors event structs in the module that actually publishes
them (`admin.rs`, `pool.rs`, `prediction.rs`, `referral.rs`, `treasury.rs`,
`oracle.rs`, `lib.rs`). **The module-local struct is what is serialized on
chain.**

Known divergences between the `events.rs` catalog and the runtime publisher:

| Event | Topic | Divergence |
| --- | --- | --- |
| `InitEvent` | `init` | Runtime adds `max_predictions_per_user: u32`, absent from `events.rs`. |
| `PoolCreatedEvent` | `pool_created` | `events.rs` adds `min_total_stake: i128`; the runtime struct omits it. |

> Because the payload is a name-keyed `Map<Symbol, Val>`, indexers that look up
> fields **by name** and tolerate missing/extra keys will be robust to these
> divergences.

### Additional runtime topics not present in `events.rs`

These topics are emitted by the contract but are declared **only** in their
functional module (not in `events.rs`). Indexers should handle them too:

| Topic | Declared in | Emitted by |
| --- | --- | --- |
| `staking_closed` | `pool.rs` | `close_staking` |
| `pool_disputed` | `pool.rs` | `flag_disputed_pool` |
| `pool_description_updated` | `pool.rs` | `update_pool_description` |
| `max_stake_increased` | `pool.rs` | `increase_max_total_stake` |
| `resolution_vote_cast` | `oracle.rs` | operator resolution voting |
| `price_feeds_cleaned` | `oracle.rs` | `cleanup_expired_feeds` |
| `oracle_whitelist_added` | `oracle.rs` | `add_oracle` |
| `oracle_whitelist_removed` | `oracle.rs` | `remove_oracle` |
| `prediction_blocked_delisted` | `prediction.rs` | `place_prediction` (delisted token) |
| `reward_claimed` | `referral.rs` | reward claim path |
| `referrer_updated` | `referral.rs` | `update_referrer` |
| `referral_threshold_updated` | `referral.rs` | `set_referral_volume_threshold` |
| `fee_change_proposed` | `admin.rs` | `set_fee_bps` |
| `fee_change_canceled` | `admin.rs` | `cancel_fee_proposal` |
| `max_predictions_update` | `admin.rs` | `set_max_predictions_per_user` |
| `prediction_cooldown_update` | `admin.rs` | `set_prediction_cooldown` |
| `storage_ttl_renewed` | `admin.rs` | `renew_storage_ttl` |
| `emergency_withdraw` | `treasury.rs` | `emergency_withdraw` |
| `creator_incentive_config_updated` | `creator_incentive.rs` | `set_creator_incentive_bps` |
| `creator_incentive_paid` | `creator_incentive.rs` | `claim_creator_incentive` |

---

## Security Assumptions

- Events are emitted **only after** a successful state change; a failing
  operation reverts the whole transaction, so no event is observable.
- Monitoring events (`unauthorized_resolution`, `unauthorized_admin_op`,
  `double_claim_attempt`, `contract_paused_alert`) are emitted on the **failure
  path** and are intended for alerting.
- `timestamp` fields use `env.ledger().timestamp()` (on-chain ledger time) and
  cannot be manipulated by the caller.

---

## Test Coverage

Event topic and payload assertions live under
`contract/contracts/predifi-contract/tests/`:

| Suite | Covers |
| --- | --- |
| `pause_unpause_tests.rs` | `contract_paused_alert`, `pause`, `unpause` |
| `pool_lifecycle_tests.rs` | `pool_created`, `staking_closed`, `pool_ready`, `pool_resolved` |
| `update_referrer_tests.rs` | `referrer_updated` payload |
| `event_assertion_tests.rs` | exact topics + payload shapes for state-changing entry points |

---

## Related Files

- `contract/contracts/predifi-contract/src/events.rs` — event catalog (this document's source of truth)
- `contract/contracts/predifi-contract/src/lib.rs` — shared event structs + role guards
- `contract/contracts/predifi-contract/src/admin.rs` — admin event structs
- `contract/contracts/predifi-contract/src/pool.rs` — pool lifecycle event structs
- `contract/contracts/predifi-contract/src/prediction.rs` — prediction/claim event structs
- `contract/contracts/predifi-contract/src/referral.rs` — referral event structs
- `contract/contracts/predifi-contract/src/treasury.rs` — treasury event structs
- `contract/contracts/predifi-contract/src/oracle.rs` — oracle event structs
- `docs/whitelist-events.md` — companion whitelist event guide
