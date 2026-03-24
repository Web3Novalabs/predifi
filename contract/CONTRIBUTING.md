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
