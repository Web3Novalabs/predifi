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
