# Security Analysis: Price Feed Manipulation Resistance

**Issue:** #1352  
**Scope:** `contract/contracts/predifi-contract/src/price_feed.rs`

## Overview

This document analyses the price feed system for manipulation vectors, covering
TWAP vs spot price usage, oracle trust assumptions, price staleness thresholds,
and flash loan attack surface.

---

## 1. TWAP vs Spot Price Usage

The system currently uses **spot price** from an external oracle (Pyth Network).
A single spot price snapshot is stored as `PriceFeed.price` and consumed
directly during resolution via `evaluate_price_condition`.

**Risk:** Spot prices can be transiently manipulated within a single ledger
close if the oracle data source itself is momentarily skewed.

**Existing mitigation:** The `confidence` field (Pyth's ± uncertainty bound)
is validated against `min_confidence_ratio` in basis points. Feeds whose
confidence ratio exceeds the configured threshold are rejected, filtering out
low-quality or abnormally wide price quotes.

**Recommendation (future):** Consider storing a rolling window of price
snapshots and computing an on-chain TWAP before resolution. This would require
a separate accumulator storage key and a minimum observation window enforced at
resolution time.

---

## 2. Oracle Trust Assumptions

- **Single oracle source:** The system trusts one `pyth_contract` address
  configured at `init_oracle`. If this address is compromised or a malicious
  contract is substituted, all price resolution is affected.
- **Whitelist enforcement:** Oracle feed updaters must be whitelisted via
  `add_oracle` / the `OracleWl` storage key. `update_price_feed` (and
  `batch_update_price_feeds`) calls `oracle.require_auth()`, ensuring only
  whitelisted oracle addresses may push price data.
- **Admin controls pyth_contract:** Only the admin can call `init_oracle`. This
  creates a trust dependency on the admin key, which is mitigated by the
  role-based access control system.

---

## 3. Price Staleness Thresholds

Staleness is double-checked in `is_price_valid`:

```rust
// Check 1 — hard expiry set by the oracle itself
if current_time > feed.expires_at { return false; }

// Check 2 — global max_price_age set by the admin
if current_time > feed.timestamp + config.max_price_age { return false; }
```

Both checks must pass. The tighter of the two governs. `max_price_age` is
configurable by the admin and should be set conservatively (e.g., ≤ 60 s for
volatile assets). Pools that attempt to resolve against stale data receive
`PredifiError::ResolutionDelayNotMet`, preventing a stale-price resolution.

---

## 4. Flash Loan Attack Surface on Stellar/Soroban

Stellar/Soroban does not support intra-transaction composability (no flash
loans in the Ethereum sense): every transaction is atomic but cross-contract
state changes across separate invocations are committed individually. The
absence of same-ledger borrow-repay atomicity eliminates the classical flash
loan vector.

However, a well-funded actor could submit a large DEX trade in ledger N to move
a spot price, then submit `resolve_pool_from_price` in the same ledger close if
the Pyth feed is updated at the same timestamp. The `expires_at` / `max_price_age`
staleness guard mitigates this because resolution windows should be long (hours
to days), making a single-ledger spike insufficient to guarantee resolution
in the attacker's favour.

**Residual risk:** Pools with very short durations (near `min_pool_duration`)
and high `max_price_age` values are more susceptible to a sustained price push
rather than a flash spike. Keeping `max_price_age` short and enforcing a
minimum pool duration (already present via `set_min_pool_duration`) reduces this
window.

---

## 5. Confidence Interval Validation

```rust
let confidence_ratio = (feed.confidence * 10000) / feed.price;
if confidence_ratio > config.min_confidence_ratio as i128 { return false; }
```

This guards against unusually wide uncertainty bands (a signal of low-quality
or manipulated feeds). The threshold is admin-configurable and should be set
to a value consistent with the asset's normal volatility (e.g., 100 bps for
stable assets, up to 500 bps for volatile assets).

---

## 6. Summary of Findings

| Vector | Status | Mitigation present |
|---|---|---|
| Spot price (no TWAP) | Open risk | Confidence ratio filter; short `max_price_age` |
| Single oracle trust | Accepted | Oracle whitelist + admin role-gated `init_oracle` |
| Stale price resolution | Mitigated | Dual staleness check (`expires_at` + `max_price_age`) |
| Flash loan (Stellar) | Not applicable | No intra-tx composability on Soroban |
| Sustained price push | Low risk | Short pools + short `max_price_age` recommended |
| Unauthorised feed update | Mitigated | `oracle.require_auth()` + oracle whitelist |
