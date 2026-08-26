#!/usr/bin/env bash
# test_wasm_size_check.sh
# ---------------------------------------------------------------------------
# Unit / integration tests for wasm_size_check.sh.
#
# Usage (run from the `contract/` workspace root):
#   bash scripts/tests/test_wasm_size_check.sh
#
# Exit code: 0 if all tests pass, 1 if any test fails.
# ---------------------------------------------------------------------------

set -euo pipefail

# Resolve paths relative to this test file so the suite can be run from any
# working directory (e.g. repo root, contract/, or scripts/tests/).
TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$(cd "${TESTS_DIR}/.." && pwd)"
SCRIPT="${SCRIPTS_DIR}/wasm_size_check.sh"
PASS=0
FAIL=0

# ── Helpers ──────────────────────────────────────────────────────────────────

# Colour codes (disabled when not a TTY so CI logs stay clean)
if [[ -t 1 ]]; then
  GREEN="\033[0;32m"; RED="\033[0;31m"; RESET="\033[0m"
else
  GREEN=""; RED=""; RESET=""
fi

pass() { echo -e "${GREEN}  PASS${RESET}  $1"; (( PASS++ )) || true; }
fail() { echo -e "${RED}  FAIL${RESET}  $1"; (( FAIL++ )) || true; }

# Run the script with a synthetic WASM file of a given byte size.
# Returns the combined stdout+stderr and the exit code via globals.
RUN_OUTPUT=""
RUN_EXIT=0
run_with_size() {
  local size_bytes="$1"
  local threshold_kb="${2:-200}"
  local tmp_wasm
  tmp_wasm=$(mktemp /tmp/test_wasm_XXXXXX.wasm)
  # Create a file of exactly `size_bytes` bytes
  dd if=/dev/zero bs=1 count="${size_bytes}" of="${tmp_wasm}" 2>/dev/null
  RUN_OUTPUT=$(WASM_FILE="${tmp_wasm}" WASM_SIZE_THRESHOLD_KB="${threshold_kb}" \
    bash "${SCRIPT}" 2>&1) || RUN_EXIT=$?
  RUN_EXIT=${RUN_EXIT:-0}
  rm -f "${tmp_wasm}"
}

# ── Tests ────────────────────────────────────────────────────────────────────

echo ""
echo "══════════════════════════════════════════════"
echo "  wasm_size_check.sh — Test Suite"
echo "══════════════════════════════════════════════"
echo ""

# 1. Missing WASM file → exit 1 + helpful message
echo "▶  1. Missing WASM file"
RUN_EXIT=0
RUN_OUTPUT=$(WASM_FILE="/tmp/nonexistent_predifi.wasm" bash "${SCRIPT}" 2>&1) || RUN_EXIT=$?
if [[ "${RUN_EXIT}" -eq 1 ]]; then
  pass "exits with code 1 when WASM file is missing"
else
  fail "expected exit 1, got ${RUN_EXIT}"
fi
if echo "${RUN_OUTPUT}" | grep -q "WASM file not found"; then
  pass "prints 'WASM file not found' message"
else
  fail "missing 'WASM file not found' in output"
fi
if echo "${RUN_OUTPUT}" | grep -q "cargo build"; then
  pass "suggests 'cargo build' command"
else
  fail "missing 'cargo build' suggestion"
fi

# 2. WASM under threshold → exit 0 + PASSED message
echo ""
echo "▶  2. WASM under threshold (100 KB, limit 200 KB)"
RUN_EXIT=0
run_with_size $(( 100 * 1024 )) 200 || true
if [[ "${RUN_EXIT}" -eq 0 ]]; then
  pass "exits with code 0 when under threshold"
else
  fail "expected exit 0, got ${RUN_EXIT}"
fi
if echo "${RUN_OUTPUT}" | grep -q "PASSED"; then
  pass "prints PASSED message"
else
  fail "missing PASSED in output"
fi
if echo "${RUN_OUTPUT}" | grep -q "100 KB"; then
  pass "reports current size (100 KB)"
else
  fail "current size not reported correctly"
fi
if echo "${RUN_OUTPUT}" | grep -q "Headroom"; then
  pass "reports headroom remaining"
else
  fail "missing headroom info"
fi
if echo "${RUN_OUTPUT}" | grep -q "Budget used"; then
  pass "reports budget percentage"
else
  fail "missing budget percentage"
fi

# 3. WASM exactly at threshold → exit 0 (boundary: equal is allowed)
echo ""
echo "▶  3. WASM exactly at threshold (200 KB, limit 200 KB)"
RUN_EXIT=0
run_with_size $(( 200 * 1024 )) 200 || true
if [[ "${RUN_EXIT}" -eq 0 ]]; then
  pass "exits with code 0 when size equals threshold"
else
  fail "expected exit 0 at boundary, got ${RUN_EXIT}"
fi

# 4. WASM over threshold → exit 1 + FAILED message + exceeded-by info
echo ""
echo "▶  4. WASM over threshold (250 KB, limit 200 KB)"
RUN_EXIT=0
run_with_size $(( 250 * 1024 )) 200 || true
if [[ "${RUN_EXIT}" -eq 1 ]]; then
  pass "exits with code 1 when over threshold"
else
  fail "expected exit 1, got ${RUN_EXIT}"
fi
if echo "${RUN_OUTPUT}" | grep -q "FAILED"; then
  pass "prints FAILED message"
else
  fail "missing FAILED in output"
fi
if echo "${RUN_OUTPUT}" | grep -q "Exceeded by"; then
  pass "reports how much the limit was exceeded"
else
  fail "missing 'Exceeded by' info"
fi
if echo "${RUN_OUTPUT}" | grep -q "250 KB"; then
  pass "reports current size (250 KB)"
else
  fail "current size not reported correctly"
fi
if echo "${RUN_OUTPUT}" | grep -q "200 KB"; then
  pass "reports the threshold (200 KB)"
else
  fail "threshold not reported correctly"
fi

# 5. Optimization suggestions present on failure
echo ""
echo "▶  5. Optimization suggestions on failure"
RUN_EXIT=0
run_with_size $(( 210 * 1024 )) 200 || true
if echo "${RUN_OUTPUT}" | grep -q "wasm-opt"; then
  pass "suggests wasm-opt"
else
  fail "missing wasm-opt suggestion"
fi
if echo "${RUN_OUTPUT}" | grep -q "stellar contract optimize"; then
  pass "suggests stellar contract optimize"
else
  fail "missing stellar CLI suggestion"
fi
if echo "${RUN_OUTPUT}" | grep -q "opt-level"; then
  pass "suggests Cargo.toml opt-level tweak"
else
  fail "missing Cargo.toml suggestion"
fi
if echo "${RUN_OUTPUT}" | grep -q "WASM_SIZE_THRESHOLD_KB"; then
  pass "explains how to raise the threshold"
else
  fail "missing threshold-raise instructions"
fi
if echo "${RUN_OUTPUT}" | grep -q "256"; then
  pass "mentions Soroban hard limit (256 KB)"
else
  fail "missing Soroban hard limit mention"
fi

# 6. Custom threshold via env var
echo ""
echo "▶  6. Custom threshold via WASM_SIZE_THRESHOLD_KB"
RUN_EXIT=0
run_with_size $(( 50 * 1024 )) 40 || true
if [[ "${RUN_EXIT}" -eq 1 ]]; then
  pass "respects custom threshold (50 KB > 40 KB limit → fail)"
else
  fail "expected exit 1 with custom threshold, got ${RUN_EXIT}"
fi
RUN_EXIT=0
run_with_size $(( 30 * 1024 )) 40 || true
if [[ "${RUN_EXIT}" -eq 0 ]]; then
  pass "respects custom threshold (30 KB < 40 KB limit → pass)"
else
  fail "expected exit 0 with custom threshold, got ${RUN_EXIT}"
fi

# 7. Report always shows file path, size, and threshold
echo ""
echo "▶  7. Report always contains file, size, and threshold"
RUN_EXIT=0
run_with_size $(( 80 * 1024 )) 200 || true
if echo "${RUN_OUTPUT}" | grep -q "File:"; then
  pass "report contains File label"
else
  fail "missing File label in report"
fi
if echo "${RUN_OUTPUT}" | grep -q "Size:"; then
  pass "report contains Size label"
else
  fail "missing Size label in report"
fi
if echo "${RUN_OUTPUT}" | grep -q "Threshold:"; then
  pass "report contains Threshold label"
else
  fail "missing Threshold label in report"
fi

# ── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════"
TOTAL=$(( PASS + FAIL ))
echo "  Results: ${PASS}/${TOTAL} tests passed"
echo "══════════════════════════════════════════════"
echo ""

if (( FAIL > 0 )); then
  echo -e "${RED}  ✗ ${FAIL} test(s) failed.${RESET}"
  exit 1
else
  echo -e "${GREEN}  ✓ All tests passed.${RESET}"
  exit 0
fi
