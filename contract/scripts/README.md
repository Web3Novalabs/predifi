# PrediFi Deployment Scripts

This directory contains scripts to automate the deployment and initialization of PrediFi smart contracts on Stellar.

## Files

- `deploy.sh`: Main deployment script that handles build, optimization, deployment, and initialization.
- `deployed_contracts_<network>.json`: (Generated) Stores the contract IDs and parameters for a specific network.

## Prerequisites

1.  **Stellar CLI**: Install via cargo:
    ```bash
    cargo install stellar-cli
    ```
2.  **Configured Networks**: Ensure you have `testnet` and `mainnet` configured in your CLI:
    ```bash
    stellar network add testnet --rpc-url https://soroban-testnet.stellar.org:443 --network-passphrase "Test SDF Network ; September 2015"
    ```
3.  **Source Account**: Have a funded account ready in your keys:
    ```bash
    stellar keys generate --network testnet default
    ```

## Usage

### ⚙️ Standard Deployment

To deploy to Testnet using an account named `default`:

```bash
./deploy.sh testnet default
```

### 🛠️ Customizing Parameters

You can set environment variables to customize the initialization:

```bash
TREASURY_ADDRESS=GC... FEE_BPS=250 ./deploy.sh testnet default
```

- `TREASURY_ADDRESS`: The address that receives fees (defaults to the deployer address).
- `FEE_BPS`: Fee in basis points (100 = 1%).

## Environment Stages

The script generates a JSON file for each network (e.g., `deployed_contracts_testnet.json`). These files should be used by the frontend and backend to identify the correct contract addresses.

```json
{
  "network": "testnet",
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

## Post-Deployment

### 👤 Role Assignment

The `AccessControl` contract is initialized with the deployer as `Admin`. To allow an address to resolve pools, you must assign the `Operator` role (1):

```bash
stellar contract invoke \
    --id <ACCESS_CONTROL_ID> \
    --source default \
    --network testnet \
    -- \
    assign_role \
    --admin_caller <ADMIN_ADDRESS> \
    --user <OPERATOR_ADDRESS> \
    --role Operator
```

Note: The `Role` argument is an Enum. In the latest CLI, you can use the variant name `Operator`.
