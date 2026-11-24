# PrediFi Deployment Quick Reference

## One-Line Deployment Commands

### Testnet Deployment
```bash
# Basic testnet deployment
cd contract && ./corrected_deploy.sh testnet default

# Testnet with custom treasury
TREASURY_ADDRESS=GB... ./corrected_deploy.sh testnet default

# Testnet with 2% fee and 2-hour resolution
FEE_BPS=200 RESOLUTION_DELAY=7200 ./corrected_deploy.sh testnet default
```

### Mainnet Deployment
```bash
# Mainnet with secure treasury
TREASURY_ADDRESS=GB...SECURE... ./corrected_deploy.sh mainnet deployer

# Mainnet with production settings
TREASURY_ADDRESS=GB... \
FEE_BPS=200 \
RESOLUTION_DELAY=7200 \
MIN_POOL_DURATION=14400 \
MAX_PREDICTIONS_PER_USER=5 \
./corrected_deploy.sh mainnet deployer
```

## Essential Post-Deployment Commands

### 1. Whitelist Tokens
```bash
# XLM (native asset - check actual address)
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_token_to_whitelist \
  --admin <ADMIN_ADDRESS> \
  --token <XLM_ASSET_ADDRESS>

# USDC or other tokens
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_token_to_whitelist \
  --admin <ADMIN_ADDRESS> \
  --token <TOKEN_CONTRACT_ADDRESS>
```

### 2. Assign Roles
```bash
# Assign Operator role
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  assign_role \
  --admin_caller <ADMIN_ADDRESS> \
  --user <OPERATOR_ADDRESS> \
  --role Operator

# Assign Oracle role
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

### 3. Initialize Oracle
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

## Verification Commands

### Check Contract State
```bash
# Get treasury address
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  get_treasury

# Get fee configuration
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  get_fee_bps

# Check if token is whitelisted
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  is_token_whitelisted \
  --token <TOKEN_ADDRESS>

# Check user roles
stellar contract read \
  --id <ACCESS_CONTROL_ID> \
  --network testnet \
  -- \
  has_role \
  --user <USER_ADDRESS> \
  --role Operator
```

### Test Pool Creation
```bash
# Create a test pool
END_TIME=$(($(date +%s) + 3600))
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  create_pool \
  --creator <CREATOR_ADDRESS> \
  --end_time $END_TIME \
  --category Sports \
  --title "Test Pool" \
  --description "Deployment verification" \
  --metadata_url "ipfs://test" \
  --options 2 \
  --config '{"min_stake": 1, "max_stake": 1000, "max_total_stake": 10000}'
```

## Environment Variables Reference

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `TREASURY_ADDRESS` | Receives protocol fees | Admin address | `GB...` |
| `FEE_BPS` | Protocol fee (100=1%) | 100 | `200` (2%) |
| `RESOLUTION_DELAY` | Delay after pool end (seconds) | 3600 | `7200` (2 hours) |
| `MIN_POOL_DURATION` | Minimum pool duration (seconds) | 3600 | `14400` (4 hours) |
| `MAX_PREDICTIONS_PER_USER` | Max predictions per user | 10 | `5` |

## Recommended Parameter Values

### For Testing
```bash
FEE_BPS=100          # 1% fee
RESOLUTION_DELAY=300  # 5 minutes
MIN_POOL_DURATION=600 # 10 minutes
MAX_PREDICTIONS_PER_USER=10
```

### For Production
```bash
FEE_BPS=200           # 2% fee
RESOLUTION_DELAY=7200  # 2 hours
MIN_POOL_DURATION=14400 # 4 hours
MAX_PREDICTIONS_PER_USER=5
```

## Network Configuration

### Testnet
```bash
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

### Mainnet
```bash
stellar network add mainnet \
  --rpc-url https://soroban.stellar.org:443 \
  --network-passphrase "Public Global Stellar Network ; September 2015"
```

## Common Issues & Solutions

### 1. Insufficient Balance
```bash
# Check balance
stellar account balance deployer --network testnet

# Fund testnet wallet
stellar account fund deployer --network testnet
```

### 2. CLI Not Found
```bash
# Install Stellar CLI
cargo install stellar-cli

# Install wasm-opt
cargo install wasm-opt
# OR
brew install binaryen        # macOS
# OR
sudo apt install binaryen    # Ubuntu/Debian
```

### 3. Build Errors
```bash
# Update Rust
rustup update

# Add WASM target
rustup target add wasm32-unknown-unknown

# Clean build
cargo clean
cargo build --target wasm32-unknown-unknown --release
```

### 4. Contract Initialization Failed
Check parameter ranges:
- `fee_bps`: 0-10000 (0-100%)
- `resolution_delay`: > 0, ≤ 2592000 (30 days)
- `min_pool_duration`: > 0
- `max_predictions_per_user`: 0 = no limit, > 0 = limit

## Security Checklist

### Before Deployment
- [ ] Test on testnet first
- [ ] Verify all parameters
- [ ] Use secure wallet for treasury
- [ ] Backup secret keys
- [ ] Document deployment process

### After Deployment
- [ ] Save contract IDs securely
- [ ] Whitelist necessary tokens
- [ ] Assign operator roles
- [ ] Initialize oracle (if using price feeds)
- [ ] Test basic operations
- [ ] Monitor initial activity

## Quick Test Script
```bash
#!/bin/bash
# quick_test.sh
# Test basic contract functionality after deployment

NETWORK=$1
PD_ID=$2
AC_ID=$3
ADMIN=$4

echo "Testing contract $PD_ID on $NETWORK"

# 1. Check treasury
echo "1. Checking treasury..."
stellar contract read --id $PD_ID --network $NETWORK -- get_treasury

# 2. Check fees
echo "2. Checking fees..."
stellar contract read --id $PD_ID --network $NETWORK -- get_fee_bps

# 3. Check admin role
echo "3. Checking admin role..."
stellar contract read --id $AC_ID --network $NETWORK -- has_role --user $ADMIN --role Admin

echo "Test complete!"
```

Usage: `./quick_test.sh testnet <PD_ID> <AC_ID> <ADMIN_ADDRESS>`

---

## Emergency Contacts

- **Stellar Documentation**: https://developers.stellar.org/docs/soroban
- **Stellar Discord**: https://discord.gg/stellar (Soroban channel)
- **Project Repository**: Check issues and documentation
- **Security Team**: Contact if you suspect security issues

---

**Last Updated**: 2026-08-28  
**For detailed instructions**: See `DEPLOYMENT_GUIDE_COMPLETE.md`
