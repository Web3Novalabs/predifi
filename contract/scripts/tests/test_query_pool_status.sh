#!/usr/bin/env bash
# test_query_pool_status.sh
# ---------------------------------------------------------------------------
# Unit tests for query_pool_status.sh using a mocked CLI binary.
# ---------------------------------------------------------------------------

set -euo pipefail

TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$(cd "${TESTS_DIR}/.." && pwd)"
SCRIPT="${SCRIPTS_DIR}/query_pool_status.sh"
PASS=0
FAIL=0

if [[ -t 1 ]]; then
  GREEN="\033[0;32m"; RED="\033[0;31m"; RESET="\033[0m"
else
  GREEN=""; RED=""; RESET=""
fi

pass() { echo -e "${GREEN}  PASS${RESET}  $1"; (( PASS++ )) || true; }
fail() { echo -e "${RED}  FAIL${RESET}  $1"; (( FAIL++ )) || true; }

echo ""
echo "══════════════════════════════════════════════"
echo "  query_pool_status.sh — Test Suite"
echo "══════════════════════════════════════════════"
echo ""

echo "▶  1. Missing args should fail"
RUN_EXIT=0
RUN_OUTPUT=$(bash "${SCRIPT}" 2>&1) || RUN_EXIT=$?
if [[ "${RUN_EXIT}" -eq 1 ]]; then
  pass "exits with code 1 on missing args"
else
  fail "expected exit 1, got ${RUN_EXIT}"
fi
if echo "${RUN_OUTPUT}" | grep -q "Usage:"; then
  pass "prints usage help"
else
  fail "expected usage output"
fi

echo ""
echo "▶  2. Successful query with mocked CLI"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT
MOCK_CLI="${TMP_DIR}/stellar"
CALLS_LOG="${TMP_DIR}/calls.log"

cat > "${MOCK_CLI}" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

echo "$*" >> "${CALLS_LOG}"

if [[ "$*" == *" get_pool_stats "* ]]; then
  echo '{"resolved":false,"total_stake":"1000000"}'
else
  echo '{"id":7,"category":"Sports","resolved":false}'
fi
EOF

chmod +x "${MOCK_CLI}"
export CALLS_LOG

RUN_EXIT=0
RUN_OUTPUT=$(CLI_BIN="${MOCK_CLI}" bash "${SCRIPT}" testnet CTESTCONTRACT 7 default 2>&1) || RUN_EXIT=$?
if [[ "${RUN_EXIT}" -eq 0 ]]; then
  pass "exits with code 0 on success"
else
  fail "expected exit 0, got ${RUN_EXIT}"
fi
if echo "${RUN_OUTPUT}" | grep -q "Pool details:"; then
  pass "prints pool details header"
else
  fail "missing pool details header"
fi
if echo "${RUN_OUTPUT}" | grep -q "Pool stats:"; then
  pass "prints pool stats header"
else
  fail "missing pool stats header"
fi
if grep -q "get_pool --pool_id 7" "${CALLS_LOG}"; then
  pass "invokes get_pool with pool id"
else
  fail "did not call get_pool correctly"
fi
if grep -q "get_pool_stats --pool_id 7" "${CALLS_LOG}"; then
  pass "invokes get_pool_stats with pool id"
else
  fail "did not call get_pool_stats correctly"
fi

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
