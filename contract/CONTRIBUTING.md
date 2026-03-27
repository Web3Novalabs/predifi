# Contributing to Predifi Smart Contracts

Welcome! We appreciate your interest in contributing to the Predifi smart contracts. This guide will help you set up your environment and understand our development workflow.

## 🛠 Environment Setup

To build and test the Predifi smart contracts, you need the following tools installed:

### 1. Rust
We use the **Rust Edition 2021**. Ensure you have the latest stable version of Rust installed.

```bash
rustup update stable
```

### 2. Stellar CLI
Install the **Stellar CLI (v25.2.0 or higher)** to interact with the Stellar network and Soroban contracts.

```bash
cargo install --locked stellar-cli --version 25.2.0
```

### 3. WASM Target
Add the `wasm32v1-none` target to your Rust toolchain, which is required for Soroban contract compilation.

```bash
rustup target add wasm32v1-none
```

---

## 🚀 Development Workflow

### Building and Testing
We maintain a high standard for code quality. Before submitting a PR, ensure all tests pass and the code is properly formatted.

1. **Run Tests**: Execute all contract tests within the workspace.
   ```bash
   cargo test --workspace
   ```

2. **Formatting**: Ensure your code follows the standard Rust formatting.
   ```bash
   cargo fmt --all --check
   ```

3. **Linting**: Run Clippy to catch common mistakes and improve your code.
   ```bash
   cargo clippy --workspace -- -D warnings
   ```

---

## 🌐 Local Network Deployment

For local development and testing, you can use a local Stellar network or the Testnet.

### Local Network (Docker)
The easiest way to run a local network is using the Stellar Quickstart Docker image.

```bash
docker run --rm -it \
  -p 8000:8000 \
  --name stellar \
  stellar/quickstart:latest \
  --local
```

### Testnet
Alternatively, you can deploy and test against the public Stellar Testnet. Use the Stellar Laboratory or the CLI to fund your accounts.

---

## 📝 Contribution Guidelines

1. **Focus on Quality**: Ensure your code is well-documented and covered by tests.
2. **Commit Messages**: Use clear and descriptive commit messages.
3. **Pull Requests**: Provide a detailed description of your changes and link them to any relevant issues.

Happy coding! 🚀
# Contributing to PrediFi Contracts

Thank you for your interest in contributing to PrediFi! This guide will help you set up your development environment and understand the workflow for contributing to our smart contracts.

## 🛠️ Environment Setup

To build and test the PrediFi smart contracts, you'll need the following tools:

1.  **Rust**: Install the latest stable version of Rust via [rustup](https://rustup.rs/).
    ```bash
    rustup update stable
    ```
2.  **WASM Target**: Add the `wasm32-unknown-unknown` target.
    ```bash
    rustup target add wasm32-unknown-unknown
    ```
3.  **Stellar CLI**: Install the latest version of the Stellar CLI.
    ```bash
    cargo install --locked stellar-cli
    ```

## 💻 Development Workflow

We follow standard Rust development practices. Before submitting a pull request, please ensure your code passes the following checks:

### Formatting
Format your code using `cargo fmt`:
```bash
cargo fmt --all
```

### Linting
Check for common issues and idiomatic improvements using `clippy`:
```bash
cargo clippy --all-targets -- -D warnings
```

### Testing
Run the unit and integration tests:
```bash
cargo test
```

## 🚀 Deployment (Local Testing)

For local development and testing, you can deploy the contracts to a local network.

1.  **Start a Local Network**: (Assuming you have a local node running or use the Stellar Quickstart Docker image)
    ```bash
    # Standard local network setup
    stellar network add local \
      --rpc-url http://localhost:8000/soroban/rpc \
      --network-passphrase "Standalone Network ; February 2017"
    ```

2.  **Generate a Local Key**:
    ```bash
    stellar keys generate --network local alice
    ```

3.  **Build & Deploy**:
    Use the provided deployment script to build and deploy to your local network:
    ```bash
    ./scripts/deploy.sh local alice
    ```

## ✅ Verification

All pull requests must pass the CI checks, which include:
- `cargo fmt --check`
- `cargo clippy`
- `cargo test`

Please ensure your changes are well-documented and include tests where appropriate.

=======

