#!/bin/bash

# Exit on any error
set -e

echo "🚀 Starting Predifi Backend Integration Tests"
echo "=============================================="

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker and try again."
    exit 1
fi

# Function to cleanup on exit
cleanup() {
    echo "🧹 Cleaning up test environment..."
    docker-compose -f docker-compose.test.yml down -v
    echo "✅ Cleanup complete"
}

# Set trap to cleanup on exit
trap cleanup EXIT

# Start test database
echo "🐘 Starting test database..."
docker-compose -f docker-compose.test.yml up -d test-db

# Wait for database to be ready
echo "⏳ Waiting for database to be ready..."
until docker-compose -f docker-compose.test.yml exec -T test-db pg_isready -U test_user -d test_db; do
    echo "Database not ready yet, waiting..."
    sleep 2
done

echo "✅ Database is ready!"

# Set test environment variables
export TEST_DATABASE_URL="postgres://test_user:test_password@localhost:5433/test_db"
export RUST_LOG=info

# Run the tests
echo "🧪 Running integration tests..."
cargo test --tests -- --nocapture

echo "✅ All tests completed successfully!" 