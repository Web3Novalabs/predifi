#!/usr/bin/env bash
# test_all.sh
# ---------------------------------------------------------------------------
# Runs the full test suite for every crate in the PrediFi workspace
# sequentially, then prints a consolidated pass/fail summary.
#
# Usage (run from the `contract/` workspace root):
#   bash scripts/test_all.sh
#
# Exit codes:
#   0 – all crates passed
#   1 – one or more crates failed
# ---------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Crates to test in order (relative to WORKSPACE_ROOT)
CRATES=(
  "contracts/predifi-errors"
  "contracts/access-control"
  "contracts/predifi-contract"
)

PASS=()
FAIL=()

echo "══════════════════════════════════════════════"
echo "  PrediFi – Test All Crates"
echo "  Workspace: ${WORKSPACE_ROOT}"
echo "══════════════════════════════════════════════"
echo ""

for crate in "${CRATES[@]}"; do
  crate_path="${WORKSPACE_ROOT}/${crate}"
  crate_name="$(basename "${crate}")"

  echo "──────────────────────────────────────────────"
  echo "  Testing: ${crate_name}"
  echo "──────────────────────────────────────────────"

  if cargo test --manifest-path "${crate_path}/Cargo.toml" 2>&1; then
    PASS+=("${crate_name}")
    echo "✅  ${crate_name} PASSED"
  else
    FAIL+=("${crate_name}")
    echo "❌  ${crate_name} FAILED"
  fi

  echo ""
done

# ── Summary ──────────────────────────────────────────────────────────────────
echo "══════════════════════════════════════════════"
echo "  Summary"
echo "══════════════════════════════════════════════"
echo "  Passed : ${#PASS[@]} / $(( ${#PASS[@]} + ${#FAIL[@]} ))"

for name in "${PASS[@]}"; do
  echo "    ✅  ${name}"
done

if [[ ${#FAIL[@]} -gt 0 ]]; then
  echo "  Failed : ${#FAIL[@]}"
  for name in "${FAIL[@]}"; do
    echo "    ❌  ${name}"
  done
  echo ""
  echo "❌  Some tests FAILED. See output above for details."
  echo "══════════════════════════════════════════════"
  exit 1
fi

echo ""
echo "✅  All crates passed."
echo "══════════════════════════════════════════════"
exit 0
