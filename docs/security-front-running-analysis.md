# Security Analysis: Front-Running Protection for Predictions

**Issue:** #1553  
**Scope:** `contract/contracts/predifi-contract/src/prediction.rs` — `place_prediction`

## Overview

This document analyses `place_prediction` for front-running vulnerabilities,
evaluates whether prediction outcomes can be influenced by transaction ordering,
assesses commit-reveal scheme feasibility, and documents MEV implications on
Stellar/Soroban.

---

## 1. Front-Running Analysis of `place_prediction`

`place_prediction` accepts `(user, pool_id, amount, outcome, referrer,
invite_key)` as plaintext parameters. The chosen `outcome` and `amount` are
also emitted in `PredictionPlacedEvent` after a successful stake. An observer
who can see a pending transaction before ledger close could:

1. **Outcome copying** — See that a well-informed user is predicting outcome X,
   then submit the same prediction (or the opposite side on a parimutuel pool)
   with a fee bump hoping to land in the same or earlier ledger.
2. **Parimutuel odds sniping** — Payout ratio shifts as stake accumulates on
   each side. A late transaction that lands in the same ledger after a large
   opposing stake gets a slightly better implied price.
3. **Oracle-timing race** — If an oracle price update and a prediction share a
   ledger close, a sophisticated actor who can predict the update may stake on
   the soon-to-be-winning side. Resolution is gated by `end_time +
   resolution_delay`, which is the primary control for this vector.

**What cannot be influenced by ordering**

- `user.require_auth()` binds the stake to the signer. An attacker cannot
  redirect someone else's prediction.
- Reentrancy is blocked (`enter_reentrancy_guard` / `exit_reentrancy_guard`).
- Stake accounting is atomic within a single Soroban invocation: `total_stake`
  and `OutcomeStake` update together or not at all. Ordering across *different*
  transactions can still change pool totals; ordering *inside* one transaction
  cannot split the stake.
- Pool state (`Active`), `end_time`, whitelist, and stake caps are checked
  before the transfer. A front-runner cannot open a closed pool or exceed
  `max_total_stake` by reordering.

**Mitigations already in place**

- `prediction_cooldown_seconds` rate-limits consecutive predictions from the
  same address (error `RateLimitOrSuspiciousActivity`). Default is `0`
  (disabled); operators should set a non-zero value on mainnet.
- `max_stake` / `max_total_stake` cap how much advantage a single front-run
  can extract.
- `resolution_delay` creates a quiet period between market close and
  resolution so last-second oracle updates cannot be paired with a prediction
  in the same close as settlement.
- `HIGH_VALUE_THRESHOLD` emits `HighValuePredictionEvent` for monitoring
  unusually large stakes.

---

## 2. Transaction Ordering on Stellar/Soroban

Stellar does not expose a public mempool like Ethereum. Transactions are
submitted directly to validator nodes and included in the next ledger close
(~5 seconds). Relevant properties:

- **No public mempool.** Pending transactions are not globally gossiped before
  inclusion. Observing a specific `place_prediction` before close requires
  running (or compromising) a validator / Horizon ingestion path.
- **SCP ordering.** The Stellar Consensus Protocol determines ledger contents.
  No single operator can unilaterally reorder third-party transactions for
  profit the way a Bitcoin/Ethereum miner historically could.
- **Fee bumps are account-scoped.** A fee-bump transaction can wrap *that
  account's* transaction. It cannot cut in front of an arbitrary third party
  the way Ethereum priority fees can.
- **Soroban invocation is atomic.** Within one contract call, state updates
  are all-or-nothing. Cross-invocation MEV (sandwiching) requires two
  separate transactions in a controlled order, which SCP does not sell.

**Conclusion:** Classical MEV (miner-extractable value) as seen on Ethereum
is largely absent. Residual risk is concentrated on (a) validator-adjacent
observers, (b) oracle update timing, and (c) public event copying *after*
a prediction is already on-chain (copy-trading, not true front-running).

---

## 3. Commit-Reveal Scheme Feasibility

A commit-reveal scheme would hide the outcome until after ordering is fixed:

1. **Commit** — `commit_prediction(user, pool_id, amount, hash(outcome || nonce || user), referrer, invite_key)`.
   Tokens are escrowed; only the hash is stored under `DataKey::Commit(user, pool_id)`.
2. **Reveal** — `reveal_prediction(user, pool_id, outcome, nonce)` after at least
   one ledger (`env.ledger().sequence()`) and within an expiry window. The
   contract checks `sha256(outcome || nonce || user)` and then records the
   prediction the same way `place_prediction` does today.

**Feasibility on Soroban**

- `soroban_sdk::crypto` exposes `sha256` / `keccak256`.
- Persistent storage can hold `(hash, amount, ledger, expiry)` per
  `(user, pool_id)`.
- A minimum of one ledger between commit and reveal is enforceable.
- Unrevealed commits can expire and refund the escrow so slots are not
  permanently locked.

**Trade-offs**

| Factor | Impact |
|---|---|
| UX | Two signatures, two fees, two round-trips (~5–10 s extra) |
| Capital lock | Stake is escrowed during the commit window |
| Griefing | Attacker commits and never reveals (mitigated by expiry + refund) |
| Gas | Extra storage writes + hash verify on reveal |
| Practical benefit on Stellar | Low — no public mempool to hide from |

**Recommendation:** Keep plaintext `place_prediction` as the default. Treat
commit-reveal as an **opt-in per-pool flag** (`pool.commit_reveal_required`)
for high-value or politically sensitive markets, not a protocol-wide
requirement. The cooldown + stake caps + `resolution_delay` are the
right default controls on Stellar.

A sketch of the storage key (not implemented, reserved for a future change):

```text
DataKey::PredictionCommit(Address, u64) -> (BytesN<32>, i128, u32 /*ledger*/, u64 /*expiry*/)
```

---

## 4. MEV Implications on Stellar/Soroban

| MEV vector | Ethereum | Stellar/Soroban (PrediFi) |
|---|---|---|
| Mempool sniping | High | Very low — no public mempool |
| Transaction reordering | High (proposer) | Not applicable — SCP quorum |
| Sandwich attacks | High | Very low — no atomic DEX composability in this call |
| Copy-trading after inclusion | Medium | Medium — events expose outcome after close |
| Oracle front-running | Medium | Low — `resolution_delay` + pool `end_time` |
| Fee-bump priority | Priority fee market | Only within a single account's own tx |

The primary residual MEV surface is **oracle update timing** and
**post-inclusion copy-trading**. Neither lets an attacker change *another
user's* outcome; they can only add their own stake. Caps and cooldown bound
the former; the latter is inherent to a public market.

---

## 5. Operator checklist

- Set `prediction_cooldown_seconds` > 0 on mainnet (e.g. 15–60 s).
- Set per-pool `max_stake` / `max_total_stake` for high-TVL markets.
- Keep `resolution_delay` large enough that oracle updates near `end_time`
  cannot be paired with a last-second prediction that is then immediately
  resolved.
- Monitor `HighValuePredictionEvent` and `RateLimitOrSuspiciousActivity`.
- Revisit commit-reveal if a future Stellar change introduces a public
  mempool or proposer-built ordering market.

---

## 6. Summary

| Finding | Severity | Status |
|---|---|---|
| Outcome visibility in plaintext params | Low | Accepted — Stellar lacks a public mempool |
| Parimutuel odds observation in-ledger | Low | Mitigated by stake caps |
| Classical MEV / reordering | Not applicable | SCP prevents miner-style reordering |
| Oracle timing front-run | Low | Mitigated by `resolution_delay` |
| Post-inclusion copy-trading | Informational | Inherent to public events |
| Commit-reveal scheme | Advisory | Recommended only as opt-in for high-value pools |
