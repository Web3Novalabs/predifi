# Security Analysis: Front-Running Protection for Predictions

**Issue:** #1353  
**Scope:** `contract/contracts/predifi-contract/src/lib.rs` — `place_prediction`

## Overview

This document analyses `place_prediction` for front-running vulnerabilities,
evaluates whether prediction outcomes can be influenced by transaction ordering,
assesses commit-reveal scheme feasibility, and documents MEV implications on
Stellar/Soroban.

---

## 1. Front-Running Analysis of `place_prediction`

`place_prediction` accepts `(user, pool_id, amount, outcome, referrer,
invite_key)` as plaintext parameters. An observer who can see pending
transactions before ledger close could:

1. **Outcome copying** — See that a well-informed user is predicting outcome X,
   then submit the same prediction with a higher fee to be ordered first.
2. **Outcome flipping on parimutuel pools** — On parimutuel-style pools the
   payout ratio shifts as more stake accumulates on each side. A front-runner
   who predicts after seeing a large incoming stake on the opposite side gains
   a slightly improved odds position.

**Mitigations already in place:**
- `prediction_cooldown_seconds` prevents rapid sequential predictions from the
  same address, limiting automated bots from reacting to every observed
  submission.
- `max_total_stake` and `max_stake` caps bound the maximum advantage a
  front-runner can extract.

---

## 2. Transaction Ordering on Stellar/Soroban

Stellar does not expose a public mempool like Ethereum. Transactions are
submitted directly to individual validator nodes and are included in the next
ledger close (~5 s). Key properties:

- **No public mempool:** Pending transactions are not globally broadcast before
  inclusion, making it significantly harder for an adversary to observe and
  front-run a specific transaction.
- **Consensus ordering:** The Stellar Consensus Protocol (SCP) determines
  ledger ordering; no single miner can reorder transactions for personal gain.
- **Deterministic fees:** Stellar uses a minimum base fee with optional fee
  bumps. A fee-bump transaction can wrap another transaction but cannot
  selectively reorder arbitrary third-party transactions in the same ledger.

**Conclusion:** Classical MEV (miner-extractable value) as seen on Ethereum
is largely absent on Stellar. A node operator could theoretically observe
submitted transactions before ledger close, but the SCP quorum requirement
makes unilateral transaction censorship or reordering impractical.

---

## 3. Commit-Reveal Scheme Feasibility

A commit-reveal scheme would allow a predictor to:

1. **Commit phase** — Submit `hash(outcome || nonce || user)` in transaction 1.
2. **Reveal phase** — Submit `(outcome, nonce)` in transaction 2 within a
   time window; the contract verifies the hash and records the prediction.

**Feasibility on Soroban:**
- Soroban supports `sha256` and `keccak256` through `soroban_sdk::crypto`.
- Persistent storage can hold commitment hashes keyed by `(user, pool_id)`.
- A two-ledger minimum between commit and reveal is enforceable via
  `env.ledger().sequence()`.

**Trade-offs:**
- Two-transaction UX increases friction for end users.
- A commit that is never revealed ties up the user slot (mitigated by a
  commit expiry window after which the slot is freed).
- Given Stellar's non-public mempool, the practical risk of front-running is
  low, making the UX cost of commit-reveal hard to justify currently.

**Recommendation:** Implement commit-reveal as an opt-in feature for
high-value pools (configurable per-pool flag) rather than a protocol-wide
requirement.

---

## 4. MEV Implications on Stellar/Soroban

| MEV vector | Ethereum | Stellar/Soroban |
|---|---|---|
| Mempool sniping | High risk | Very low — no public mempool |
| Transaction reordering | High risk | Not applicable — SCP consensus |
| Sandwich attacks | High risk | Very low — no atomic swap composability |
| Oracle front-running | Medium risk | Low — oracle updates are separate txns |
| Fee-bump priority | N/A | Possible within single-account scope only |

The primary residual MEV surface on Stellar is **oracle update timing**: if a
sophisticated actor can predict when an oracle price update will arrive and
submit `place_prediction` in the same ledger close, they may exploit the
outcome. This is mitigated by the `prediction_cooldown_seconds` guard and the
recommendation to enforce a resolution delay between the last price update and
pool resolution (`resolution_delay` config parameter).

---

## 5. Summary

| Finding | Severity | Status |
|---|---|---|
| Outcome visibility in plaintext params | Low | Accepted — Stellar lacks public mempool |
| Parimutuel odds observation | Low | Mitigated by stake caps |
| Classical MEV / reordering | Not applicable | SCP prevents miner reordering |
| Oracle timing front-run | Low | Mitigated by `resolution_delay` |
| Commit-reveal scheme | Advisory | Recommended for high-value pools |
