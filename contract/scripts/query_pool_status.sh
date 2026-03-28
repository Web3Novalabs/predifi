#!/usr/bin/env bash

# Query and print status/details for a specific PrediFi pool.
#
# Usage:
#   ./query_pool_status.sh <network> <predifi_contract_id> <pool_id> [source]
#
# Example:
#   ./query_pool_status.sh testnet CD... 7 default

set -euo pipefail

NETWORK="${1:-}"
CONTRACT_ID="${2:-}"
POOL_ID="${3:-}"
SOURCE="${4:-default}"

if [[ -z "${NETWORK}" || -z "${CONTRACT_ID}" || -z "${POOL_ID}" ]]; then
    echo "Error: missing required arguments."
    echo "Usage: $0 <network> <predifi_contract_id> <pool_id> [source]"
    exit 1
fi

if [[ -n "${CLI_BIN:-}" ]]; then
    CLI="${CLI_BIN}"
elif command -v stellar >/dev/null 2>&1; then
    CLI="stellar"
elif command -v soroban >/dev/null 2>&1; then
    CLI="soroban"
else
    echo "Error: neither 'stellar' nor 'soroban' CLI is available in PATH."
    exit 1
fi

echo "Using CLI: ${CLI}"
echo "Network: ${NETWORK}"
echo "Contract: ${CONTRACT_ID}"
echo "Pool ID: ${POOL_ID}"
echo "Source: ${SOURCE}"

POOL_DATA=$("${CLI}" contract invoke \
    --id "${CONTRACT_ID}" \
    --source "${SOURCE}" \
    --network "${NETWORK}" \
    -- \
    get_pool \
    --pool_id "${POOL_ID}")

POOL_STATS=$("${CLI}" contract invoke \
    --id "${CONTRACT_ID}" \
    --source "${SOURCE}" \
    --network "${NETWORK}" \
    -- \
    get_pool_stats \
    --pool_id "${POOL_ID}")

echo ""
echo "Pool details:"
echo "${POOL_DATA}"
echo ""
echo "Pool stats:"
echo "${POOL_STATS}"
