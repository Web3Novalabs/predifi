#!/bin/bash

# test_deploy_optimization.sh - Unit test for wasm-opt integration in deploy.sh

set -euo pipefail

echo "🧪 Testing deploy.sh wasm-opt -O3 optimization step..."

# Mock wasm-opt (record calls, don't actually optimize)
export PATH=".:$PATH"
cat > mock_wasm_opt << 'EOF'
#!/bin/bash
echo "mock_wasm_opt called: $*" > /tmp/wasm_opt_calls.log
exit 0
EOF
chmod +x mock_wasm_opt
export PATH="/tmp:$PATH"  # Use mock in PATH

# Create mock environment
mkdir -p mock_target/wasm32-unknown-unknown/release
touch mock_target/wasm32-unknown-unknown/release/{access_control,predifi_contract}.wasm

# Variables to match deploy.sh
WASM_DIR="$(pwd)/mock_target/wasm32-unknown-unknown/release"
AC_WASM="$WASM_DIR/access_control.wasm"
PD_WASM="$WASM_DIR/predifi_contract.wasm"

# Source the relevant snippet (simulate the step)
source <(cat << 'SCRIPT'
# 2. Run explicit wasm-opt -O3 for smaller deployment footprints (Soroban/Rust best practice)
echo "--- ⚡ Step 2: wasm-opt -O3 Optimization ---"
wasm-opt -O3 -o "${AC_WASM}.opt.wasm" "$AC_WASM" || { echo "❌ wasm-opt failed for access_control"; exit 1; }
wasm-opt -O3 -o "${PD_WASM}.opt.wasm" "$PD_WASM" || { echo "❌ wasm-opt failed for predifi_contract"; exit 1; }

AC_WASM_OPT="${AC_WASM}.opt.wasm"
PD_WASM_OPT="${PD_WASM}.opt.wasm"
SCRIPT
)

# Verify calls logged correctly
if [[ ! -f /tmp/wasm_opt_calls.log ]]; then
  echo "❌ FAIL: wasm-opt calls not logged"
  exit 1
fi

CALLS=$(cat /tmp/wasm_opt_calls.log)

if [[ $CALLS != *"mock_wasm_opt called: -O3 -o "*access_control.wasm.opt.wasm* "*access_control.wasm* ]]; then
  echo "❌ FAIL: access_control wasm-opt call incorrect: $CALLS"
  exit 1
fi

if [[ $CALLS != *"mock_wasm_opt called: -O3 -o "*predifi_contract.wasm.opt.wasm* "*predifi_contract.wasm* ]]; then
  echo "❌ FAIL: predifi_contract wasm-opt call incorrect: $CALLS"
  exit 1
fi

if [[ ! -f "${AC_WASM}.opt.wasm" ]]; then
  echo "❌ FAIL: access_control.opt.wasm not created"
  exit 1
fi

if [[ ! -f "${PD_WASM}.opt.wasm" ]]; then
  echo "❌ FAIL: predifi_contract.opt.wasm not created"
  exit 1
fi

echo "✅ PASS: wasm-opt -O3 integration test passed!"
rm -f /tmp/wasm_opt_calls.log mock_wasm_opt *.wasm *.opt.wasm mock_target

