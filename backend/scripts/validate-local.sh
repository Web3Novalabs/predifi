#!/bin/bash

# Predifi Backend Local Validation Script
# This script runs the same checks as the CI pipeline locally

set -e

echo "🚀 Predifi Backend Local Validation"
echo "=================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "success" ]; then
        echo -e "${GREEN}✅${NC} $message"
    elif [ "$status" = "error" ]; then
        echo -e "${RED}❌${NC} $message"
    elif [ "$status" = "warning" ]; then
        echo -e "${YELLOW}⚠️${NC} $message"
    fi
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
echo "🔍 Checking prerequisites..."

if ! command_exists cargo; then
    print_status "error" "Rust/Cargo not found. Please install Rust first."
    exit 1
fi

if ! command_exists sqlx; then
    print_status "warning" "SQLx CLI not found. Installing..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

print_status "success" "Prerequisites check completed"

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    print_status "warning" "DATABASE_URL not set. Using default test database..."
    export DATABASE_URL="postgres://ew@localhost:5432/predifi_test"
fi

# Run code quality checks
echo ""
echo "🔍 Running code quality checks..."

echo "Running clippy..."
if cargo clippy -- -D warnings; then
    print_status "success" "Clippy checks passed"
else
    print_status "error" "Clippy checks failed"
    exit 1
fi

echo "Checking code format..."
if cargo fmt -- --check; then
    print_status "success" "Code format check passed"
else
    print_status "error" "Code format check failed"
    print_status "warning" "Run 'cargo fmt' to fix formatting issues"
    exit 1
fi

# Run tests
echo ""
echo "🧪 Running tests..."

echo "Running unit tests..."
if cargo test --lib --bins; then
    print_status "success" "Unit tests passed"
else
    print_status "error" "Unit tests failed"
    exit 1
fi

echo "Running integration tests..."
if cargo test --tests -- --nocapture; then
    print_status "success" "Integration tests passed"
else
    print_status "error" "Integration tests failed"
    exit 1
fi

# Build release version
echo ""
echo "🏗️ Building release version..."
if cargo build --release; then
    print_status "success" "Release build successful"
else
    print_status "error" "Release build failed"
    exit 1
fi

# Final summary
echo ""
echo "🎉 Validation Summary"
echo "===================="
print_status "success" "All checks passed!"
print_status "success" "Ready to push to GitHub"
print_status "success" "CI pipeline should pass automatically"

echo ""
echo "💡 Next steps:"
echo "   git add ."
echo "   git commit -m 'Your commit message'"
echo "   git push origin your-branch"
echo ""
echo "The CI pipeline will automatically run all tests and validations." 