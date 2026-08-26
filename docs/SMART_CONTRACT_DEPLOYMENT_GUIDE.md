# PrediFi Smart Contract Deployment Guide

This guide provides step-by-step instructions for deploying the PrediFi smart contracts to Stellar testnet and mainnet, including wallet setup, contract compilation, deployment, initialization, token whitelisting, oracle registration, and post-deployment verification.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Wallet Setup](#wallet-setup)
- [Network Configuration](#network-configuration)
- [Contract Compilation](#contract-compilation)
- [Deployment to Testnet](#deployment-to-testnet)
- [Deployment to Mainnet](#deployment-to-mainnet)
- [Initialization Parameters](#initialization-parameters)
- [Token Whitelisting](#token-whitelisting)
- [Oracle Registration](#oracle-registration)
- [Role Assignment](#role-assignment)
- [Post-Deployment Verification](#post-deployment-verification)
- [Troubleshooting](#troubleshooting)
- [Security Considerations](#security-considerations)

---

## Prerequisites

Before deploying the PrediFi contracts, ensure you have the following installed:

### Required Tools

1. **Rust Toolchain** (with WASM target)
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. **Stellar CLI** (preferred) or Soroban CLI
   ```bash
   cargo install stellar-cli
   # or
   cargo install soroban-cli
   ```

3. **Binaryen** (for WASM optimization)
   ```bash
   # macOS (Homebrew)
   brew install binaryen
   
   # Ubuntu/Debian
   sudo apt-get install -y binaryen
   
   # From Cargo
   cargo install wasm-opt
   ```

4. **Git** (for cloning the repository)

### System Requirements

- **RAM**: 8GB minimum (16GB recommended)
- **Disk Space**: 5GB free space
- **Network**: Stable internet connection for RPC calls

---

## Wallet Setup

### 1. Generate or Import Wallet

#### Generate New Wallet

```bash
# Generate a new key pair for testnet
stellar keys generate --network testnet deployer

# Generate a new key pair for mainnet
stellar keys generate --network mainnet deployer
```

#### Import Existing Wallet

```bash
# Import from secret key
stellar keys add --network testnet deployer --secret-key S...

# Import from private key file
stellar keys add --network mainnet deployer --file /path/to/private_key.txt
```

### 2. Fund Your Wallet

#### Testnet Funding

```bash
# Request testnet XLM from friendbot
stellar account fund deployer --network testnet
```

Or use the [Stellar Testnet Faucet](https://friendbot.stellar.org/) with your public key.

#### Mainnet Funding

For mainnet, you'll need to acquire XLM from an exchange and transfer it to your wallet address:

```bash
# Check your wallet address
stellar keys address deployer --network mainnet

# Check balance
stellar account balance deployer --network mainnet
```

**Recommended minimum balance**: 100 XLM for deployment (covers transaction fees and rent).

### 3. Backup Your Keys

**CRITICAL**: Always backup your secret key securely.

```bash
# Export secret key (store this securely!)
stellar keys show deployer --network testnet --secret
```

Store your secret key in:
- A password manager (e.g., 1Password, Bitwarden)
- A hardware wallet (if supported)
- An encrypted file with strong passphrase

**NEVER**:
- Commit secret keys to version control
- Share secret keys via email/chat
- Store secret keys in plain text

---

## Network Configuration

### Configure Stellar Networks

#### Testnet

```bash
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

#### Mainnet

```bash
stellar network add mainnet \
  --rpc-url https://soroban.stellar.org:443 \
  --network-passphrase "Public Global Stellar Network ; September 2015"
```

### Verify Network Configuration

```bash
# List configured networks
stellar network list

# Set active network
stellar network use testnet
# or
stellar network use mainnet
```

---

## Contract Compilation

### 1. Clone the Repository

```bash
git clone <repository-url>
cd predifi/contract
```

### 2. Build Contracts

#### Using Build Script (Recommended)

```bash
# From the contract directory
./build.sh
```

This script:
- Compiles contracts to WASM with Cargo for the `wasm32-unknown-unknown` target
- Optimizes WASM files with `wasm-opt`
- Runs Stellar CLI contract optimization
- Outputs optimized files to `target/wasm32-unknown-unknown/release/`

#### Manual Build

```bash
# Build WASM
cargo build --target wasm32-unknown-unknown --release

# Optimize with wasm-opt
wasm-opt -Oz --enable-bulk-memory \
  target/wasm32-unknown-unknown/release/access_control.wasm \
  -o target/wasm32-unknown-unknown/release/access_control_optimized.wasm

wasm-opt -Oz --enable-bulk-memory \
  target/wasm32-unknown-unknown/release/predifi_contract.wasm \
  -o target/wasm32-unknown-unknown/release/predifi_contract_optimized.wasm
```

### 3. Verify Build Output

```bash
# Check file sizes
ls -lh target/wasm32-unknown-unknown/release/*_optimized.wasm
```

Expected output:
- `predifi_contract_optimized.wasm`: ~200-400 KB
- `access_control_optimized.wasm`: ~50-100 KB

### 4. Additional Optimization (Optional)

For even smaller deployment footprints:

```bash
# Run Stellar CLI optimization (combines well with wasm-opt -Oz)
stellar contract optimize \
  --wasm target/wasm32-unknown-unknown/release/predifi_contract_optimized.wasm

stellar contract optimize \
  --wasm target/wasm32-unknown-unknown/release/access_control_optimized.wasm
```

---

## Deployment to Testnet

### Automated Deployment (Recommended)

The deployment script handles the entire process: build → optimize → deploy → initialize.

```bash
cd contract/scripts

# Deploy to testnet
./deploy.sh testnet deployer
```

### Custom Parameters

Set environment variables to customize initialization:

```bash
# Custom treasury address and fee
TREASURY_ADDRESS=GB... FEE_BPS=250 ./deploy.sh testnet deployer

# Custom fee only (treasury defaults to deployer)
FEE_BPS=150 ./deploy.sh testnet deployer
```

### Manual Deployment

If you prefer manual control over each step:

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

#### Step 4: Initialize PrediFi Contract

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
  --referral_cut_bps 0
```

### Save Deployment Information

The deployment script automatically saves deployment details to `deployed_contracts_testnet.json`:

```json
{
  "network": "testnet",
  "timestamp": "2026-07-28T14:00:00Z",
  "cli_used": "stellar",
  "source_account": "deployer",
  "contracts": {
    "access_control": {
      "id": "CD...",
      "admin": "GC..."
    },
    "predifi_contract": {
      "id": "CB...",
      "treasury": "GC...",
      "fee_bps": 100
    }
  }
}
```

---

## Deployment to Mainnet

### Pre-Deployment Checklist

Before deploying to mainnet:

- [ ] Review all contract code thoroughly
- [ ] Test extensively on testnet
- [ ] Ensure wallet has sufficient XLM (100+ XLM recommended)
- [ ] Prepare treasury address (can be same as deployer)
- [ ] Decide on protocol parameters (fee, resolution delay, etc.)
- [ ] Have operator addresses ready for role assignment
- [ ] Prepare oracle addresses if using price feeds
- [ ] Test the deployment script on testnet first

### Automated Deployment

```bash
cd contract/scripts

# Deploy to mainnet
./deploy.sh mainnet deployer
```

### Custom Parameters for Mainnet

```bash
# Example: 2% fee, custom treasury
TREASURY_ADDRESS=GB... FEE_BPS=200 ./deploy.sh mainnet deployer

# Example: 1.5% fee, 1 hour resolution delay
TREASURY_ADDRESS=GB... FEE_BPS=150 RESOLUTION_DELAY=3600 ./deploy.sh mainnet deployer
```

### Manual Deployment (Mainnet)

The steps are identical to testnet, but use `--network mainnet`:

#### Step 1: Deploy AccessControl

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/access_control_optimized.wasm \
  --source deployer \
  --network mainnet
```

#### Step 2: Initialize AccessControl

```bash
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network mainnet \
  -- \
  init \
  --admin $(stellar keys address deployer --network mainnet)
```

#### Step 3: Deploy PrediFi Contract

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predifi_contract_optimized.wasm \
  --source deployer \
  --network mainnet
```

#### Step 4: Initialize PrediFi Contract

```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network mainnet \
  -- \
  init \
  --access_control <ACCESS_CONTROL_ID> \
  --treasury <TREASURY_ADDRESS> \
  --fee_bps 100 \
  --resolution_delay 3600 \
  --referral_cut_bps 0
```

### Save Deployment Information

The script saves to `deployed_contracts_mainnet.json`. **Backup this file securely** as it contains critical contract addresses.

---

## Initialization Parameters

### AccessControl Initialization

The `access-control` contract requires only one parameter:

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `admin` | Address | The initial administrator address | `GD...` |

**Example**:
```bash
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  init \
  --admin GD...
```

### PrediFi Contract Initialization

The `predifi-contract` requires the following parameters:

| Parameter | Type | Description | Default | Recommended Range |
|-----------|------|-------------|---------|-------------------|
| `access_control` | Address | Access control contract address | Required | - |
| `treasury` | Address | Address that receives protocol fees | Deployer address | Secure wallet |
| `fee_bps` | u32 | Protocol fee in basis points (100 = 1%) | 100 | 0-500 (0-5%) |
| `resolution_delay` | u64 | Delay after pool end before resolution (seconds) | 3600 | 300-86400 (5min-24hr) |
| `referral_cut_bps` | u32 | Referral fee share in basis points | 0 | 0-5000 (0-50%) |

**Parameter Details**:

#### `fee_bps` (Protocol Fee)

- **100 bps** = 1% fee on winning payouts
- **200 bps** = 2% fee
- **0 bps** = No protocol fee (for testing or subsidized operations)

#### `resolution_delay`

- **300** = 5 minutes (fast resolution, higher dispute risk)
- **3600** = 1 hour (balanced)
- **86400** = 24 hours (maximum safety, slower payouts)

#### `referral_cut_bps`

- **0** = No referral program
- **1000** = 10% of protocol fee goes to referrer
- **5000** = 50% of protocol fee goes to referrer (maximum)

**Example Initialization**:

```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  init \
  --access_control CD... \
  --treasury GD... \
  --fee_bps 200 \
  --resolution_delay 1800 \
  --referral_cut_bps 1000
```

---

## Token Whitelisting

Before users can place predictions, you must whitelist the tokens they can use for betting.

### Add Token to Whitelist

Only users with the **Admin role (0)** can whitelist tokens.

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

### Common Tokens to Whitelist

#### Stellar Native Asset (XLM)

XLM is typically always available, but you may want to explicitly allow it:

```bash
# XLM is represented by the native asset identifier
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_token_to_whitelist \
  --admin $(stellar keys address deployer --network testnet) \
  --token CD...  # Use the native asset address
```

#### Custom Tokens

For custom tokens (e.g., USDC, stablecoins):

```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_token_to_whitelist \
  --admin $(stellar keys address deployer --network testnet) \
  --token GB...  # Token contract address
```

### Remove Token from Whitelist

```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  remove_token_from_whitelist \
  --admin $(stellar keys address deployer --network testnet) \
  --token <TOKEN_CONTRACT_ADDRESS>
```

### Check Whitelist Status

```bash
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  is_token_whitelisted \
  --token <TOKEN_CONTRACT_ADDRESS>
```

---

## Oracle Registration

Oracle registration enables automated price-based pool resolution using external price feeds (e.g., Pyth Network).

### Initialize Oracle Configuration

Configure the oracle with Pyth contract address and validation parameters:

```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  init_oracle \
  --admin $(stellar keys address deployer --network testnet) \
  --pyth_contract <PYTH_CONTRACT_ADDRESS> \
  --max_price_age 60 \
  --min_confidence_ratio 9500
```

**Parameters**:

| Parameter | Type | Description | Recommended |
|-----------|------|-------------|-------------|
| `pyth_contract` | Address | Pyth Network contract address | Network-specific |
| `max_price_age` | u64 | Maximum age of price data (seconds) | 60 (1 minute) |
| `min_confidence_ratio` | u32 | Minimum confidence ratio (basis points) | 9500 (95%) |

**Pyth Contract Addresses**:

- **Testnet**: `CC...` (check Pyth documentation for current address)
- **Mainnet**: `CD...` (check Pyth documentation for current address)

### Add Oracle to Whitelist

Whitelist oracle addresses that can update price feeds:

```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_oracle \
  --admin $(stellar keys address deployer --network testnet) \
  --oracle_address <ORACLE_ADDRESS>
```

### Update Price Feed

Authorized oracles can update price data:

```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source oracle \
  --network testnet \
  -- \
  update_price_feed \
  --feed_pair ETHUSD \
  --price 3000000000 \
  --conf 100000 \
  --timestamp <UNIX_TIMESTAMP> \
  --publish_time <UNIX_TIMESTAMP>
```

**Parameters**:

| Parameter | Type | Description |
|-----------|------|-------------|
| `feed_pair` | Symbol | Asset pair (e.g., `ETHUSD`, `BTCUSD`) |
| `price` | i128 | Price in scaled units (check Pyth scaling) |
| `conf` | i128 | Confidence interval |
| `timestamp` | u64 | When the price was observed |
| `publish_time` | u64 | When the price was published |

### Set Price Condition for Pool

Link a pool to a price feed for automated resolution:

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

**Parameters**:

| Parameter | Type | Description |
|-----------|------|-------------|
| `pool_id` | u64 | Pool identifier |
| `feed_pair` | Symbol | Asset pair to monitor |
| `target_price` | i128 | Target price for resolution |
| `operator_type` | u32 | Comparison: 0=Equal, 1=GreaterThan, 2=LessThan |
| `tolerance_bps` | u32 | Tolerance in basis points (100 = 1%) |

### Resolve Pool from Price

Once the pool end time + resolution delay has passed, anyone can trigger automated resolution:

```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source any_user \
  --network testnet \
  -- \
  resolve_pool_from_price \
  --pool_id <POOL_ID>
```

---

## Role Assignment

After deployment, assign roles to trusted addresses for protocol operations.

### Role Hierarchy

| Role | Value | Permissions |
|------|-------|-------------|
| **Admin** | 0 | Full control: pause, fees, treasury, whitelisting, upgrades |
| **Operator** | 1 | Pool operations: resolve pools, cancel pools, set stake limits |
| **Moderator** | 2 | Dispute resolution (reserved for future use) |
| **Oracle** | 3 | Price feed updates and oracle resolution |
| **User** | 4 | Basic participant (no special permissions) |

### Assign Operator Role

```bash
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  assign_role \
  --admin_caller $(stellar keys address deployer --network testnet) \
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
  --admin_caller $(stellar keys address deployer --network testnet) \
  --user <ORACLE_ADDRESS> \
  --role Oracle
```

### Assign Multiple Roles

An address can have multiple roles:

```bash
# Assign both Operator and Oracle to same address
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  assign_role \
  --admin_caller $(stellar keys address deployer --network testnet) \
  --user <MULTI_ROLE_ADDRESS> \
  --role Operator

stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  assign_role \
  --admin_caller $(stellar keys address deployer --network testnet) \
  --user <MULTI_ROLE_ADDRESS> \
  --role Oracle
```

### Revoke Role

```bash
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  revoke_role \
  --admin_caller $(stellar keys address deployer --network testnet) \
  --user <USER_ADDRESS> \
  --role Operator
```

### Check User Roles

```bash
stellar contract read \
  --id <ACCESS_CONTROL_ID> \
  --network testnet \
  -- \
  has_role \
  --user <USER_ADDRESS> \
  --role Operator
```

### Transfer Admin Role (Two-Step Process)

For security, use the two-step admin transfer:

```bash
# Step 1: Current admin proposes new admin
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source deployer \
  --network testnet \
  -- \
  propose_new_admin \
  --admin_caller $(stellar keys address deployer --network testnet) \
  --proposed_admin <NEW_ADMIN_ADDRESS>

# Step 2: New admin accepts the role
stellar contract invoke \
  --id <ACCESS_CONTROL_ID> \
  --source new_admin \
  --network testnet \
  -- \
  accept_admin_role \
  --new_admin <NEW_ADMIN_ADDRESS>
```

---

## Post-Deployment Verification

After deployment, verify that everything is configured correctly.

### 1. Verify Contract Deployment

```bash
# Check AccessControl contract
stellar contract read \
  --id <ACCESS_CONTROL_ID> \
  --network testnet \
  -- \
  is_admin \
  --address $(stellar keys address deployer --network testnet)

# Check PrediFi contract
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  get_treasury
```

### 2. Verify Initialization Parameters

```bash
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

# Check referral cut
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  get_referral_cut_bps
```

### 3. Verify Token Whitelist

```bash
# Check if a token is whitelisted
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  is_token_whitelisted \
  --token <TOKEN_ADDRESS>
```

### 4. Verify Oracle Configuration

```bash
# Check oracle config
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  get_oracle_config

# Check if oracle is whitelisted
stellar contract read \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  -- \
  is_oracle_whitelisted \
  --oracle <ORACLE_ADDRESS>
```

### 5. Verify Role Assignments

```bash
# Check operator count
stellar contract read \
  --id <ACCESS_CONTROL_ID> \
  --network testnet \
  -- \
  get_operator_count

# Check specific user role
stellar contract read \
  --id <ACCESS_CONTROL_ID> \
  --network testnet \
  -- \
  has_role \
  --user <USER_ADDRESS> \
  --role Operator
```

### 6. Test Pool Creation

Create a test pool to verify the contract is operational:

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
  --title "Test Pool" \
  --description "A test pool for deployment verification" \
  --metadata_url "ipfs://test" \
  --options 2 \
  --config '{"min_stake": 1, "max_stake": 1000, "max_total_stake": 10000}'
```

### 7. Monitor Contract Events

Set up event monitoring to track contract activity:

```bash
# Monitor all events (requires a subscription service)
stellar contract events \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet \
  --from <LEDGER_SEQUENCE>
```

### 8. Verify Contract Code

```bash
# Get contract code hash
stellar contract inspect \
  --id <PREDIFI_CONTRACT_ID> \
  --network testnet
```

Compare the hash with your local build to ensure the deployed code matches.

---

## Troubleshooting

### Common Issues

#### 1. Insufficient Balance

**Error**: `Error(Contract, #44)` - Insufficient balance

**Solution**: Ensure your wallet has enough XLM for transaction fees:
```bash
stellar account balance deployer --network testnet
stellar account fund deployer --network testnet
```

#### 2. Unauthorized Access

**Error**: `Error(Contract, #10)` - Unauthorized

**Solution**: Verify the caller has the required role:
```bash
stellar contract read \
  --id <ACCESS_CONTROL_ID> \
  --network testnet \
  -- \
  has_role \
  --user <YOUR_ADDRESS> \
  --role Admin
```

#### 3. Contract Not Initialized

**Error**: `Error(Contract, #1)` - NotInitialized

**Solution**: Ensure you called `init` on both contracts before other operations.

#### 4. Token Not Whitelisted

**Error**: `Error(Contract, #150)` - TokenError

**Solution**: Whitelist the token before using it for predictions:
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_token_to_whitelist \
  --admin <ADMIN_ADDRESS> \
  --token <TOKEN_ADDRESS>
```

#### 5. wasm-opt Not Found

**Error**: `wasm-opt: command not found`

**Solution**: Install Binaryen:
```bash
# macOS
brew install binaryen

# Ubuntu/Debian
sudo apt-get install -y binaryen

# Cargo
cargo install wasm-opt
```

#### 6. Network Connection Issues

**Error**: RPC connection failed

**Solution**: 
- Check your internet connection
- Verify RPC URL is correct for the network
- Try a different RPC endpoint if available

#### 7. Transaction Timeout

**Error**: Transaction timeout

**Solution**:
- Increase timeout in CLI configuration
- Check network congestion
- Retry during off-peak hours

### Debug Mode

For debugging, you can build contracts with debug symbols:

```bash
# Build with debug profile
cargo build --profile release-with-logs --target wasm32-unknown-unknown
```

This provides more detailed error messages but increases WASM size.

### Get Help

- **Stellar Discord**: https://discord.gg/stellar
- **Soroban Documentation**: https://developers.stellar.org/docs/soroban
- **PrediFi Issues**: Check the project's issue tracker

---

## Security Considerations

### Pre-Deployment Security

1. **Code Review**
   - Thoroughly review all contract code
   - Run security audits if handling significant value
   - Test all edge cases and error conditions

2. **Access Control**
   - Use hardware wallets for admin accounts
   - Implement multi-sig for critical operations
   - Limit admin permissions to necessary addresses

3. **Parameter Validation**
   - Test initialization parameters on testnet first
   - Ensure fee rates are reasonable
   - Set appropriate resolution delays

### Post-Deployment Security

1. **Monitor Activity**
   - Set up alerts for unusual activity
   - Monitor large transactions
   - Track error rates (see ERROR_HANDLING_REFERENCE.md)

2. **Role Management**
   - Regularly review role assignments
   - Revoke roles from inactive addresses
   - Use two-step admin transfer

3. **Emergency Procedures**
   - Have a pause mechanism ready
   - Prepare emergency upgrade procedures
   - Document incident response steps

4. **Key Security**
   - Never commit secret keys to version control
   - Use secure key storage (hardware wallet, password manager)
   - Rotate keys periodically if possible

### Mainnet-Specific Considerations

1. **Gradual Rollout**
   - Start with low limits and increase gradually
   - Monitor for unexpected behavior
   - Have rollback plan ready

2. **Insurance**
   - Consider protocol insurance if handling large amounts
   - Set aside emergency funds
   - Implement circuit breakers

3. **Compliance**
   - Ensure compliance with local regulations
   - Implement KYC/AML if required
   - Consult legal counsel

### Best Practices

1. **Testing**
   - Deploy to testnet first
   - Run comprehensive integration tests
   - Test with real users on testnet

2. **Monitoring**
   - Set up comprehensive logging
   - Monitor contract events
   - Track key metrics (TVL, volume, errors)

3. **Documentation**
   - Document all deployment steps
   - Maintain runbooks for common operations
   - Keep contact information for team members

4. **Upgrades**
   - Plan upgrade path before deployment
   - Test upgrade procedures on testnet
   - Communicate upgrades to users in advance

---

## Appendix: Quick Reference

### Deployment Commands Summary

**Testnet**:
```bash
cd contract/scripts
./deploy.sh testnet deployer
```

**Mainnet**:
```bash
cd contract/scripts
./deploy.sh mainnet deployer
```

**Custom Parameters**:
```bash
TREASURY_ADDRESS=GB... FEE_BPS=200 ./deploy.sh testnet deployer
```

### Common Post-Deployment Commands

**Whitelist Token**:
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  add_token_to_whitelist \
  --admin <ADMIN_ADDRESS> \
  --token <TOKEN_ADDRESS>
```

**Assign Operator Role**:
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

**Initialize Oracle**:
```bash
stellar contract invoke \
  --id <PREDIFI_CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- \
  init_oracle \
  --admin <ADMIN_ADDRESS> \
  --pyth_contract <PYTH_ADDRESS> \
  --max_price_age 60 \
  --min_confidence_ratio 9500
```

### Network RPC URLs

- **Testnet**: `https://soroban-testnet.stellar.org:443`
- **Mainnet**: `https://soroban.stellar.org:443`

### Network Passphrases

- **Testnet**: `Test SDF Network ; September 2015`
- **Mainnet**: `Public Global Stellar Network ; September 2015`

---

## Related Documentation

- [Error Handling Reference](ERROR_HANDLING_REFERENCE.md) - Comprehensive error codes and troubleshooting
- [Contract README](../contract/README.md) - Contract architecture and features
- [Access Control Documentation](../contract/contracts/access-control/README.md) - Role-based access control details
- [PrediFi Errors Reference](../contract/contracts/predifi-errors/README.md) - Contract error codes

---

## Support

For deployment issues or questions:

1. Check the [troubleshooting section](#troubleshooting)
2. Review the [error handling reference](ERROR_HANDLING_REFERENCE.md)
3. Open an issue on the project repository
4. Contact the development team through official channels

---

**Last Updated**: 2026-07-28
**Version**: 1.0.0
