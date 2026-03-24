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

WASM_FILE="${WASM_FILE:-target/wasm32-unknown-unknown/release/predifi_contract.wasm}"
THRESHOLD_KB="${WASM_SIZE_THRESHOLD_KB:-200}"
# Soroban hard limit is 256 KB
SOROBAN_HARD_LIMIT_KB=256

# ── Sanity checks ────────────────────────────────────────────────────────────
if [[ ! -f "${WASM_FILE}" ]]; then
  echo "❌  WASM file not found: ${WASM_FILE}"
  echo ""
  echo "    Make sure you have built the contract first:"
  echo "      cargo build --target wasm32-unknown-unknown --release"
  echo ""
  echo "    If you are running this script from a different directory, set:"
  echo "      WASM_FILE=<path/to/predifi_contract.wasm> bash scripts/wasm_size_check.sh"
  exit 1
fi

# ── Size calculation ─────────────────────────────────────────────────────────
BYTES=$(wc -c < "${WASM_FILE}")
KB=$(( BYTES / 1024 ))
EXCEEDED_BY_KB=$(( KB - THRESHOLD_KB ))
HEADROOM_KB=$(( THRESHOLD_KB - KB ))
PERCENT=$(( KB * 100 / THRESHOLD_KB ))

echo "══════════════════════════════════════════════"
echo "  WASM Size Report"
echo "══════════════════════════════════════════════"
printf "  %-12s %s\n" "File:"      "${WASM_FILE}"
printf "  %-12s %d bytes  (%d KB)\n" "Size:"  "${BYTES}" "${KB}"
printf "  %-12s %d KB  (Soroban hard limit: %d KB)\n" "Threshold:" "${THRESHOLD_KB}" "${SOROBAN_HARD_LIMIT_KB}"
printf "  %-12s %d%%  of allowed budget used\n" "Usage:" "${PERCENT}"
echo "══════════════════════════════════════════════"

# ── Decision ────────────────────────────────────────────────────────────────
if (( KB > THRESHOLD_KB )); then
  echo ""
  echo "❌  WASM size check FAILED"
  echo ""
  echo "    Current size : ${KB} KB  (${BYTES} bytes)"
  echo "    Limit        : ${THRESHOLD_KB} KB"
  echo "    Exceeded by  : ${EXCEEDED_BY_KB} KB"
  echo ""
  echo "  ── Suggested optimizations ──────────────────"
  echo ""
  echo "  1. Run wasm-opt (Binaryen) to shrink the binary:"
  echo "       wasm-opt -Oz -o optimized.wasm ${WASM_FILE}"
  echo ""
  echo "  2. Use the Stellar CLI optimizer (wraps wasm-opt):"
  echo "       stellar contract optimize --wasm ${WASM_FILE}"
  echo ""
  echo "  3. Add release-profile tweaks to Cargo.toml:"
  echo "       [profile.release]"
  echo "       opt-level = \"z\"   # optimise for size"
  echo "       lto = true         # link-time optimisation"
  echo "       codegen-units = 1  # single codegen unit"
  echo "       strip = true       # strip debug symbols"
  echo ""
  echo "  4. Audit dependencies — remove unused crates or"
  echo "     replace heavy ones with lighter alternatives."
  echo ""
  echo "  5. If the growth is intentional, raise the limit:"
  echo "       WASM_SIZE_THRESHOLD_KB=<new_limit> bash scripts/wasm_size_check.sh"
  echo "     or update WASM_SIZE_THRESHOLD_KB in .github/workflows/ci.yml."
  echo ""
  echo "  Note: Soroban's hard limit is ${SOROBAN_HARD_LIMIT_KB} KB — the contract"
  echo "        will be rejected on-chain above that size regardless."
  echo "══════════════════════════════════════════════"
  exit 1
else
  echo ""
  echo "✅  WASM size check PASSED"
  echo ""
  echo "    Current size : ${KB} KB  (${BYTES} bytes)"
  echo "    Limit        : ${THRESHOLD_KB} KB"
  echo "    Headroom     : ${HEADROOM_KB} KB remaining"
  echo "    Budget used  : ${PERCENT}%"
  echo "══════════════════════════════════════════════"
  exit 0
fi
