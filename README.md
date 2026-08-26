# PrediFi – Decentralized Outcome Prediction Protocol (Stellar / Soroban)

PrediFi is a decentralized prediction protocol built on the **Stellar network** using **Soroban smart contracts**. It enables users to create and participate in prediction markets in a **trustless, transparent, and verifiable** environment.

All market logic, outcomes, and settlements are executed **on-chain**, ensuring immutability and eliminating reliance on centralized intermediaries.

Telegram Community: https://t.me/predifi_onchain_build/1

---

## 🧠 Architecture Overview

PrediFi follows a **modular smart contract architecture** designed for composability, security, and maintainability, aligned with **Soroban and Rust best practices**.

### Core Design Principles

* **Separation of Concerns** – Each contract has a single responsibility.
* **Reusability** – Shared logic is abstracted into reusable crates.
* **Deterministic Execution** – All state transitions are predictable and verifiable on-chain.
* **Minimal Storage Footprint** – Efficient storage usage to reduce on-chain costs.
* **Explicit Error Handling** – Strongly typed errors across all contracts.

---

### 🏗️ Contract Architecture

#### 1. PrediFi Core Contract (`predifi-contract`)

Main protocol contract.

**Responsibilities:**

* Prediction pool creation and configuration
* User participation (staking on outcomes)
* Pool lifecycle management (open → closed → resolved)
* Outcome resolution (manual or oracle-based)
* Reward distribution

---

#### 2. Access Control Contract (`access-control`)

Reusable **role-based access control (RBAC)** module.

**Responsibilities:**

* Admin and role management
* Permission enforcement
* Decoupled authorization logic

---

#### 3. Shared Errors Crate (`predifi-errors`)

Common error handling across contracts.

**Responsibilities:**

* Standardized error enums
* Consistent failure handling
* Improved debugging and testing

---

### 🔗 Interaction Flow

1. **Pool Creation**

   * Authorized user creates a pool
   * Optional: attach a `PriceCondition`

2. **Participation**

   * Users stake tokens on outcomes

3. **Pool Closure**

   * Pool stops accepting entries after end time

4. **Resolution**

   * Manual OR oracle-based resolution

5. **Payout**

   * Winners claim rewards

---

### 📡 Oracle Integration (PriceFeed)

PrediFi supports automated price-based resolution using external oracles (e.g., Pyth).

**Flow:**

1. Fetch latest price
2. Validate freshness
3. Evaluate condition
4. Resolve outcome

---

## 📁 Project Structure

The repository is organized into two main workspaces:

* `contract/`: Soroban smart contracts (Rust)
* `frontend/`: Next.js app (TypeScript)

### Smart Contracts (`contract/`)

* `contracts/predifi-contract/`: Core prediction logic
* `contracts/access-control/`: RBAC module
* `contracts/predifi-errors/`: Shared error definitions

### Frontend (`frontend/`)

Built with **Next.js**, **Tailwind CSS**, and **TypeScript**

---

## 🚀 Development

### Prerequisites

* Rust → https://www.rust-lang.org/tools/install
* Soroban CLI → https://soroban.stellar.org/docs/getting-started/setup
* Node.js → https://nodejs.org/
* pnpm → https://pnpm.io/installation

---

### Installation & Setup

#### 1. Clone Repository

```bash
git clone https://github.com/Web3Novalabs/predifi.git
cd predifi
```

---

#### 2. Smart Contracts

```bash
cd contract
```

Build:

```bash
soroban contract build
```

Run tests:

```bash
cargo test
```

Install WASM target:

```bash
rustup target add wasm32-unknown-unknown
```

Match CI checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings
cargo build --workspace --target wasm32-unknown-unknown --release
cargo test --workspace
bash scripts/wasm_size_check.sh
```

---

#### 3. Frontend

```bash
cd ../frontend
pnpm install
pnpm dev
```

Open: http://localhost:3000

---

## ⚙️ Backend CI

The backend workflow lives at:

```
.github/workflows/backend.yml
```

Runs:

* Formatting checks
* Clippy
* WASM build
* Unit tests
* Contract size checks

---

## 📊 PriceFeed Integration

PrediFi supports automated, price-based resolution via external oracles.

### Price-based Pool Creation

1. Initialize oracle (admin only)
2. Define `PriceCondition`
3. Attach to pool

---

### PriceCondition Parameters

| Parameter       | Type   | Description                |
| --------------- | ------ | -------------------------- |
| `feed_pair`     | Symbol | Asset pair (e.g., BTC/USD) |
| `target_price`  | i128   | Target price               |
| `operator`      | u32    | 0=Equal, 1=Greater, 2=Less |
| `tolerance_bps` | u32    | Noise buffer               |

---

### Automated Resolution

`resolve_pool_from_price` will:

1. Fetch oracle price
2. Validate data
3. Evaluate condition
4. Resolve outcome

---

## ❗ Backend Error Handling

Unified error system via `AppError`.

| Variant            | HTTP | Description       |
| ------------------ | ---- | ----------------- |
| Validation         | 400  | Invalid input     |
| Unauthorized       | 401  | Auth failure      |
| NotFound           | 404  | Missing resource  |
| Database           | 500  | Query failure     |
| DatabaseConnection | 500  | Connection issues |

Example:

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

---

## 🧪 Testing

* Unit tests across contracts
* Deterministic execution via Soroban SDK
* Edge case coverage

Run:

```bash
cargo test --workspace
```

---

## 🤝 Contributing

1. Fork repo
2. Create branch (`feature/your-feature`)
3. Commit clean code
4. Run tests
5. Open PR

---

## 📄 License

(MIT)[LICENSE]
