#!/usr/bin/env bash
# wasm_size_check.sh
# ---------------------------------------------------------------------------
# Checks the size of the compiled predifi-contract WASM binary and fails
# the build if it exceeds WASM_SIZE_THRESHOLD_KB (default: 200 KB).
#
# Usage (run from the `contract/` workspace root):
#   bash scripts/wasm_size_check.sh
#
# Environment variables:
#   WASM_SIZE_THRESHOLD_KB  Maximum allowed size in kibibytes (default: 200)
# ---------------------------------------------------------------------------

set -euo pipefail

WASM_FILE="target/wasm32-unknown-unknown/release/predifi_contract.wasm"
THRESHOLD_KB="${WASM_SIZE_THRESHOLD_KB:-200}"

# ── Sanity checks ────────────────────────────────────────────────────────────
if [[ ! -f "${WASM_FILE}" ]]; then
  echo "❌  WASM file not found: ${WASM_FILE}"
  echo "    Make sure you have run:"
  echo "      cargo build --target wasm32-unknown-unknown --release"
  exit 1
fi

# ── Size calculation ─────────────────────────────────────────────────────────
BYTES=$(wc -c < "${WASM_FILE}")
KB=$(( BYTES / 1024 ))

echo "──────────────────────────────────────────────"
echo " WASM Size Report"
echo "──────────────────────────────────────────────"
echo "  File      : ${WASM_FILE}"
echo "  Size      : ${BYTES} bytes  (${KB} KB)"
echo "  Threshold : ${THRESHOLD_KB} KB"
echo "──────────────────────────────────────────────"

# ── Decision ────────────────────────────────────────────────────────────────
if (( KB > THRESHOLD_KB )); then
  echo ""
  echo "❌  WASM size check FAILED!"
  echo "    ${KB} KB exceeds the ${THRESHOLD_KB} KB threshold."
  echo "    Optimise the contract or raise WASM_SIZE_THRESHOLD_KB intentionally."
  exit 1
else
  echo ""
  echo "✅  WASM size check PASSED  (${KB} KB / ${THRESHOLD_KB} KB)"
  exit 0
fi
