#!/bin/bash

# ==============================================================================
# PrediFi Contract Deployment Script
# ==============================================================================
# Automates: Build -> Optimize -> Deploy -> Initialize
# Usage: ./deploy.sh <network> <source_account>
# Example: ./deploy.sh testnet default
# ==============================================================================

set -e

NETWORK=$1
SOURCE=$2

# --- Check Prerequisites ---

if [[ -z "$NETWORK" || -z "$SOURCE" ]]; then
    echo "❌ Error: Missing arguments."
    echo "Usage: $0 <network> <source_account>"
    exit 1
fi

# Detect CLI command (stellar preferred, fallback to soroban)
if command -v stellar &> /dev/null; then
    CLI="stellar"
elif command -v soroban &> /dev/null; then
    CLI="soroban"
else
    echo "❌ Error: Neither 'stellar' nor 'soroban' CLI found in PATH."
    echo "Please install it: cargo install stellar-cli"
    exit 1
fi

echo "🚀 Using CLI: $CLI"
echo "🌐 Network: $NETWORK"
echo "👤 Source Account: $SOURCE"

# Detect wasm-opt (required for Step 2 optimization pass)
if ! command -v wasm-opt &> /dev/null; then
    echo "❌ Error: 'wasm-opt' not found in PATH."
    echo "Install it via your system package manager:"
    echo "  Debian/Ubuntu : sudo apt-get install -y binaryen"
    echo "  macOS (Homebrew): brew install binaryen"
    echo "  Cargo         : cargo install wasm-opt"
    exit 1
fi

# --- Configuration ---

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPTS_DIR/.." && pwd)"
WASM_DIR="$PROJECT_ROOT/target/wasm32-unknown-unknown/release"
OUTPUT_FILE="$SCRIPTS_DIR/deployed_contracts_$NETWORK.json"

# Default parameters (can be overridden by environment variables)
# TREASURY_ADDRESS will default to admin address if not set
# FEE_BPS default to 100 (1%)
FEE_BPS=${FEE_BPS:-100}

# 1. Build Contracts
echo "--- 📦 Step 1: Building Contracts ---"
cd "$PROJECT_ROOT"
cargo build --target wasm32-unknown-unknown --release

# 2. Identify WASM files
# Rust converts dashes to underscores in filenames
AC_WASM="$WASM_DIR/access_control.wasm"
PD_WASM="$WASM_DIR/predifi_contract.wasm"

if [ ! -f "$AC_WASM" ] || [ ! -f "$PD_WASM" ]; then
    echo "❌ Error: WASM files not found in $WASM_DIR"
    ls -l "$WASM_DIR"/*.wasm
    exit 1
fi

# 3. wasm-opt -O3 (first-pass: general perf + size optimizations)
echo "--- ⚡ Step 2: Running wasm-opt -O3 ---"
# wasm-opt buffers the full output before writing, so in-place use is safe.
# This pass runs before stellar contract optimize, which applies a second
# Soroban-specific pass (-Oz) to produce the final .optimized.wasm binaries.
wasm-opt -O3 --strip-debug -o "$AC_WASM" "$AC_WASM"
wasm-opt -O3 --strip-debug -o "$PD_WASM" "$PD_WASM"

# 4. Optimize Contracts (second pass via stellar CLI)
echo "--- ✨ Step 3: Optimizing Contracts (stellar contract optimize) ---"
$CLI contract optimize --wasm "$AC_WASM"
$CLI contract optimize --wasm "$PD_WASM"

AC_WASM_OPT="$WASM_DIR/access_control.optimized.wasm"
PD_WASM_OPT="$WASM_DIR/predifi_contract.optimized.wasm"

# 5. Get Admin Address
ADMIN_ADDRESS=$($CLI keys address "$SOURCE")
echo "🔑 Admin/Deployer Address: $ADMIN_ADDRESS"

TREASURY_ADDRESS=${TREASURY_ADDRESS:-$ADMIN_ADDRESS}

# 6. Deploy & Initialize AccessControl
echo "--- 🛡️ Step 4: Deploying AccessControl ---"
AC_ID=$($CLI contract deploy \
    --wasm "$AC_WASM_OPT" \
    --source "$SOURCE" \
    --network "$NETWORK")

echo "✅ AccessControl ID: $AC_ID"

echo "⚙️ Initializing AccessControl with admin $ADMIN_ADDRESS..."
$CLI contract invoke \
    --id "$AC_ID" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    -- \
    init \
    --admin "$ADMIN_ADDRESS"

# 7. Deploy & Initialize PredifiContract
echo "--- ⚖️ Step 5: Deploying PredifiContract ---"
PD_ID=$($CLI contract deploy \
    --wasm "$PD_WASM_OPT" \
    --source "$SOURCE" \
    --network "$NETWORK")

echo "✅ PredifiContract ID: $PD_ID"

echo "⚙️ Initializing PredifiContract..."
echo "   - Access Control: $AC_ID"
echo "   - Treasury: $TREASURY_ADDRESS"
echo "   - Fee (BPS): $FEE_BPS"

$CLI contract invoke \
    --id "$PD_ID" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    -- \
    init \
    --access_control "$AC_ID" \
    --treasury "$TREASURY_ADDRESS" \
    --fee_bps "$FEE_BPS"

# 8. Store Deployment IDs
echo "--- 💾 Step 6: Saving Deployment Info ---"
cat <<EOF > "$OUTPUT_FILE"
{
  "network": "$NETWORK",
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "cli_used": "$CLI",
  "source_account": "$SOURCE",
  "contracts": {
    "access_control": {
      "id": "$AC_ID",
      "admin": "$ADMIN_ADDRESS"
    },
    "predifi_contract": {
      "id": "$PD_ID",
      "treasury": "$TREASURY_ADDRESS",
      "fee_bps": $FEE_BPS
    }
  }
}
EOF

echo "🎉 Deployment complete for $NETWORK!"
echo "📄 Contract IDs saved to: $OUTPUT_FILE"
