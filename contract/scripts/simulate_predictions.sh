#!/bin/bash

# Predifi Prediction Simulator
# Simulates volume on a deployed contract and measures throughput.

set -e

# Default values
PREDICTIONS=${PREDICTIONS:-50}
NETWORK=${NETWORK:-sandbox}
IDENTITY=${IDENTITY:-admin}
TOKEN=${TOKEN:-}
AMOUNT=${AMOUNT:-1000}

echo "--- Predifi Simulator ---"
echo "Target: $NETWORK"
echo "Identity: $IDENTITY"
echo "Volume: $PREDICTIONS predictions"

# 1. Build contract
echo "Building contract..."
cargo build --target wasm32-unknown-unknown --release

# 2. Deploy (simulating if sandbox/standalone)
if [ "$NETWORK" == "sandbox" ]; then
    echo "Running simulation in sandbox mode (using cargo test)..."
    cargo test stress_test::test_prediction_throughput_measurement -- --nocapture
    exit 0
fi

# 3. For live network (Testnet/Mainnet)
if [ -z "$CONTRACT_ID" ]; then
    echo "CONTRACT_ID not set. Please deploy or set the variable."
    exit 1
fi

if [ -z "$TOKEN" ]; then
    echo "TOKEN address not set. Required for predictions."
    exit 1
fi

echo "Simulating $PREDICTIONS on pool 0..."
START_TIME=$(date +%s)

for i in $(seq 1 $PREDICTIONS); do
    echo "Prediction $i/$PREDICTIONS..."
    # Generate dummy addresses or use a pool of test accounts
    # This part requires a pool of funded accounts to be truly concurrent/high-volume
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$IDENTITY" \
        -- \
        place_prediction \
        --user "$IDENTITY" \
        --pool_id 0 \
        --amount "$AMOUNT" \
        --outcome 0
done

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "--- Metrics ---"
echo "Total Time: $DURATION seconds"
if [ $DURATION -gt 0 ]; then
    THROUGHPUT=$(echo "$PREDICTIONS / $DURATION" | bc -l)
    echo "Throughput: $THROUGHPUT predictions/second"
fi
echo "Simulation Complete."
