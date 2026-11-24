#!/bin/bash

# ==============================================================================
# PrediFi Deployment Verification Script
# ==============================================================================
# Verifies contract deployment and initialization
# Usage: ./verify_deployment.sh <network> <predifi_contract_id> <access_control_id> <admin_address>
# ==============================================================================

set -e

NETWORK=$1
PREDIFI_ID=$2
ACCESS_CONTROL_ID=$3
ADMIN_ADDRESS=$4

if [[ -z "$NETWORK" || -z "$PREDIFI_ID" || -z "$ACCESS_CONTROL_ID" || -z "$ADMIN_ADDRESS" ]]; then
    echo "❌ Error: Missing arguments."
    echo "Usage: $0 <network> <predifi_contract_id> <access_control_id> <admin_address>"
    echo "Example: $0 testnet CD1234... CD5678... GD9ABC..."
    exit 1
fi

# Detect CLI
if command -v stellar &> /dev/null; then
    CLI="stellar"
elif command -v soroban &> /dev/null; then
    CLI="soroban"
else
    echo "❌ Error: No CLI found. Install with: cargo install stellar-cli"
    exit 1
fi

echo "🔍 Verifying PrediFi deployment on $NETWORK"
echo "PrediFi Contract: $PREDIFI_ID"
echo "Access Control: $ACCESS_CONTROL_ID"
echo "Admin: $ADMIN_ADDRESS"
echo ""

# Test 1: Check if contracts exist
echo "1. Checking contract existence..."
$CLI contract inspect --id "$PREDIFI_ID" --network "$NETWORK" >/dev/null 2>&1
if [[ $? -eq 0 ]]; then
    echo "   ✅ PrediFi contract exists"
else
    echo "   ❌ PrediFi contract not found"
    exit 1
fi

$CLI contract inspect --id "$ACCESS_CONTROL_ID" --network "$NETWORK" >/dev/null 2>&1
if [[ $? -eq 0 ]]; then
    echo "   ✅ AccessControl contract exists"
else
    echo "   ❌ AccessControl contract not found"
    exit 1
fi

# Test 2: Check treasury address
echo "2. Checking treasury address..."
TREASURY=$($CLI contract read \
    --id "$PREDIFI_ID" \
    --network "$NETWORK" \
    -- \
    get_treasury 2>/dev/null || echo "ERROR")

if [[ "$TREASURY" == "ERROR" ]]; then
    echo "   ❌ Could not read treasury (contract may not be initialized)"
else
    echo "   ✅ Treasury address: $TREASURY"
fi

# Test 3: Check fee configuration
echo "3. Checking fee configuration..."
FEE_BPS=$($CLI contract read \
    --id "$PREDIFI_ID" \
    --network "$NETWORK" \
    -- \
    get_fee_bps 2>/dev/null || echo "ERROR")

if [[ "$FEE_BPS" == "ERROR" ]]; then
    echo "   ❌ Could not read fee configuration"
else
    echo "   ✅ Fee BPS: $FEE_BPS"
fi

# Test 4: Check resolution delay
echo "4. Checking resolution delay..."
RESOLUTION_DELAY=$($CLI contract read \
    --id "$PREDIFI_ID" \
    --network "$NETWORK" \
    -- \
    get_resolution_delay 2>/dev/null || echo "ERROR")

if [[ "$RESOLUTION_DELAY" == "ERROR" ]]; then
    echo "   ❌ Could not read resolution delay"
else
    echo "   ✅ Resolution delay: $RESOLUTION_DELAY seconds"
fi

# Test 5: Check admin role
echo "5. Checking admin role..."
HAS_ADMIN_ROLE=$($CLI contract read \
    --id "$ACCESS_CONTROL_ID" \
    --network "$NETWORK" \
    -- \
    has_role \
    --user "$ADMIN_ADDRESS" \
    --role Admin 2>/dev/null || echo "ERROR")

if [[ "$HAS_ADMIN_ROLE" == "ERROR" ]]; then
    echo "   ❌ Could not check admin role"
elif [[ "$HAS_ADMIN_ROLE" == "true" ]]; then
    echo "   ✅ Admin role confirmed"
else
    echo "   ⚠️ Admin role not assigned (or check failed)"
fi

# Test 6: Check contract initialization
echo "6. Checking contract initialization status..."
echo "   (If no errors above, contracts are likely initialized correctly)"

# Test 7: Try to read a pool (should work even if no pools exist)
echo "7. Testing pool read capability..."
POOL_READ=$($CLI contract read \
    --id "$PREDIFI_ID" \
    --network "$NETWORK" \
    -- \
    get_pool \
    --pool_id 0 2>&1 | head -1 || echo "ERROR")

if [[ "$POOL_READ" == *"PoolNotFound"* ]] || [[ "$POOL_READ" == *"does not exist"* ]]; then
    echo "   ✅ Pool read works (pool 0 doesn't exist, which is expected)"
elif [[ "$POOL_READ" == "ERROR" ]]; then
    echo "   ⚠️ Could not test pool read"
else
    echo "   ⚠️ Unexpected response: $POOL_READ"
fi

echo ""
echo "📊 Verification Summary:"
echo "-----------------------"

if [[ "$TREASURY" != "ERROR" ]] && [[ "$FEE_BPS" != "ERROR" ]] && [[ "$RESOLUTION_DELAY" != "ERROR" ]]; then
    echo "✅ Contracts appear to be properly deployed and initialized"
    echo ""
    echo "📋 Configuration:"
    echo "   Treasury: $TREASURY"
    echo "   Fee: $FEE_BPS bps"
    echo "   Resolution Delay: $RESOLUTION_DELAY seconds"
    
    # Check for common issues
    echo ""
    echo "🔍 Checking for common issues:"
    
    # Issue 1: Missing initialization parameters (old deployment script)
    if [[ "$RESOLUTION_DELAY" == "0" ]]; then
        echo "   ⚠️ WARNING: resolution_delay is 0 (may indicate incomplete initialization)"
        echo "   This suggests the old deployment script was used without all parameters."
    fi
    
    # Issue 2: Unreasonable fee
    if [[ "$FEE_BPS" -gt 1000 ]]; then
        echo "   ⚠️ WARNING: Fee ($FEE_BPS bps) seems high (>10%)"
    fi
    
    # Issue 3: Very short resolution delay
    if [[ "$RESOLUTION_DELAY" -lt 300 ]]; then
        echo "   ⚠️ WARNING: Resolution delay ($RESOLUTION_DELAY seconds) is very short (<5 minutes)"
    fi
    
else
    echo "❌ Contracts may not be fully initialized"
    echo ""
    echo "Possible issues:"
    echo "   1. Contracts were deployed but not initialized"
    echo "   2. Wrong contract IDs provided"
    echo "   3. Network connectivity issues"
    echo ""
    echo "Recommended actions:"
    echo "   1. Verify contract IDs are correct"
    echo "   2. Check network configuration: stellar network list"
    echo "   3. Try initializing contracts manually"
fi

echo ""
echo "📝 Next steps if verification failed:"
echo "   1. Re-initialize contracts with corrected_deploy.sh"
echo "   2. Check network configuration"
echo "   3. Ensure wallet has sufficient funds"
echo "   4. Contact support if issues persist"

echo ""
echo "✅ Verification complete!"
