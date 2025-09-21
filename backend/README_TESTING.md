# Predifi Backend Integration Testing Guide

This guide explains how to set up and run comprehensive integration tests for the Predifi backend.

## 🚀 Quick Start

### Prerequisites

1. **Docker Desktop** - Must be running
2. **Rust toolchain** - Latest stable version
3. **Environment setup** - Database configuration

### Running Tests

#### Option 1: Using the Test Script (Recommended)

```bash
# Make the script executable (first time only)
chmod +x scripts/run_tests.sh

# Run all integration tests
./scripts/run_tests.sh
```

#### Option 2: Manual Setup

```bash
# 1. Start the test database
docker compose -f docker-compose.test.yml up -d test-db

# 2. Wait for database to be ready
docker compose -f docker-compose.test.yml exec test-db pg_isready -U test_user -d test_db

# 3. Set environment variables
export TEST_DATABASE_URL="postgres://test_user:test_password@localhost:5433/test_db"
export RUST_LOG=info

# 4. Run the tests
cargo test --tests -- --nocapture
```

#### Option 3: Using Existing Database

If you have a local PostgreSQL instance running:

```bash
# Set the test database URL to your existing instance
export TEST_DATABASE_URL="postgres://username:password@localhost:5432/database_name"

# Run tests
cargo test --tests -- --nocapture
```

## 🧪 Test Structure

### Test Files

- **`tests/test_health_endpoints.rs`** - Basic integration tests and database setup
- **`tests/test_market_creation.rs`** - Market endpoint tests
- **`tests/test_pool.rs`** - Pool endpoint tests
- **`tests/test_validator_api.rs`** - Validator endpoint tests

### Test Utilities

The integration tests include:

- **Database setup/teardown** - Automatic migration and cleanup
- **Test fixtures** - Sample data for markets, pools, and validators
- **HTTP testing utilities** - Request/response testing helpers
- **Isolated test environment** - Each test runs in isolation

## 🐘 Test Database Configuration

### Docker Compose Test Setup

The `docker-compose.test.yml` file provides:

- **Isolated PostgreSQL 16** instance
- **Port 5433** (avoids conflicts with main database)
- **Automatic health checks** and readiness detection
- **Clean volume management** for test data

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TEST_DATABASE_URL` | `postgres://test_user:test_password@localhost:5433/test_db` | Test database connection string |
| `RUST_LOG` | `info` | Logging level for tests |

## 📋 Test Categories

### 1. Health & Connectivity Tests
- Database connection validation
- Migration verification
- Basic endpoint availability

### 2. Market Endpoint Tests
- Market creation and retrieval
- Data validation and persistence
- Error handling scenarios

### 3. Pool Endpoint Tests
- Pool creation and management
- Market association validation
- Status updates and queries

### 4. Validator Endpoint Tests
- Validator registration
- Contract address validation
- Active status management

## 🔧 Troubleshooting

### Common Issues

#### Docker Not Running
```bash
# Start Docker Desktop
# On macOS: Open Docker Desktop application
# On Linux: sudo systemctl start docker
```

#### Port Conflicts
```bash
# Check if port 5433 is in use
lsof -i :5433

# Modify docker-compose.test.yml to use different port
```

#### Database Connection Issues
```bash
# Verify test database is running
docker compose -f docker-compose.test.yml ps

# Check database logs
docker compose -f docker-compose.test.yml logs test-db
```

#### Migration Failures
```bash
# Ensure migrations directory exists
ls -la migrations/

# Check migration files are valid SQL
cat migrations/*.sql
```

### Debug Mode

Run tests with verbose output:

```bash
# Enable debug logging
export RUST_LOG=debug

# Run specific test with output
cargo test test_database_connection -- --nocapture

# Run all tests with output
cargo test --tests -- --nocapture
```

## 🚀 CI/CD Integration

### GitHub Actions

The tests are designed to run in CI/CD pipelines:

```yaml
# Example GitHub Actions step
- name: Run Integration Tests
  run: |
    docker compose -f docker-compose.test.yml up -d test-db
    export TEST_DATABASE_URL="postgres://test_user:test_password@localhost:5433/test_db"
    cargo test --tests
```

### Local CI Simulation

Test your CI setup locally:

```bash
# Simulate CI environment
export CI=true
export TEST_DATABASE_URL="postgres://test_user:test_password@localhost:5433/test_db"

# Run tests
cargo test --tests
```

## 📊 Test Coverage

### Current Coverage

- ✅ **Database connectivity** - Connection and migration tests
- ✅ **Basic endpoint structure** - App creation and routing
- ✅ **Data fixtures** - Sample data generation
- ✅ **Cleanup utilities** - Test data isolation

### Planned Coverage

- 🔄 **HTTP endpoint testing** - Full request/response validation
- 🔄 **Authentication testing** - Security and access control
- 🔄 **Performance testing** - Load and stress testing
- 🔄 **Edge case testing** - Error conditions and boundary testing

## 🤝 Contributing

### Adding New Tests

1. **Create test file** in `tests/` directory
2. **Use existing utilities** from test files
3. **Follow naming convention** - `test_*.rs`
4. **Include cleanup** - Always clean up test data

### Test Best Practices

- **Isolation** - Each test should be independent
- **Cleanup** - Always clean up after tests
- **Descriptive names** - Test names should explain what they test
- **Assertions** - Use meaningful assertions with clear error messages

## 📚 Additional Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Axum Testing](https://docs.rs/axum/latest/axum/testing/index.html)
- [SQLx Testing](https://docs.rs/sqlx/latest/sqlx/testing/index.html)
- [Testcontainers Rust](https://docs.rs/testcontainers/latest/testcontainers/)

---

**Happy Testing! 🧪✨** 