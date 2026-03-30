#!/usr/bin/env bash
# test_wasm_opt.sh
# ---------------------------------------------------------------------------
# Unit tests for the wasm-opt -O3 optimization step used in deploy.sh.
#
# Usage (run from the `contract/` workspace root):
#   bash scripts/tests/test_wasm_opt.sh
#
# Exit code: 0 if all tests pass, 1 if any test fails.
# ---------------------------------------------------------------------------

set -euo pipefail

# Resolve paths relative to this test file so the suite can be run from any
# working directory (e.g. repo root, contract/, or scripts/tests/).
TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTRACT_DIR="$(cd "${TESTS_DIR}/../.." && pwd)"
TMP_DIR="$(mktemp -d /tmp/test_wasm_opt_XXXXXX)"
trap 'rm -rf "${TMP_DIR}"' EXIT

PASS=0
FAIL=0
WASM_OPT_AVAILABLE=0

# ── Helpers ──────────────────────────────────────────────────────────────────

# Colour codes (disabled when not a TTY so CI logs stay clean)
if [[ -t 1 ]]; then
  GREEN="\033[0;32m"; RED="\033[0;31m"; YELLOW="\033[0;33m"; RESET="\033[0m"
else
  GREEN=""; RED=""; YELLOW=""; RESET=""
fi

pass() { echo -e "${GREEN}  PASS${RESET}  $1"; (( PASS++ )) || true; }
fail() { echo -e "${RED}  FAIL${RESET}  $1"; (( FAIL++ )) || true; }
skip() { echo -e "${YELLOW}  SKIP${RESET}  $1"; }

# Build the smallest valid WASM module (magic + version = 8 bytes).
make_minimal_wasm() {
  local path="$1"
  printf '\x00\x61\x73\x6d\x01\x00\x00\x00' > "$path"
}

# ── Test suite ────────────────────────────────────────────────────────────────

echo ""
echo "══════════════════════════════════════════════"
echo "  test_wasm_opt.sh — Test Suite"
echo "══════════════════════════════════════════════"
echo ""

# 1. wasm-opt is installed ────────────────────────────────────────────────────
echo "▶  1. wasm-opt is installed"
if command -v wasm-opt &> /dev/null; then
  pass "wasm-opt found in PATH ($(command -v wasm-opt))"
  WASM_OPT_AVAILABLE=1
else
  fail "wasm-opt not found — install binaryen: sudo apt-get install -y binaryen"
fi

# 2–7 require wasm-opt; skip gracefully if not present ───────────────────────
if [[ "${WASM_OPT_AVAILABLE}" -eq 0 ]]; then
  echo ""
  echo "  Tests 2–7 skipped: wasm-opt not available."
else

# 2. Rejects invalid (non-WASM) input ─────────────────────────────────────────
echo ""
echo "▶  2. Rejects non-WASM input"
INVALID="${TMP_DIR}/invalid.wasm"
printf 'this is not wasm' > "${INVALID}"
EXIT_CODE=0
wasm-opt -O3 --strip-debug -o "${TMP_DIR}/invalid_out.wasm" "${INVALID}" 2>/dev/null || EXIT_CODE=$?
if [[ "${EXIT_CODE}" -ne 0 ]]; then
  pass "exits non-zero on invalid WASM input"
else
  fail "expected non-zero exit for invalid input, got 0"
fi

# 3. Accepts a minimal valid WASM module ──────────────────────────────────────
echo ""
echo "▶  3. Accepts minimal valid WASM module"
MINIMAL="${TMP_DIR}/minimal.wasm"
OUT3="${TMP_DIR}/minimal_out.wasm"
make_minimal_wasm "${MINIMAL}"
EXIT_CODE=0
wasm-opt -O3 --strip-debug -o "${OUT3}" "${MINIMAL}" 2>/dev/null || EXIT_CODE=$?
if [[ "${EXIT_CODE}" -eq 0 ]]; then
  pass "exits 0 on valid minimal WASM input"
else
  fail "expected exit 0 for valid WASM, got ${EXIT_CODE}"
fi

# 4. Output has valid WASM magic bytes ────────────────────────────────────────
echo ""
echo "▶  4. Output has valid WASM magic bytes (\\x00\\x61\\x73\\x6d)"
if [[ -f "${OUT3}" ]]; then
  MAGIC=$(od -A n -t x1 -N 4 "${OUT3}" | tr -d ' \n')
  if [[ "${MAGIC}" == "00617 36d" || "${MAGIC}" == "0061736d" ]]; then
    pass "output starts with WASM magic bytes (00 61 73 6d)"
  else
    # Normalize any spacing/formatting differences from od
    MAGIC_NORM=$(od -A n -t x1 -N 4 "${OUT3}" | tr -d ' \n\t')
    if [[ "${MAGIC_NORM}" == "0061736d" ]]; then
      pass "output starts with WASM magic bytes (00 61 73 6d)"
    else
      fail "unexpected magic bytes: '${MAGIC_NORM}' (expected '0061736d')"
    fi
  fi
else
  fail "output file not found from test 3 — cannot check magic bytes"
fi

# 5. -o output path is respected (output written to specified path) ────────────
echo ""
echo "▶  5. -o output path is respected"
MINIMAL2="${TMP_DIR}/minimal2.wasm"
EXPLICIT_OUT="${TMP_DIR}/explicit_output.wasm"
make_minimal_wasm "${MINIMAL2}"
wasm-opt -O3 --strip-debug -o "${EXPLICIT_OUT}" "${MINIMAL2}" 2>/dev/null
if [[ -f "${EXPLICIT_OUT}" ]]; then
  pass "output file created at the path given to -o"
else
  fail "output file not found at ${EXPLICIT_OUT}"
fi
if [[ ! -f "${TMP_DIR}/minimal2_out.wasm" ]]; then
  pass "no stray output file created alongside input"
else
  fail "unexpected stray output file appeared"
fi

# 6. --strip-debug flag is accepted ───────────────────────────────────────────
echo ""
echo "▶  6. --strip-debug flag is accepted"
MINIMAL3="${TMP_DIR}/minimal3.wasm"
OUT6="${TMP_DIR}/minimal3_stripped.wasm"
make_minimal_wasm "${MINIMAL3}"
EXIT_CODE=0
wasm-opt -O3 --strip-debug -o "${OUT6}" "${MINIMAL3}" 2>/dev/null || EXIT_CODE=$?
if [[ "${EXIT_CODE}" -eq 0 ]]; then
  pass "--strip-debug accepted without error"
else
  fail "--strip-debug caused exit ${EXIT_CODE} — flag may not be supported by this wasm-opt version"
fi

# 7. Real artifact size check (integration; skip if build not present) ────────
echo ""
echo "▶  7. Optimized artifact is not larger than raw artifact"
REAL_WASM="${CONTRACT_DIR}/target/wasm32-unknown-unknown/release/predifi_contract.wasm"
if [[ ! -f "${REAL_WASM}" ]]; then
  skip "predifi_contract.wasm not found — run 'cargo build --target wasm32-unknown-unknown --release' first"
else
  COPY="${TMP_DIR}/predifi_contract_copy.wasm"
  OPT_OUT="${TMP_DIR}/predifi_contract_opt.wasm"
  cp "${REAL_WASM}" "${COPY}"
  RAW_BYTES=$(wc -c < "${COPY}")
  EXIT_CODE=0
  wasm-opt -O3 --strip-debug -o "${OPT_OUT}" "${COPY}" 2>/dev/null || EXIT_CODE=$?
  if [[ "${EXIT_CODE}" -ne 0 ]]; then
    fail "wasm-opt exited ${EXIT_CODE} on real predifi_contract.wasm"
  else
    OPT_BYTES=$(wc -c < "${OPT_OUT}")
    if (( OPT_BYTES <= RAW_BYTES )); then
      REDUCTION=$(( RAW_BYTES - OPT_BYTES ))
      pass "optimized (${OPT_BYTES} bytes) ≤ raw (${RAW_BYTES} bytes) — saved ${REDUCTION} bytes"
    else
      fail "optimized (${OPT_BYTES} bytes) is larger than raw (${RAW_BYTES} bytes)"
    fi
  fi
fi

fi  # end WASM_OPT_AVAILABLE guard

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
