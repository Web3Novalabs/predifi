#!/bin/bash

# ==============================================================================
# PrediFi Example Interaction Script
# ==============================================================================
# Demonstrates how to call init, create_pool, and place_prediction using the
# Soroban CLI. Adapt the contract IDs and addresses to your deployment.
#
# Prerequisites:
#   - stellar CLI installed (https://developers.stellar.org/docs/tools/cli)
#   - A funded testnet account (use `stellar keys generate` or Friendbot)
#   - Contract deployed (see deploy.sh)
#
# Usage: ./example_interaction.sh
# ==============================================================================

set -euo pipefail

# ── Configuration (replace with your deployed values) ─────────────────────────

NETWORK="testnet"
SOURCE="default"                                      # Stellar CLI identity name

# Contract IDs (replace after deployment)
PREDIFI_CONTRACT="CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
ACCESS_CONTROL_CONTRACT="CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
TOKEN_CONTRACT="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2QLGM5A"  # Testnet XLM SAC

# Addresses (replace with your keys)
TREASURY="GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
OPERATOR="GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
CREATOR="GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"

echo "=============================================="
echo "  PrediFi Example Interaction"
echo "=============================================="

# ── Step 1: Initialize the contract ───────────────────────────────────────────
#
# Sets the access control contract, treasury address, fee (in basis points),
# and the resolution delay (seconds after pool end before resolution is allowed).
# This is idempotent -- calling it again has no effect.

echo ""
echo "Step 1: Initialize contract..."
echo "------------------------------"

stellar contract invoke \
  --network "$NETWORK" \
  --source "$SOURCE" \
  --id "$PREDIFI_CONTRACT" \
  -- \
  init \
  --access_control "$ACCESS_CONTROL_CONTRACT" \
  --treasury "$TREASURY" \
  --fee_bps 200 \
  --resolution_delay 3600

echo "Contract initialized (fee: 2%, resolution delay: 1 hour)"

# ── Step 2: Whitelist a token ─────────────────────────────────────────────────
#
# Only whitelisted tokens can be used for pool stakes.
# Requires Admin role (role 0) in the access control contract.

echo ""
echo "Step 2: Whitelist token for staking..."
echo "--------------------------------------"

stellar contract invoke \
  --network "$NETWORK" \
  --source "$SOURCE" \
  --id "$PREDIFI_CONTRACT" \
  -- \
  add_token_to_whitelist \
  --admin "$OPERATOR" \
  --token "$TOKEN_CONTRACT"

echo "Token whitelisted: $TOKEN_CONTRACT"

# ── Step 3: Create a prediction pool ─────────────────────────────────────────
#
# Creates a new binary (2-outcome) prediction market.
# end_time is set to 24 hours from now (Unix timestamp).
# Returns the new pool ID.

echo ""
echo "Step 3: Create prediction pool..."
echo "----------------------------------"

END_TIME=$(($(date +%s) + 86400))  # 24 hours from now

POOL_ID=$(stellar contract invoke \
  --network "$NETWORK" \
  --source "$SOURCE" \
  --id "$PREDIFI_CONTRACT" \
  -- \
  create_pool \
  --creator "$CREATOR" \
  --end_time "$END_TIME" \
  --token "$TOKEN_CONTRACT" \
  --options_count 2 \
  --category "Sports" \
  --config "{\"description\":\"Will Team A win?\",\"metadata_url\":\"https://example.com/match\",\"min_stake\":1000000,\"max_stake\":0,\"max_total_stake\":0,\"initial_liquidity\":0,\"required_resolutions\":1,\"private\":false,\"whitelist_key\":null}")

echo "Pool created with ID: $POOL_ID"
echo "  End time: $(date -r "$END_TIME" 2>/dev/null || date -d "@$END_TIME" 2>/dev/null || echo "$END_TIME")"
echo "  Options: 2 (binary: Yes/No)"
echo "  Min stake: 1,000,000 stroops (1 XLM)"

# ── Step 4: Place a prediction ────────────────────────────────────────────────
#
# Stake 5 XLM on outcome 0 (e.g., "Yes").
# The user must have enough token balance and have approved the contract.

echo ""
echo "Step 4: Place prediction..."
echo "---------------------------"

stellar contract invoke \
  --network "$NETWORK" \
  --source "$SOURCE" \
  --id "$PREDIFI_CONTRACT" \
  -- \
  place_prediction \
  --user "$CREATOR" \
  --pool_id "$POOL_ID" \
  --amount 5000000 \
  --outcome 0

echo "Prediction placed: 5 XLM on outcome 0 (Yes)"

# ── Step 5: Check outcome stakes ─────────────────────────────────────────────

echo ""
echo "Step 5: Check outcome stakes..."
echo "--------------------------------"

STAKE_0=$(stellar contract invoke \
  --network "$NETWORK" \
  --source "$SOURCE" \
  --id "$PREDIFI_CONTRACT" \
  -- \
  get_outcome_stake \
  --pool_id "$POOL_ID" \
  --outcome 0)

STAKE_1=$(stellar contract invoke \
  --network "$NETWORK" \
  --source "$SOURCE" \
  --id "$PREDIFI_CONTRACT" \
  -- \
  get_outcome_stake \
  --pool_id "$POOL_ID" \
  --outcome 1)

echo "  Outcome 0 (Yes): $STAKE_0 stroops"
echo "  Outcome 1 (No):  $STAKE_1 stroops"

# ── Step 6: Get pool details ─────────────────────────────────────────────────

echo ""
echo "Step 6: Get pool details..."
echo "----------------------------"

stellar contract invoke \
  --network "$NETWORK" \
  --source "$SOURCE" \
  --id "$PREDIFI_CONTRACT" \
  -- \
  get_pool \
  --pool_id "$POOL_ID"

echo ""
echo "=============================================="
echo "  Example interaction complete!"
echo "=============================================="
echo ""
echo "Next steps:"
echo "  - Wait for pool end_time to pass"
echo "  - Resolve pool: invoke resolve_pool --operator <addr> --pool_id $POOL_ID --outcome 0"
echo "  - Claim winnings: invoke claim_winnings --user <addr> --pool_id $POOL_ID"
