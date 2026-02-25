# PrediFi Protocol — Monitoring & Observability Reference

> **Audience:** DevOps / SRE teams operating the PrediFi protocol indexer,
> Grafana dashboards, SIEM rules, or PagerDuty integrations.

---

## 1. On-Chain Event Catalogue

All on-chain events are emitted via Soroban's `contractevent` mechanism and
streamed through the Stellar Horizon API.  The **topic** field is the primary
key for filtering.

### 1.1 Alert Severity Index

| Severity | Topic | Description | Action |
|----------|-------|-------------|--------|
| 🔴 HIGH | `unauthorized_resolution` | Address without Operator role attempted `resolve_pool` | Page immediately |
| 🔴 HIGH | `unauthorized_admin_op` | Address without Admin role attempted a privileged operation | Page immediately |
| 🔴 HIGH | `double_claim_attempt` | `claim_winnings` called twice for the same (user, pool) | Investigate within 5 min |
| 🔴 HIGH | `contract_paused_alert` | Contract was successfully paused by an admin | Page immediately |
| 🟡 MEDIUM | `high_value_prediction` | Stake ≥ 1,000,000 base units placed on a single prediction | Alert within 15 min |
| 🟢 INFO | `pool_resolved_diag` | Enriched resolution stats (stakes, timestamp) | Log; alert if `winning_stake == 0` |
| 🟢 INFO | `pool_resolved` | Standard resolution notification | Log |
| 🟢 INFO | `pool_created` | New prediction pool created | Log |
| 🟢 INFO | `prediction_placed` | User placed a prediction | Log |
| 🟢 INFO | `winnings_claimed` | User claimed their winnings | Log |
| 🟢 INFO | `pause` | Contract paused (standard event) | Log |
| 🟢 INFO | `unpause` | Contract unpaused | Log |
| 🟢 INFO | `fee_update` | Fee basis points changed | Log |
| 🟢 INFO | `treasury_update` | Treasury address changed | Log |
| 🟢 INFO | `init` | Contract initialised | Log |

---

## 2. Horizon API Query Snippets

Replace `<CONTRACT_ID>` with the deployed Soroban contract address.

### Stream all events for the contract
```bash
curl "https://horizon-testnet.stellar.org/contracts/<CONTRACT_ID>/events?limit=200&order=asc"
```

### Filter by a specific topic (e.g., HIGH-alert events only)
```bash
# Fetch unauthorized resolution attempts
curl "https://horizon-testnet.stellar.org/contracts/<CONTRACT_ID>/events?\
topic1=AAAADwAAABp1bmF1dGhvcml6ZWRfcmVzb2x1dGlvbgAA"
# Note: topic values are base64-encoded XDR Symbols.
# Use stellar-xdr or soroban CLI to encode: soroban xdr encode --type ScSymbol "unauthorized_resolution"
```

### Using the Soroban CLI event watcher
```bash
soroban events \
  --contract-id <CONTRACT_ID> \
  --network testnet \
  --start-ledger <START_LEDGER> \
  --filter-topics unauthorized_resolution,unauthorized_admin_op,double_claim_attempt,contract_paused_alert
```

---

## 3. Structured JSON Log Schema

Off-chain indexers should normalise every on-chain event into the following
JSON structure before writing to Elasticsearch / Loki / CloudWatch:

```json
{
  "schema_version": "1.0",
  "contract_id": "<CONTRACT_ID>",
  "ledger":       12345678,
  "ledger_ts":    1740224496,
  "tx_hash":      "abc123...",
  "topic":        "unauthorized_resolution",
  "severity":     "HIGH",
  "payload": {
    "caller":     "GABCD....",
    "pool_id":    42,
    "timestamp":  1740224490
  }
}
```

**Field notes:**
- `severity` — derive from the [Severity Index](#11-alert-severity-index) table.
- `ledger_ts` — Stellar ledger close time (Unix seconds).
- `payload` — mirrors the event struct field names from `lib.rs` verbatim.

---

## 4. Alert Rule Definitions

### 4.1 PagerDuty / Opsgenie Rules

| Rule Name | Trigger Condition | Priority |
|-----------|------------------|----------|
| `predifi-unauthorized-resolution` | topic = `unauthorized_resolution` | P1 |
| `predifi-unauthorized-admin` | topic = `unauthorized_admin_op` | P1 |
| `predifi-double-claim` | topic = `double_claim_attempt` | P1 |
| `predifi-contract-paused` | topic = `contract_paused_alert` | P1 |
| `predifi-high-value-prediction` | topic = `high_value_prediction` AND `amount >= 1000000` | P2 |
| `predifi-no-winners` | topic = `pool_resolved_diag` AND `winning_stake == 0` | P2 |

### 4.2 Example Grafana Loki Alert (LogQL)

```logql
# Fire when any HIGH-severity event is indexed within last 5 minutes
count_over_time(
  {job="predifi-indexer"} |= `"severity":"HIGH"` [5m]
) > 0
```

### 4.3 Example Prometheus Rule (via log exporter)

```yaml
groups:
  - name: predifi_contract
    rules:
      - alert: PredifiUnauthorizedAccess
        expr: increase(predifi_events_total{severity="HIGH"}[5m]) > 0
        for: 0m
        labels:
          severity: critical
        annotations:
          summary: "Unauthorized access attempt on PrediFi contract"
          description: "Topic: {{ $labels.topic }}, caller: {{ $labels.caller }}"

      - alert: PredifiContractPaused
        expr: increase(predifi_events_total{topic="contract_paused_alert"}[5m]) > 0
        for: 0m
        labels:
          severity: critical
        annotations:
          summary: "PrediFi contract has been paused"

      - alert: PredifiHighValuePrediction
        expr: increase(predifi_events_total{topic="high_value_prediction"}[15m]) > 0
        for: 0m
        labels:
          severity: warning
        annotations:
          summary: "High-value prediction detected on PrediFi"
```

---

## 5. Critical Panic Strings

The following `panic!` messages are emitted by the contract on certain severe
conditions.  Soroban node logs include the panic string in the transaction
diagnostic event.  Configure your log scraper to alert when these exact strings
appear.

| Panic String | Context | Severity |
|---|---|---|
| `"Unauthorized: missing required role"` | Non-admin called `pause` or `unpause` | 🔴 HIGH |
| `"Contract is paused"` | State-mutating call while paused — should not reach production | 🟡 MEDIUM |
| `"Pool not found"` | `resolve_pool` / `claim_winnings` called with invalid pool ID | 🟡 MEDIUM |
| `"Pool already resolved"` | Duplicate resolution attempt in `resolve_pool` | 🟡 MEDIUM |
| `"overflow"` | `checked_add` or `checked_mul` overflowed — arithmetic invariant violated | 🔴 HIGH |
| `"division by zero"` | `winning_stake` was zero during payout calc | 🔴 HIGH |
| `"index not found"` | User prediction index corrupt | 🔴 HIGH |
| `"Config not set"` | Contract called before `init` | 🔴 HIGH |

### Regex to catch all HIGH panics in node logs
```
(overflow|division by zero|index not found|Config not set|Unauthorized: missing required role)
```

---

## 6. Error Code Routing

Soroban returns contract errors as `Error(Contract, #<code>)` in transaction
results.  The table below maps codes to alert tiers.

### 6.1 HIGH — Immediate page

| Code | Variant | Regex match |
|------|---------|-------------|
| 10 | `Unauthorized` | `Error\(Contract, #10\)` |
| 11 | `InsufficientPermissions` | `Error\(Contract, #11\)` |
| 120 | `StorageError` | `Error\(Contract, #120\)` |
| 121 | `ConsistencyError` | `Error\(Contract, #121\)` |
| 122 | `BalanceMismatch` | `Error\(Contract, #122\)` |
| 160 | `OracleError` | `Error\(Contract, #160\)` |
| 161 | `ResolutionError` | `Error\(Contract, #161\)` |
| 180 | `AdminError` | `Error\(Contract, #180\)` |
| 190 | `RateLimitOrSuspiciousActivity` | `Error\(Contract, #190\)` |

**Combined HIGH regex:**
```
Error\(Contract, #(10|11|120|121|122|160|161|180|190)\)
```

### 6.2 MEDIUM — Alert within 15 minutes

| Code | Variant |
|------|---------|
| 60 | `AlreadyClaimed` |
| 62 | `RewardError` |
| 110 | `ArithmeticError` |
| 111 | `FeeExceedsAmount` |
| 150 | `TokenError` |
| 151 | `WithdrawalOrTreasuryError` |

**Combined MEDIUM regex:**
```
Error\(Contract, #(60|62|110|111|150|151)\)
```

### 6.3 LOW — Review during business hours

All remaining codes: 1, 2, 20, 21, 22, 23, 24, 25, 26, 40, 41, 42, 43, 44,
61, 80, 81, 90, 91, 92, 93, 94.

---

## 7. Diagnostic Checklist for Incident Response

When a HIGH-alert event fires:

1. **Identify the transaction** — use the `tx_hash` from the structured log.
2. **Inspect the caller** — cross-reference `caller` field with known operator/admin addresses.
3. **Check access-control state** — call `has_role(caller, role)` on the access-control contract via Horizon RPC.
4. **Assess blast radius** — for `unauthorized_resolution`, check if any pool was actually resolved (look for `pool_resolved` event in the same ledger).
5. **Pause if necessary** — admin should call `pause` immediately if an active exploit is suspected.
6. **Post-mortem** — review `pool_resolved_diag` events for anomalous `winning_stake == 0` patterns that could indicate oracle manipulation.

---

*Last updated: 2026-02-22 — generated alongside contract v1 monitoring implementation.*
