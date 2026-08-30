#!/bin/bash

# ==============================================================================
# PrediFi Contract Deployment Script (Corrected)
# ==============================================================================
# Automates: Build -> Optimize -> Deploy -> Initialize
# Usage: ./corrected_deploy.sh <network> <source_account>
# Example: ./corrected_deploy.sh testnet default
# ==============================================================================
# NOTE: This corrected version includes ALL required initialization parameters
# that were missing from the original deploy.sh script
# ==============================================================================

set -e

NETWORK=$1
SOURCE=$2

# --- Check Prerequisites ---

if [[ -z "$NETWORK" || -z "$SOURCE" ]]; then
    echo "❌ Error: Missing arguments."
    echo "Usage: $0 <network> <source_account>"
    echo "Example: $0 testnet default"
    echo ""
    echo "Optional environment variables:"
    echo "  TREASURY_ADDRESS    - Treasury address (defaults to admin address)"
    echo "  FEE_BPS             - Protocol fee in basis points (default: 100)"
    echo "  RESOLUTION_DELAY    - Resolution delay in seconds (default: 3600)"
    echo "  MIN_POOL_DURATION   - Minimum pool duration in seconds (default: 3600)"
    echo "  MAX_PREDICTIONS_PER_USER - Max predictions per user (default: 10)"
    exit 1
fi

# Detect CLI command (stellar preferred, fallback to soroban)
if command -v stellar &> /dev/null; then
    CLI="stellar"
    echo "🚀 Using: Stellar CLI"
elif command -v soroban &> /dev/null; then
    CLI="soroban"
    echo "🚀 Using: Soroban CLI"
else
    echo "❌ Error: Neither 'stellar' nor 'soroban' CLI found in PATH."
    echo "Please install it: cargo install stellar-cli"
    exit 1
fi

# Detect wasm-opt (required for optimization)
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
PROJECT_ROOT="$(cd "$SCRIPTS_DIR" && pwd)"
WASM_DIR="$PROJECT_ROOT/target/wasm32-unknown-unknown/release"
OUTPUT_FILE="$SCRIPTS_DIR/deployed_contracts_${NETWORK}_corrected.json"

# Define WASM file paths
AC_WASM="$WASM_DIR/access_control.wasm"
PD_WASM="$WASM_DIR/predifi_contract.wasm"

# Default parameters (can be overridden by environment variables)
ADMIN_ADDRESS=$($CLI keys address "$SOURCE" --network "$NETWORK")
TREASURY_ADDRESS=${TREASURY_ADDRESS:-$ADMIN_ADDRESS}
FEE_BPS=${FEE_BPS:-100}
RESOLUTION_DELAY=${RESOLUTION_DELAY:-3600}
MIN_POOL_DURATION=${MIN_POOL_DURATION:-3600}
MAX_PREDICTIONS_PER_USER=${MAX_PREDICTIONS_PER_USER:-10}

echo "🌐 Network: $NETWORK"
echo "👤 Source Account: $SOURCE"
echo "🔑 Admin Address: $ADMIN_ADDRESS"
echo "💰 Treasury Address: $TREASURY_ADDRESS"
echo "📊 Fee: ${FEE_BPS} bps (${FEE_BPS}%)"
echo "⏰ Resolution Delay: ${RESOLUTION_DELAY} seconds"
echo "⏱️ Min Pool Duration: ${MIN_POOL_DURATION} seconds"
echo "🎯 Max Predictions/User: ${MAX_PREDICTIONS_PER_USER}"

# --- 1. Build Contracts ---
echo ""
echo "--- 📦 Step 1: Building Contracts ---"
cd "$PROJECT_ROOT"

if [[ ! -f "$AC_WASM" ]] || [[ ! -f "$PD_WASM" ]]; then
    echo "Building contracts from source..."
    cargo build --target wasm32-unknown-unknown --release
    
    if [[ $? -ne 0 ]]; then
        echo "❌ Build failed. Check Rust/Cargo installation."
        exit 1
    fi
else
    echo "Using existing WASM files."
fi

# --- 2. Optimize WASM Files ---
echo ""
echo "--- ⚡ Step 2: WASM Optimization ---"

AC_WASM_OPT="$WASM_DIR/access_control_optimized.wasm"
PD_WASM_OPT="$WASM_DIR/predifi_contract_optimized.wasm"

echo "Optimizing AccessControl contract..."
wasm-opt -Oz --enable-bulk-memory "$AC_WASM" -o "$AC_WASM_OPT"

echo "Optimizing PrediFi contract..."
wasm-opt -Oz --enable-bulk-memory "$PD_WASM" -o "$PD_WASM_OPT"

# Optional: Additional Stellar CLI optimization
echo "Running Stellar CLI optimization..."
$CLI contract optimize --wasm "$AC_WASM_OPT" 2>/dev/null || echo "Warning: Stellar optimization failed, continuing anyway..."
$CLI contract optimize --wasm "$PD_WASM_OPT" 2>/dev/null || echo "Warning: Stellar optimization failed, continuing anyway..."

# --- 3. Deploy AccessControl Contract ---
echo ""
echo "--- 🛡️ Step 3: Deploying AccessControl Contract ---"

AC_ID=$($CLI contract deploy \
    --wasm "$AC_WASM_OPT" \
    --source "$SOURCE" \
    --network "$NETWORK")

if [[ -z "$AC_ID" ]]; then
    echo "❌ Failed to deploy AccessControl contract"
    exit 1
fi

echo "✅ AccessControl Contract ID: $AC_ID"

# --- 4. Initialize AccessControl ---
echo ""
echo "--- ⚙️ Step 4: Initializing AccessControl ---"

echo "Setting admin to: $ADMIN_ADDRESS"
$CLI contract invoke \
    --id "$AC_ID" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    -- \
    init \
    --admin "$ADMIN_ADDRESS"

if [[ $? -eq 0 ]]; then
    echo "✅ AccessControl initialized successfully"
else
    echo "❌ Failed to initialize AccessControl"
    exit 1
fi

# --- 5. Deploy PrediFi Contract ---
echo ""
echo "--- ⚖️ Step 5: Deploying PrediFi Contract ---"

PD_ID=$($CLI contract deploy \
    --wasm "$PD_WASM_OPT" \
    --source "$SOURCE" \
    --network "$NETWORK")

if [[ -z "$PD_ID" ]]; then
    echo "❌ Failed to deploy PrediFi contract"
    exit 1
fi

echo "✅ PrediFi Contract ID: $PD_ID"

# --- 6. Initialize PrediFi Contract (WITH ALL REQUIRED PARAMETERS) ---
echo ""
echo "--- ⚙️ Step 6: Initializing PrediFi Contract ---"
echo "Initialization Parameters:"
echo "  • Access Control: $AC_ID"
echo "  • Treasury: $TREASURY_ADDRESS"
echo "  • Fee BPS: $FEE_BPS"
echo "  • Resolution Delay: $RESOLUTION_DELAY seconds"
echo "  • Min Pool Duration: $MIN_POOL_DURATION seconds"
echo "  • Max Predictions/User: $MAX_PREDICTIONS_PER_USER"

$CLI contract invoke \
    --id "$PD_ID" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    -- \
    init \
    --access_control "$AC_ID" \
    --treasury "$TREASURY_ADDRESS" \
    --fee_bps "$FEE_BPS" \
    --resolution_delay "$RESOLUTION_DELAY" \
    --min_pool_duration "$MIN_POOL_DURATION" \
    --max_predictions_per_user "$MAX_PREDICTIONS_PER_USER"

if [[ $? -eq 0 ]]; then
    echo "✅ PrediFi contract initialized successfully"
else
    echo "❌ Failed to initialize PrediFi contract"
    echo "Check that all parameters are within valid ranges:"
    echo "  - fee_bps: 0-10000 (0-100%)"
    echo "  - resolution_delay: > 0"
    echo "  - min_pool_duration: > 0"
    echo "  - max_predictions_per_user: 0 = no limit, > 0 = limit"
    exit 1
fi

# --- 7. Save Deployment Information ---
echo ""
echo "--- 💾 Step 7: Saving Deployment Information ---"

cat <<EOF > "$OUTPUT_FILE"
{
  "network": "$NETWORK",
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "cli_used": "$CLI",
  "source_account": "$SOURCE",
  "admin_address": "$ADMIN_ADDRESS",
  "contracts": {
    "access_control": {
      "id": "$AC_ID",
      "admin": "$ADMIN_ADDRESS"
    },
    "predifi_contract": {
      "id": "$PD_ID",
      "treasury": "$TREASURY_ADDRESS",
      "fee_bps": $FEE_BPS,
      "resolution_delay": $RESOLUTION_DELAY,
      "min_pool_duration": $MIN_POOL_DURATION,
      "max_predictions_per_user": $MAX_PREDICTIONS_PER_USER
    }
  }
}
EOF

echo "✅ Deployment information saved to: $OUTPUT_FILE"

# --- 8. Verification ---
echo ""
echo "--- 🔍 Step 8: Verification ---"

echo "Verifying contract initialization..."
echo "Checking treasury address..."

TREASURY_CHECK=$($CLI contract read \
    --id "$PD_ID" \
    --network "$NETWORK" \
    -- \
    get_treasury 2>/dev/null || echo "")

if [[ "$TREASURY_CHECK" == "$TREASURY_ADDRESS" ]]; then
    echo "✅ Treasury address verified: $TREASURY_CHECK"
else
    echo "⚠️ Treasury verification failed. Expected: $TREASURY_ADDRESS, Got: $TREASURY_CHECK"
fi

echo ""
echo "🎉 Deployment Complete!"
echo ""
echo "📋 Summary:"
echo "  Network: $NETWORK"
echo "  AccessControl ID: $AC_ID"
echo "  PrediFi Contract ID: $PD_ID"
echo "  Treasury: $TREASURY_ADDRESS"
echo "  Fee: ${FEE_BPS} bps"
echo ""
echo "📄 Contract details saved to: $OUTPUT_FILE"
echo ""
echo "📝 Next Steps:"
echo "  1. Whitelist tokens: ./scripts/whitelist_tokens.sh"
echo "  2. Assign operator roles: ./scripts/assign_roles.sh"
echo "  3. Initialize oracle: ./scripts/init_oracle.sh"
echo "  4. Test pool creation"
echo ""
echo "⚠️ IMPORTANT: Backup your contract IDs and secret keys securely!"
