# Security Analysis: Price Feed Manipulation Resistance

**Issue:** #1919 (supersedes #1352)  
**Scope:** `contract/contracts/predifi-contract/src/` (`price_feed.rs`, `oracle.rs`)

## Overview

This document analyses the price feed system for manipulation vectors, covering
TWAP vs spot price usage, oracle trust assumptions, price staleness thresholds,
maximum price deviation guards, and flash loan attack surface.

---

## 1. TWAP vs Spot Price Usage

The system currently uses **spot price** from an external oracle (Pyth Network).
A single spot price snapshot is stored as `PriceFeed.price` and consumed
directly during resolution via `evaluate_price_condition`.

**Risk:** Spot prices can be transiently manipulated within a single ledger
close if the oracle data source itself is momentarily skewed.

**Existing mitigation:** 
- The `confidence` field (Pyth's ± uncertainty bound) is validated against `min_confidence_ratio` in basis points. Feeds whose confidence ratio exceeds the configured threshold are rejected, filtering out low-quality or abnormally wide price quotes.
- Price deviation protection (`MAX_PRICE_DEVIATION_MULTIPLIER = 5` in `oracle.rs`) rejects updates deviating by more than 5x from the previous price.

**TWAP Evaluation:**
Implementing an on-chain TWAP requires periodic accumulator updates and historical ring buffers in persistent storage, which incurs substantial storage rent on Stellar/Soroban. Furthermore, for fixed-deadline binary prediction markets ("Will BTC exceed \$X at 12:00?"), TWAPs lag behind real market outcomes, risking incorrect resolution. Spot price resolution backed by Pyth multi-source consensus, confidence bounds, and deviation multiplier guards provides optimal security without state-rent or time-lag penalties.

---

## 2. Oracle Trust Assumptions

- **Multi-source off-chain consensus:** Pyth Network aggregates data from 40+ premier independent market makers and exchanges (e.g., Jane Street, Virtu, Binance, OKX), making local DEX manipulation ineffective against global feed indices.
- **Whitelist enforcement:** Oracle feed updaters must be whitelisted via `add_oracle` / the `OracleWl` storage key. `update_price_feed` (and `batch_update_price_feeds`) calls `oracle.require_auth()`, ensuring only whitelisted oracle addresses may push price data.
- **Admin controls pyth_contract:** Only the admin can call `init_oracle`. This creates a trust dependency on the admin key, which is guarded by role-based access control.

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

Additionally, `update_price_feed` enforces temporal past-check: `timestamp < ledger.timestamp()`, rejecting future-dated or same-ledger front-running injections.

---

## 4. Flash Loan Attack Surface on Stellar/Soroban

Stellar/Soroban does not support intra-transaction uncollateralized borrow-lend composability (no flash loans in the EVM sense). Furthermore:
1. **DEX Decoupling**: PrediFi does not read on-chain AMM liquidity reserves. A massive swap on a Stellar DEX (e.g. Soroswap, Phoenix) has zero impact on Pyth Network's globally aggregated index.
2. **Deviation Multiplier Guard**: In `oracle.rs`, `MAX_PRICE_DEVIATION_MULTIPLIER` rejects single-update swings outside $[P_{\text{prev}} / 5, 5 \cdot P_{\text{prev}}]$.

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

## 6. Time & Space Complexity

- **Price Feed Ingestion (`update_price_feed`)**: $\mathcal{O}(1)$ time, $\mathcal{O}(1)$ persistent storage.
- **Price Validity & Staleness Check (`is_price_valid`)**: $\mathcal{O}(1)$ time, $\mathcal{O}(1)$ stack space.
- **Condition Evaluation (`evaluate_price_condition`)**: $\mathcal{O}(1)$ time, $\mathcal{O}(1)$ space.

---

## 7. Summary of Findings

| Vector | Status | Mitigation present |
|---|---|---|
| Spot price (no TWAP) | Mitigated | Multi-source Pyth feeds, confidence ratio filter, 5x deviation multiplier |
| Single oracle trust | Mitigated | Multi-publisher Pyth consensus + oracle whitelist + admin role gating |
| Stale price resolution | Mitigated | Dual staleness check (`expires_at` + `max_price_age`) + past timestamp invariant |
| Flash loan (Stellar) | Mitigated / N/A | Off-chain aggregator decoupling + lack of intra-tx flash borrow composability |
| Sudden extreme spike | Mitigated | `MAX_PRICE_DEVIATION_MULTIPLIER = 5` in `oracle.rs` |
| Unauthorised feed update | Mitigated | `oracle.require_auth()` + `OracleWl` whitelist |

