 Soroban Project

## Project Structure

This repository uses the recommended structure for a Soroban project:

```text
.
├── contracts
│   └── hello_world
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

- New Soroban contracts can be put in `contracts`, each in their own directory. There is already a `hello_world` contract in there to get you started.
- If you initialized this project with any other example contracts via `--with-example`, those contracts will be in the `contracts` directory as well.
- Contracts should have their own `Cargo.toml` files that rely on the top-level `Cargo.toml` workspace for their dependencies.
- Frontend libraries can be added to the top-level directory as well. If you initialized this project with a frontend template via `--frontend-template` you will have those files already included.

## 💹 PriceFeed Integration & Price-based Pools

Predifi supports automated "Price-based Pools" that resolve automatically based on real-time asset prices from decentralized oracles (e.g., Pyth Network).

### PriceCondition Configuration

To enable automated resolution, a pool must be associated with a `PriceCondition` struct:

| Field | Type | Description |
| :--- | :--- | :--- |
| `asset` | `Asset` | The asset pair identifier (e.g., `"ETH/USD"`). |
| `target_price` | `i128` | The threshold price for comparison. |
| `compare_op` | `ComparisonOp` | `Equal`, `GreaterThan`, or `LessThan`. |

### Integration Process (Two-Step Initialization)

Price-based pools are initialized in two distinct steps to ensure flexibility:

1.  **Pool Creation**: Create a standard prediction pool. Note the returned `pool_id`.
2.  **Attach Condition**: Call `set_price_condition` with the `pool_id` and the desired `PriceCondition` parameters. This requires `Operator` role authorization.

### Automated Resolution

Once the pool's `end_time` plus the global `resolution_delay` has passed, anyone can call `resolve_pool_from_price`. The contract will fetch the latest price and resolve the pool automatically to Outcome 1 (condition met) or Outcome 0 (condition not met).

## 🔐 Oracle Trust Model

`oracle.rs` and `price_feed.rs` are the only places where data from outside the chain enters pool resolution, so they define the protocol's main trust assumption. This section documents that assumption explicitly.

### What "the oracle" actually is

- The contract does **not** verify Pyth signatures or make a cross-contract call into the `pyth_contract` address configured via `init_oracle`. That address is stored purely as metadata (`get_oracle_config`) — nothing in `oracle.rs` ever invokes it. The contract has no on-chain way to confirm a submitted price actually came from Pyth or any other real feed.
- Trust is placed entirely in the `OracleWl` whitelist maintained by Admin (`add_oracle` / `remove_oracle`, role 0). Any whitelisted address can call `update_price_feed` and have its numbers accepted; fetching genuine off-chain price data and validating it before submission is the responsibility of whoever runs that whitelisted keeper, not the contract.
- Manual resolution via `oracle_resolve` accepts an outcome from any address holding the Oracle role (role 3). The contract has no way to know whether that account's vote reflects real-world events.

### What the on-chain guards do and do not cover

- **Deviation cap**: `update_price_feed` rejects a new price more than `MAX_PRICE_DEVIATION_MULTIPLIER` (5x) away from the previously stored price for that pair. This limits a single bad update once a legitimate baseline exists, but provides no protection for the first-ever update to a pair (the bound is `[i128::MIN, i128::MAX]` when no prior price exists).
- **Staleness checks** (`max_price_age`, `expires_at`, timestamp-must-be-in-the-past) stop replay of old, previously-valid prices. They do nothing to stop a whitelisted oracle from submitting a fabricated but fresh price.
- **Multi-oracle voting** (`oracle_resolve`): a pool only resolves once `required_resolutions` oracle addresses agree; disagreement raises `ResolutionConflict` instead of resolving. This means a single compromised or malicious oracle cannot unilaterally resolve a pool as long as `required_resolutions > 1`. Setting `required_resolutions = 1` collapses this protection to trusting one address.
- There is no on-chain slashing, staking, or reputation mechanism for oracles. `remove_oracle` is the only recourse once a whitelisted address misbehaves, and it is reactive — it stops future updates, it does not undo one already accepted.

### The real root of trust is the Admin key

Whoever holds the role-0 Admin key controls this entire trust model: they choose which addresses are whitelisted oracles (`add_oracle` / `remove_oracle`), set the staleness/confidence parameters (`init_oracle`), and grant the Operator and Oracle roles used by `set_price_condition` and `oracle_resolve`. Reviewers and integrators evaluating oracle risk should focus on Admin key custody and the whitelisting process, not the price-comparison arithmetic, which is a secondary safeguard at best.

### Fallback

If oracle data is stale, missing, or manipulated beyond what the guards above catch, an Operator can still resolve a pool manually via `resolve_pool`, independent of both the price-feed and oracle-vote paths.

### Private Pool Whitelist Helper

The contract exposes a public read-only `is_whitelisted(pool_id, user)` helper.
It returns `true` only when that address has an explicit whitelist entry stored
for the pool, and `false` otherwise.
