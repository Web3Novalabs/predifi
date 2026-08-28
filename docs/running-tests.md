# Running the Test Suites

This document outlines the commands and requirements for running test suites across each layer of the PrediFi project: the Next.js frontend, the Axum backend (unit and integration tests), and the Soroban smart contracts.

---

## Test Suites Overview

| Area | Command | Test Framework | Docker Required? | Scope |
| :--- | :--- | :--- | :--- | :--- |
| **Frontend** | `cd frontend && pnpm test` | Jest + React Testing Library | No | React components, UI hooks, state managers, and formatting utilities |
| **Backend Unit Tests** | `cd backend && cargo test` | Rust built-in test runner | No | Route validation, types, caching logic, rate limiting, and business logic |
| **Backend Integration Tests** | `cd backend && cargo test --features integration-tests` | `testcontainers` (Rust) | **Yes** (Docker daemon) | Real PostgreSQL migrations, schema roundtrips, and database queries |
| **Contract Unit Tests** | `cd contract && cargo test` | Soroban Rust SDK test env | No | Contract mechanics, authorization checks, odds calculations, and payouts |
| **Contract Full Workspace** | `bash contract/scripts/test_all.sh` | Bash + Cargo workspace | No | Sequential run across `predifi-errors`, `access-control`, and `predifi-contract` with summary report |

---

## 1. Frontend Tests

The frontend uses **Jest** with `ts-jest` and `@testing-library/react`.

### Command
```bash
cd frontend && pnpm test
```

### Options & Common Workflows
- **Run tests in watch mode** (during component development):
  ```bash
  cd frontend && pnpm test -- --watch
  ```
- **Run a specific test file**:
  ```bash
  cd frontend && pnpm test -- src/components/Header.test.tsx
  ```
- **Run with coverage report**:
  ```bash
  cd frontend && pnpm test -- --coverage
  ```

### Requirements
- Node.js 20+
- `pnpm` (`pnpm install` must be run once inside `frontend/`)
- *No Docker or external services required.*

---

## 2. Backend Unit Tests

The backend uses Rust's built-in `cargo test` harness. In unit test builds, rate limiters are no-ops and database connections are mocked to ensure fast, isolated execution.

### Command
```bash
cd backend && cargo test
```

### Options & Common Workflows
- **Run a specific test module or test name**:
  ```bash
  cd backend && cargo test validated_types
  cd backend && cargo test rate_limit::tests
  ```
- **Run single-threaded** (useful when debugging test output):
  ```bash
  cd backend && cargo test -- --test-threads=1
  ```
- **Show stdout logs during test execution**:
  ```bash
  cd backend && cargo test -- --nocapture
  ```

### Requirements
- Rust toolchain (edition 2021)
- *No Docker or external services required.*

---

## 3. Backend Integration Tests

Integration tests verify database migrations and roundtrip operations against real database instances using the `integration-tests` Cargo feature and `testcontainers`.

### Command
```bash
cd backend && cargo test --features integration-tests
```

### What Needs to be Running
- **Docker Daemon**: The integration test suite uses `testcontainers` to automatically spin up temporary PostgreSQL (`postgres:16-alpine`) and Redis containers. The Docker daemon must be running locally (`docker info` should return successfully).
- The test suite handles container lifecycle (starting, migrating, testing, and tearing down) automatically.

### Running against the Local Docker Compose Stack
If you have the full development stack running via `docker compose up`, you can also run integration tests directly against the local compose database:
```bash
DATABASE_URL="postgres://postgres:postgres@localhost:5432/predifi" \
REDIS_URL="redis://localhost:6379" \
cd backend && cargo test --features integration-tests
```

---

## 4. Smart Contract Tests

Smart contracts are written in Rust using the Soroban SDK. The test environment mocks the Soroban ledger, user authentication, and token balances in-memory.

### Command
```bash
cd contract && cargo test
```

### Running Individual Contract Crates
To test a specific contract package:
```bash
# Test access control logic
cargo test --manifest-path contract/contracts/access-control/Cargo.toml

# Test core prediction pool contract
cargo test --manifest-path contract/contracts/predifi-contract/Cargo.toml

# Test error definitions and conversions
cargo test --manifest-path contract/contracts/predifi-errors/Cargo.toml
```

### Requirements
- Rust toolchain (`wasm32-unknown-unknown` target installed if building wasm artifacts)
- *No Docker or external services required.*

---

## 5. Contract Workspace Test Script

The `test_all.sh` script executes the full test suite sequentially across all three contract crates and prints a consolidated pass/fail summary.

### Command
```bash
bash contract/scripts/test_all.sh
```

### Output Example
```text
══════════════════════════════════════════════
  PrediFi – Test All Crates
  Workspace: /path/to/predifi/contract
══════════════════════════════════════════════

──────────────────────────────────────────────
  Testing: predifi-errors
──────────────────────────────────────────────
✅  predifi-errors PASSED

──────────────────────────────────────────────
  Testing: access-control
──────────────────────────────────────────────
✅  access-control PASSED

──────────────────────────────────────────────
  Testing: predifi-contract
──────────────────────────────────────────────
✅  predifi-contract PASSED

══════════════════════════════════════════════
  Summary
══════════════════════════════════════════════
  Passed : 3 / 3
    ✅  predifi-errors
    ✅  access-control
    ✅  predifi-contract

✅  All crates passed.
══════════════════════════════════════════════
```

---

## Troubleshooting & Performance Tips

1. **Slow `cargo test` compilation**:
   - Run `cargo check` before running tests to quickly catch syntax errors.
   - Target only the crate or test you are working on: `cargo test <test_name>`.
   - Consider enabling `sccache` (`export RUSTC_WRAPPER=sccache`) to cache compilation artifacts.
2. **Integration tests failing to connect to Docker**:
   - Ensure Docker Desktop or the Docker daemon is running: `docker ps`.
   - Check file permissions on the Docker socket (`/var/run/docker.sock`).
3. **Frontend test module resolution**:
   - Ensure dependencies are installed: `cd frontend && pnpm install`.
   - If Jest cache becomes stale: `cd frontend && pnpm test -- --clearCache`.
