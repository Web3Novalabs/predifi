#!/bin/bash
set -e

echo "Building Soroban contracts..."

# Build using soroban CLI which handles proper WASM generation
soroban contract build

echo ""
echo "Optimizing WASM files..."

# Check if wasm-opt is available
if ! command -v wasm-opt &> /dev/null; then
    echo "Error: wasm-opt not found. Please install binaryen:"
    echo "  brew install binaryen  # macOS"
    echo "  apt-get install binaryen  # Ubuntu/Debian"
    exit 1
fi

# Optimize contracts with bulk memory support
# Note: Soroban requires bulk memory operations to be enabled
wasm-opt -Oz --enable-bulk-memory \
    target/wasm32-unknown-unknown/release/predifi_contract.wasm \
    -o target/wasm32-unknown-unknown/release/predifi_contract_optimized.wasm

wasm-opt -Oz --enable-bulk-memory \
    target/wasm32-unknown-unknown/release/access_control.wasm \
    -o target/wasm32-unknown-unknown/release/access_control_optimized.wasm

echo ""
echo "Build complete!"
echo ""
echo "Original WASM files:"
ls -lh target/wasm32-unknown-unknown/release/predifi_contract.wasm
ls -lh target/wasm32-unknown-unknown/release/access_control.wasm
echo ""
echo "Optimized WASM files (use these for deployment):"
ls -lh target/wasm32-unknown-unknown/release/predifi_contract_optimized.wasm
ls -lh target/wasm32-unknown-unknown/release/access_control_optimized.wasm
