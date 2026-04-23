# Contributing to PrediFi Backend

Welcome! This guide outlines the development workflow and standards for the PrediFi backend. Our backend is built with **Axum** and **Rust**, designed to provide a robust and scalable API for the PrediFi platform.

## 🛠 Environment Setup

To contribute to the backend, you'll need the following tools:

### 1. Rust
Ensure you have the latest stable version of Rust installed.
```bash
rustup update stable
```

### 2. PostgreSQL
The backend uses PostgreSQL for data persistence. You can run it locally or via Docker:
```bash
docker run --name predifi-db -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=predifi -p 5432:5432 -d postgres
```

### 3. SQLx CLI (Optional)
While the connection pool is lazy-initialized, the SQLx CLI is useful for managing migrations.
```bash
cargo install sqlx-cli --no-default-features --features postgres
```

---

## 🚀 Development Workflow

### 1. Configuration
Copy the example environment file and adjust it as needed:
```bash
cd backend
cp .env.example .env
```

### 2. Running the Server
Start the development server:
```bash
cargo run
```
The server will be available at `http://localhost:3000`.

### 3. Auto-Reloading
For a smoother experience, use `cargo-watch`:
```bash
cargo watch -x run
```

---

## 🧪 Testing and Verification

We maintain a high standard for code quality. Before submitting a PR, ensure all checks pass.

### Running Tests
Execute unit and integration tests:
```bash
cargo test
```

### Formatting
Ensure your code follows the standard Rust formatting:
```bash
cargo fmt --all --check
```

### Linting
Run Clippy to catch common mistakes and improve your code:
```bash
cargo clippy --all-targets -- -D warnings
```

---

## 📐 Project Structure

The backend follows a modular structure:
- `src/main.rs`: Entry point and router initialization.
- `src/config.rs`: Typed environment configuration.
- `src/db.rs`: Database connection pool management.
- `src/routes/`: API endpoint definitions versioned by directory (e.g., `v1/`).
- `src/request_logger.rs`: Custom tracing middleware.

---

## 🤝 Best Practices

- **Error Handling**: Use descriptive error types and avoid `unwrap()` or `expect()` in production code.
- **Logging**: Leverage the `tracing` crate for structured logging.
- **Async/Await**: Ensure non-blocking code throughout the application.
- **Security**: Never commit sensitive information (like real database credentials) to the repository.

Thank you for contributing to PrediFi! 🚀
