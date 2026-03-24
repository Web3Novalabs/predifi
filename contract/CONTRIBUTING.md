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
