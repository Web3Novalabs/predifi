# PrediFi Smart Contract Deployment Guide

## Table of Contents
1. [Prerequisites](#prerequisites)
2. [Wallet Setup](#wallet-setup)
3. [Environment Configuration](#environment-configuration)
4. [Contract Compilation](#contract-compilation)
5. [Testnet Deployment](#testnet-deployment)
6. [Mainnet Deployment](#mainnet-deployment)
7. [Initialization Parameters](#initialization-parameters)
8. [Token Whitelisting](#token-whitelisting)
9. [Oracle Setup](#oracle-setup)
10. [Role Management](#role-management)
11. [Post-Deployment Verification](#post-deployment-verification)
12. [Troubleshooting](#troubleshooting)
13. [Security Checklist](#security-checklist)

---

## Prerequisites

### Required Software
- **Rust** (latest stable): `rustup update`
- **Stellar CLI**: `cargo install stellar-cli`
- **Binaryen** (for WASM optimization): `cargo install wasm-opt` or system package manager
- **Git**: For cloning the repository

### Rust Targets
```bash
rustup target add wasm32-unknown-unknown
```

### Verify Installations
```bash
rustc --version
stellar --version
wasm-opt --version
```

---

## Wallet Setup

### 1. Generate Deployment Wallet
```bash
# For testnet
stellar keys generate --network testnet deployer

# For mainnet (BE CAREFUL!)
stellar keys generate --network mainnet deployer
```

### 2. Fund Your Wallet

#### Testnet Funding
```bash
# Get test XLM from friendbot
stellar account fund deployer --network testnet
```

#### Mainnet Funding
1. Purchase XLM from an exchange
2. Send to your wallet address (get with `stellar keys address deployer --network mainnet`)
3. Recommended minimum: 100 XLM for deployment

### 3. Backup Your Keys
```bash
# Export and SECURELY store your secret key
stellar keys show deployer --network testnet --secret
```

**CRITICAL**: Never commit secret keys to version control or share them.

---

## Environment Configuration

### Configure Networks
```bash
# Testnet configuration
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"

# Mainnet configuration  
stellar network add mainnet \
  --rpc-url https://soroban.stellar.org:443 \
  --network-passphrase "Public Global Stellar Network ; September 2015"
```

### Verify Configuration
```bash
stellar network list
stellar network use testnet  # or mainnet
```

---

## Contract Compilation

### 1. Clone Repository
```bash
git clone <repository-url>
cd predifi
```

### 2. Build Contracts
```bash
cd contract

# Option A: Use build script (recommended)
./build.sh

# Option B: Manual build
cargo build --target wasm32-unknown-unknown --release
```

### 3. Optimize WASM Files
```bash
# Optimize access control contract
wasm-opt -Oz --enable-bulk-memory \
  target/wasm32-unknown-unknown/release/access_control.wasm \
  -o target/wasm32-unknown-unknown/release/access_control_optimized.wasm

# Optimize predifi contract
wasm-opt -Oz --enable-bulk-memory \
  target/wasm32-unknown-unknown/release/predifi_contract.wasm \
  -o target/wasm32-unknown-unknown/release/predifi_contract_optimized.wasm

# Additional Stellar optimization
stellar contract optimize \
  --wasm target/wasm32-unknown-unknown/release/access_control_optimized.wasm
  
stellar contract optimize \
  --wasm target/wasm32-unknown-unknown/release/predifi_contract_optimized.wasm
```

### 4. Verify Build Output
```bash
ls -lh target/wasm32-unknown-unknown/release/*_optimized.wasm
```
Expected sizes: 50-400KB each.

---

## Testnet Deployment

### Automated Deployment (Fixed Script)
First, let's fix the deployment script to include all required parameters:

```bash
# Update the deployment script or use this corrected manual approach
```

### Manual Deployment (Step-by-Step)

#### Step 1: Deploy AccessControl Contract
```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/access_control_optimized.wasm \
  --source deployer \
  --network testnet
```
Save the returned contract ID as `ACCESS_CONTROL_ID`.

#### Step 2: Initialize AccessControl
```bash
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  init \
  --admin $(stellar keys address deployer --network testnet)
```

#### Step 3: Deploy PrediFi Contract
```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predifi_contract_optimized.wasm \
  --source deployer \
  --network testnet
```
Save the returned contract ID as `PREDIFI_CONTRACT_ID`.

#### Step 4: Initialize PrediFi Contract (CORRECTED)
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  init \
  --access_control <ACCESS_CONTROL_ID> \
  --treasury $(stellar keys address deployer --network testnet) \
  --fee_bps 100 \
  --resolution_delay 3600 \
  --min_pool_duration 3600 \
  --max_predictions_per_user 10
```

### Fixed Deployment Script
Create a corrected deployment script:

```bash
#!/bin/bash
# corrected_deploy.sh
set -e

NETWORK=$1
SOURCE=$2

# Configuration
ADMIN_ADDRESS=$(stellar keys address "$SOURCE" --network "$NETWORK")
TREASURY_ADDRESS=${TREASURY_ADDRESS:-$ADMIN_ADDRESS}
FEE_BPS=${FEE_BPS:-100}
RESOLUTION_DELAY=${RESOLUTION_DELAY:-3600}
MIN_POOL_DURATION=${MIN_POOL_DURATION:-3600}
MAX_PREDICTIONS_PER_USER=${MAX_PREDICTIONS_PER_USER:-10}

echo "Deploying to $NETWORK with source $SOURCE"
echo "Admin: $ADMIN_ADDRESS"
echo "Treasury: $TREASURY_ADDRESS"
echo "Fee: ${FEE_BPS}bps"
echo "Resolution Delay: ${RESOLUTION_DELAY}s"
echo "Min Pool Duration: ${MIN_POOL_DURATION}s"
echo "Max Predictions/User: ${MAX_PREDICTIONS_PER_USER}"

# Deploy AccessControl
AC_ID=$(stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/access_control_optimized.wasm \
  --source "$SOURCE" \
  --network "$NETWORK")
echo "AccessControl ID: $AC_ID"

# Initialize AccessControl
stellar contract invoke \
  --id "$AC_ID" \
  --source "$SOURCE" \
  --network "$NETWORK" \
  -- \
  init \
  --admin "$ADMIN_ADDRESS"

# Deploy PrediFi Contract
PD_ID=$(stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predifi_contract_optimized.wasm \
  --source "$SOURCE" \
  --network "$NETWORK")
echo "PrediFi Contract ID: $PD_ID"

# Initialize PrediFi Contract with ALL required parameters
stellar contract invoke \
  --id "$PD_ID" \
  --source "$SOURCE" \
  --network "$NETWORK" \
  -- \
  init \
  --access_control "$AC_ID" \
  --treasury "$TREASURY_ADDRESS" \
  --fee_bps "$FEE_BPS" \
  --resolution_delay "$RESOLUTION_DELAY" \
  --min_pool_duration "$MIN_POOL_DURATION" \
  --max_predictions_per_user "$MAX_PREDICTIONS_PER_USER"

# Save deployment info
cat <<EOF > "deployed_contracts_${NETWORK}_corrected.json"
{
  "network": "$NETWORK",
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "admin_address": "$ADMIN_ADDRESS",
  "contracts": {
    "access_control": {
      "id": "$AC_ID",
      "admin": "$ADMIN_ADDRESS"
    },
    "predifi_contract": {
      "id": "$PD_ID",
      "treasury": "$TREASURY_ADDRESS",
      "fee_bps": $FEE_BPS,
      "resolution_delay": $RESOLUTION_DELAY,
      "min_pool_duration": $MIN_POOL_DURATION,
      "max_predictions_per_user": $MAX_PREDICTIONS_PER_USER
    }
  }
}
EOF

echo "Deployment complete! Details saved to deployed_contracts_${NETWORK}_corrected.json"
```

---

## Mainnet Deployment

### Pre-Deployment Checklist
- [ ] Test thoroughly on testnet
- [ ] Review all contract code
- [ ] Ensure wallet has 100+ XLM
- [ ] Decide on protocol parameters
- [ ] Prepare operator addresses
- [ ] Have oracle addresses ready
- [ ] Test deployment script on testnet

### Deployment Commands

```bash
# Using corrected script with custom parameters
TREASURY_ADDRESS=GB... \
FEE_BPS=200 \
RESOLUTION_DELAY=7200 \
MIN_POOL_DURATION=7200 \
MAX_PREDICTIONS_PER_USER=5 \
./corrected_deploy.sh mainnet deployer
```

### Manual Mainnet Deployment
Same steps as testnet, but use `--network mainnet` and be extra careful with parameters:

```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network mainnet \
  -- \
  init \
  --access_control <ACCESS_CONTROL_ID> \
  --treasury <SECURE_TREASURY_ADDRESS> \
  --fee_bps 200 \
  --resolution_delay 7200 \
  --min_pool_duration 7200 \
  --max_predictions_per_user 5
```

---

## Initialization Parameters

### AccessControl Initialization
| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `admin` | Address | Initial administrator | `GD...` |

### PrediFi Contract Initialization
| Parameter | Type | Description | Default | Recommended |
|-----------|------|-------------|---------|-------------|
| `access_control` | Address | Access control contract | Required | - |
| `treasury` | Address | Receives protocol fees | Deployer | Secure wallet |
| `fee_bps` | u32 | Protocol fee (100=1%) | 100 | 100-500 |
| `resolution_delay` | u64 | Delay after pool end (seconds) | 3600 | 300-86400 |
| `min_pool_duration` | u64 | Minimum pool duration (seconds) | 3600 | 3600-86400 |
| `max_predictions_per_user` | u32 | Max predictions per user per pool | 10 | 1-100 |

### Parameter Recommendations

#### Production (Mainnet)
```bash
--fee_bps 200                 # 2% protocol fee
--resolution_delay 7200       # 2 hour resolution delay
--min_pool_duration 14400     # 4 hour minimum pool duration
--max_predictions_per_user 5  # Limit predictions per user
```

#### Testing (Testnet)
```bash
--fee_bps 100                 # 1% fee for testing
--resolution_delay 300        # 5 minute delay for quick testing
--min_pool_duration 600       # 10 minute pools for testing
--max_predictions_per_user 10 # Allow more predictions for testing
```

---

## Token Whitelisting

### Add Token to Whitelist
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_token_to_whitelist \
  --admin $(stellar keys address deployer --network testnet) \
  --token <TOKEN_CONTRACT_ADDRESS>
```

### Common Tokens

#### XLM (Native)
```bash
# XLM uses the native asset identifier
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_token_to_whitelist \
  --admin <ADMIN_ADDRESS> \
  --token <NATIVE_ASSET_ADDRESS>
```

#### Custom Tokens (USDC, etc.)
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_token_to_whitelist \
  --admin <ADMIN_ADDRESS> \
  --token <TOKEN_CONTRACT_ADDRESS>
```

### Verify Whitelist Status
```bash
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  is_token_whitelisted \
  --token <TOKEN_ADDRESS>
```

---

## Oracle Setup

### 1. Initialize Oracle Configuration
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  init_oracle \
  --admin <ADMIN_ADDRESS> \
  --pyth_contract <PYTH_CONTRACT_ADDRESS> \
  --max_price_age 60 \
  --min_confidence_ratio 9500
```

### 2. Add Oracle Addresses
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_oracle \
  --admin <ADMIN_ADDRESS> \
  --oracle_address <ORACLE_ADDRESS>
```

### 3. Set Price Condition for Pool
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source operator \
  --network testnet \
  -- \
  set_price_condition \
  --operator <OPERATOR_ADDRESS> \
  --pool_id <POOL_ID> \
  --feed_pair ETHUSD \
  --target_price 3500000000 \
  --operator_type 1 \
  --tolerance_bps 100
```

### Pyth Network Addresses
- **Testnet**: Check current Pyth documentation
- **Mainnet**: Check current Pyth documentation

---

## Role Management

### Role Hierarchy
| Role | Value | Permissions |
|------|-------|-------------|
| Admin | 0 | Full control: pause, fees, treasury, whitelisting |
| Operator | 1 | Resolve pools, cancel pools, set limits |
| Oracle | 3 | Update price feeds |
| User | 4 | Basic participant |

### Assign Operator Role
```bash
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  assign_role \
  --admin_caller <ADMIN_ADDRESS> \
  --user <OPERATOR_ADDRESS> \
  --role Operator
```

### Assign Oracle Role
```bash
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  assign_role \
  --admin_caller <ADMIN_ADDRESS> \
  --user <ORACLE_ADDRESS> \
  --role Oracle
```

### Check Roles
```bash
stellar contract read \
  --id <ACCESS_CONTROL_ID> \
  --network testnet \
  -- \
  has_role \
  --user <USER_ADDRESS> \
  --role Operator
```

---

## Post-Deployment Verification

### 1. Verify Contract Initialization
```bash
# Check treasury address
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  get_treasury

# Check fee configuration
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  get_fee_bps

# Check resolution delay
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  get_resolution_delay
```

### 2. Test Pool Creation
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  create_pool \
  --creator $(stellar keys address deployer --network testnet) \
  --end_time $(($(date +%s) + 3600)) \
  --category Sports \
  --title "Deployment Test Pool" \
  --description "Testing deployment" \
  --metadata_url "ipfs://test" \
  --options 2 \
  --config '{"min_stake": 1, "max_stake": 1000, "max_total_stake": 10000}'
```

### 3. Verify Token Whitelisting
```bash
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  is_token_whitelisted \
  --token <WHITELISTED_TOKEN_ADDRESS>
```

### 4. Check Role Assignments
```bash
stellar contract read \
  --id <ACCESS_CONTROL_ID> \
  --network testnet \
  -- \
  get_operator_count
```

### 5. Monitor Contract Events
```bash
stellar contract events \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  --from <LEDGER_SEQUENCE>
```

---

## Troubleshooting

### Common Errors

#### 1. Insufficient Balance
**Error**: `Error(Contract, #44)` - Insufficient balance
**Solution**: Fund your wallet
```bash
stellar account balance deployer --network testnet
stellar account fund deployer --network testnet
```

#### 2. Unauthorized Access
**Error**: `Error(Contract, #10)` - Unauthorized
**Solution**: Check role permissions
```bash
stellar contract read \
  --id <ACCESS_CONTROL_ID> \
  --network testnet \
  -- \
  has_role \
  --user <YOUR_ADDRESS> \
  --role Admin
```

#### 3. Invalid Parameters
**Error**: `Error(Contract, #1)` - Invalid data
**Solution**: Check parameter ranges and types

#### 4. Contract Not Initialized
**Error**: `Error(Contract, #1)` - NotInitialized
**Solution**: Ensure `init` was called on both contracts

### Debug Mode
```bash
# Build with debug symbols
cargo build --profile release-with-logs --target wasm32-unknown-unknown
```

### Network Issues
```bash
# Check RPC connection
curl -s https://soroban-testnet.stellar.org:443

# Try different RPC endpoint if available
stellar network update testnet --rpc-url <ALTERNATE_RPC_URL>
```

---

## Security Checklist

### Pre-Deployment
- [ ] Code review completed
- [ ] Security audit performed (if handling significant value)
- [ ] Tested on testnet with real transactions
- [ ] Parameters validated (fees, delays, limits)
- [ ] Wallet security verified (hardware wallet recommended)
- [ ] Backup procedures documented
- [ ] Emergency procedures documented

### Deployment
- [ ] Use corrected deployment script or manual commands
- [ ] Verify all initialization parameters
- [ ] Save contract IDs securely
- [ ] Test all contract functions
- [ ] Verify token whitelisting works
- [ ] Test oracle integration

### Post-Deployment
- [ ] Monitor contract activity
- [ ] Set up alerts for unusual activity
- [ ] Regular security reviews scheduled
- [ ] Key rotation plan documented
- [ ] Upgrade procedures tested

### Key Security Practices
1. **Never commit secret keys** to version control
2. **Use hardware wallets** for admin accounts
3. **Implement multi-sig** for critical operations
4. **Regularly audit** role assignments
5. **Monitor** for unusual activity
6. **Have emergency pause** ready

---

## Quick Reference

### Deployment Commands
```bash
# Testnet
./corrected_deploy.sh testnet deployer

# Mainnet with custom parameters
TREASURY_ADDRESS=GB... FEE_BPS=200 ./corrected_deploy.sh mainnet deployer
```

### Common Operations
```bash
# Whitelist token
stellar contract invoke --id <PD_ID> --source deployer --network testnet -- add_token_to_whitelist --admin <ADMIN> --token <TOKEN>

# Assign operator
stellar contract invoke --id <AC_ID> --source deployer --network testnet -- assign_role --admin_caller <ADMIN> --user <OPERATOR> --role Operator

# Initialize oracle
stellar contract invoke --id <PD_ID> --source deployer --network testnet -- init_oracle --admin <ADMIN> --pyth_contract <PYTH> --max_price_age 60 --min_confidence_ratio 9500
```

### Network URLs
- **Testnet RPC**: `https://soroban-testnet.stellar.org:443`
- **Mainnet RPC**: `https://soroban.stellar.org:443`
- **Testnet Passphrase**: `Test SDF Network ; September 2015`
- **Mainnet Passphrase**: `Public Global Stellar Network ; September 2015`

---

## Support Resources

1. **Stellar Documentation**: https://developers.stellar.org/docs/soroban
2. **Stellar Discord**: https://discord.gg/stellar
3. **Pyth Network**: https://docs.pyth.network/
4. **Project Repository**: Check issue tracker for known issues
5. **Security Audits**: Review security documentation in the project

---

**Last Updated**: 2026-08-28  
**Version**: 2.0.0 (Corrected for actual contract parameters)
**Note**: This guide corrects discrepancies between the contract code, deployment script, and existing documentation.
