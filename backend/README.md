# Predifi Backend

[![CI/CD Pipeline](https://github.com/your-username/predifi-backend/workflows/Predifi%20Backend%20CI%2FCD/badge.svg)](https://github.com/your-username/predifi-backend/actions)

A robust, production-ready backend service for the Predifi platform with comprehensive integration testing and CI/CD automation.

## Backend Setup

## 1. Environment Configuration

1. Copy the example environment file:
   ```sh
   cp .env.example .env
   ```
2. Edit `.env` if needed to match your local setup (the default should work with Docker Compose).

## 2. Start the Database

Start the PostgreSQL database using Docker Compose:
```sh
docker compose up -d
```
This will run the database in the background.

## 3. Run the Backend

Start the backend server locally:
```sh
cargo run
```
You can also use cargo watch which works like nodemon:
```sh
cargo watch -x run
```

The backend will connect to the database running in Docker.

## 4. Testing

### Local Testing Setup

1. **Database Setup**: Ensure you have a test database running:
   ```sh
   # Create test database (if using local PostgreSQL)
   createdb predifi_test
   
   # Or use Docker for testing
   docker run --name predifi-test-db \
     -e POSTGRES_DB=predifi_test \
     -e POSTGRES_USER=ew \
     -p 5433:5432 \
     -d postgres:15
   ```

2. **Environment Variables**: Set up test environment:
   ```sh
   export DATABASE_URL="postgres://ew@localhost:5433/predifi_test"
   ```

3. **Run Tests**:
   ```sh
   # Run all tests
   cargo test --tests -- --nocapture
   
   # Run specific test categories
   cargo test --test test_market_api
   cargo test --test test_pool_api
   cargo test --test test_validator_api
   
   # Run with verbose output
   cargo test --tests -- --nocapture --test-threads=1
   ```

### Test Categories

- **Unit Tests**: `cargo test --lib --bins`
- **Integration Tests**: `cargo test --tests`
- **Health Endpoints**: Tests database connection and migrations
- **Market API**: Tests market creation, retrieval, and persistence
- **Pool API**: Tests pool management and data persistence
- **Validator API**: Tests validator operations and constraints
- **Pool Controller**: Tests direct controller function calls

### Test Architecture

- **Transaction Isolation**: Each test runs in its own database transaction
- **Automatic Cleanup**: Tests auto-rollback, no manual cleanup needed
- **Isolated Environment**: Tests don't interfere with each other
- **Real Database**: Tests use actual PostgreSQL with migrations

## 5. CI/CD Pipeline

### GitHub Actions

The project includes a comprehensive CI/CD pipeline that runs automatically on:

- **Push to main/develop branches**
- **Pull requests to main/develop branches**

### Pipeline Features

1. **Integration Testing**: Runs all tests against PostgreSQL 15
2. **Code Quality**: Clippy linting and format checking
3. **Security Audit**: Cargo audit for vulnerability scanning
4. **Database Setup**: Automatic test database creation and migration
5. **Caching**: Optimized dependency caching for faster builds

### Pipeline Jobs

- **Integration Tests**: Runs all tests with PostgreSQL service
- **Security Audit**: Scans dependencies for vulnerabilities
- **Results Summary**: Provides clear pass/fail notifications

### Local CI Validation

To validate your changes locally before pushing:

```sh
# Run the same checks as CI
cargo clippy -- -D warnings
cargo fmt -- --check
cargo test --tests -- --nocapture
cargo build --release
```

## 6. Stopping Services

To stop the database:
```sh
docker compose down
```

---

**Note:**
- Make sure Docker is running before starting the database.
- The `.env` file must match the credentials in `docker-compose.yml`.
- All tests must pass before merging to main branch.
- The CI pipeline will automatically validate all changes.
