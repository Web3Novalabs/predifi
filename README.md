# PrediFi - Decentralized Outcome Prediction Protocol (Stellar/Soroban)

PrediFi is a decentralized prediction protocol built on the **Stellar network** using **Soroban smart contracts**. In a trustless, transparent, and secure environment, it allows users to predict future outcomes across various fields, including sports, finance, and global events. By utilizing Stellar's fast and low-cost technology, PrediFi ensures that all predictions and their results are verifiable on-chain and immutable.

Telegram Community: [here](https://t.me/predifi_onchain_build/1)

## Project Overview

PrediFi is designed to empower individuals, influencers, and communities to enter the dynamic world of prediction markets. Leveraging the power of blockchain, PrediFi allows anyone to establish custom prediction markets focused on any event imaginable.

## Project Structure

The repository is organized into two main workspaces:

- `contract/`: Contains the Stellar/Soroban smart contracts (Rust).
- `frontend/`: Contains the Next.js web application (TypeScript).

### Smart Contracts (`contract/`)

The smart contract logic is written in **Rust** for the **Soroban** platform.

- `contracts/predifi-contract/`: Main prediction market contract.
- `contracts/access-control/`: Shared role-based access control contract.
- `contracts/predifi-errors/`: Shared error types and helpers used across backend crates.

### Frontend (`frontend/`)

The user interface is built with **Next.js**, **Tailwind CSS**, and **TypeScript**.

## Development

### Prerequisites

- **Rust**: [Install Rust](https://www.rust-lang.org/tools/install)
- **Soroban CLI**: [Install Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup#install-the-soroban-cli)
- **Node.js**: [Install Node.js](https://nodejs.org/)
- **pnpm**: [Install pnpm](https://pnpm.io/installation)

### Installation & Setup

1. **Clone the repository:**

   ```bash
   git clone https://github.com/Web3Novalabs/predifi.git
   cd predifi
   ```

2. **Smart Contracts:**

   Navigate to the contract directory:

   ```bash
   cd contract
   ```

   Build the contracts:

   ```bash
   soroban contract build
   ```

   Run tests:

   ```bash
   cargo test
   ```

   Install the WASM target used by CI if you do not already have it:

   ```bash
   rustup target add wasm32-unknown-unknown
   ```

   Match the backend CI checks locally:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings
   cargo build --workspace --target wasm32-unknown-unknown --release
   cargo test --workspace
   bash scripts/wasm_size_check.sh
   ```

3. **Frontend:**

   Navigate to the frontend directory:

   ```bash
   cd ../frontend
   ```

   Install dependencies:

   ```bash
   pnpm install
   ```

   Run the development server:

   ```bash
   pnpm dev
   ```

   Open [http://localhost:3000](http://localhost:3000) with your browser to see the result.

### Backend CI

The backend workflow lives at `.github/workflows/backend.yml`. It runs formatting, clippy, release WASM builds, unit tests, and the contract size check whenever backend files change.

## PriceFeed Integration

PrediFi supports automated, price-based resolution for prediction pools via external oracles (e.g., Pyth Network). This enables markets to settle automatically once a target price is reached.

### Price-based Pool Creation

To create a price-linked pool, follow these steps:

1.  **Initialize Oracle**: The contract admin must register the oracle provider's address and staleness parameters once.
2.  **Define PriceCondition**: Specify the asset pair (e.g., `ETH/USD`), target price, and the comparison operator.
3.  **Setup Pool**: Link the `PriceCondition` to the pool ID using `set_price_condition`.

### PriceCondition Configuration

A `PriceCondition` defines exactly how a pool should be resolved:

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `feed_pair` | `Symbol` | The asset pair identifier (e.g., `BTC/USD`). |
| `target_price` | `i128` | The price level to monitor (using oracle's decimal scale). |
| `operator` | `u32` | `0` (Equal), `1` (Greater Than), `2` (Less Than). |
| `tolerance_bps` | `u32` | Buffer in basis points (1 bp = 0.01%) to prevent noise flips. |

### Automated Resolution

Once the pool's end time is reached, anyone can trigger the resolution by calling `resolve_pool_from_price`. The contract will:
1.  Retrieve the latest price from the oracle.
2.  Verify the price data is fresh and reliable (confidence check).
3.  Evaluate the `PriceCondition`.
4.  Resolve the pool to outcome `1` (Condition Met) or `0` (Condition Not Met).

## Backend Error Handling

The `backend/` crate provides a unified `AppError` enum (via [`thiserror`](https://docs.rs/thiserror)) for all API and database errors.

| Variant | HTTP | When |
| :--- | :--- | :--- |
| `Validation(String)` | 400 | Invalid input / missing field |
| `Unauthorized(String)` | 401 | Missing or invalid auth token |
| `NotFound(String)` | 404 | Resource does not exist |
| `Database(String)` | 500 | Query failure |
| `DatabaseConnection(String)` | 500 | Connection refused / timeout |

```rust
use predifi_backend::AppError;

fn get_pool(id: u64) -> Result<Pool, AppError> {
    db.find(id).ok_or_else(|| AppError::NotFound(format!("pool {id}")))
}
```

Run backend tests:

```bash
cd backend
cargo test
```

## Contributing

We welcome contributions! Please follow these steps:

1. Fork the repository.
2. Create your feature branch (`git checkout -b feature/your-feature-name`).
3. Commit your changes with meaningful messages.
4. Test your changes thoroughly.
5. Submit a Pull Request.

## License

[MIT](LICENSE)
