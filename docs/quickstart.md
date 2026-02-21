# Quickstart: Make Your First Prediction in 5 Minutes

Get started with PrediFi by placing your first prediction. This guide walks you through connecting your wallet, finding a market, and placing a bet.

## Prerequisites

- A Stellar wallet (e.g., [Freighter](https://freighter.app/))
- XLM or supported token for gas fees
- Basic familiarity with Stellar/Soroban

:::tip
**Testnet First**: Start on Stellar testnet to experiment without real funds. Get testnet XLM from the [Stellar Laboratory](https://laboratory.stellar.org/#account-creator?network=test).
:::

## Step 1: Install the Soroban SDK

```bash
npm install @stellar/stellar-sdk
```

Or with TypeScript:

```bash
npm install @stellar/stellar-sdk @types/node
```

## Step 2: Connect Your Wallet

```typescript
import { Contract, Networks, nativeToScVal } from '@stellar/stellar-sdk';

// Connect to Stellar network
const network = Networks.TESTNET; // or Networks.PUBLIC for mainnet
const server = new StellarSdk.Server('https://horizon-testnet.stellar.org');

// Contract address (replace with actual deployed contract)
const contractId = 'YOUR_CONTRACT_ID_HERE';

// Initialize contract client
const contract = new Contract(contractId);
```

:::info
**Contract Addresses**: Contract addresses differ between testnet and mainnet. Check the [deployment guide](./deployment.md) for current addresses.
:::

## Step 3: Find an Active Pool

```typescript
// Get pool details
async function getPool(poolId: number) {
  const result = await contract.call('get_pool', {
    pool_id: nativeToScVal(poolId, { type: 'u64' })
  });
  
  return {
    endTime: result.end_time,
    resolved: result.resolved,
    outcome: result.outcome,
    totalStake: result.total_stake,
    description: result.description
  };
}

// Example: Get pool #1
const pool = await getPool(1);
console.log('Pool:', pool.description);
```

## Step 4: Place Your Prediction

```typescript
import { Keypair, TransactionBuilder, Operation } from '@stellar/stellar-sdk';

async function placePrediction(
  poolId: number,
  amount: number, // in smallest unit (e.g., stroops for XLM)
  outcome: number, // 0 for "No", 1 for "Yes", etc.
  sourceKeypair: Keypair
) {
  // Build transaction
  const account = await server.loadAccount(sourceKeypair.publicKey());
  
  const transaction = new TransactionBuilder(account, {
    fee: '100', // Base fee
    networkPassphrase: network
  })
    .addOperation(
      contract.call('place_prediction', {
        user: sourceKeypair.publicKey(),
        pool_id: nativeToScVal(poolId, { type: 'u64' }),
        amount: nativeToScVal(amount, { type: 'i128' }),
        outcome: nativeToScVal(outcome, { type: 'u32' })
      })
    )
    .setTimeout(30)
    .build();

  // Sign and submit
  transaction.sign(sourceKeypair);
  const result = await server.submitTransaction(transaction);
  
  return result;
}

// Example: Predict "Yes" (outcome 1) with 100 tokens
const keypair = Keypair.fromSecret('YOUR_SECRET_KEY');
await placePrediction(1, 1000000000, 1, keypair);
```

:::warning
**Amount Format**: Amounts are in the smallest unit of the token. For XLM, use stroops (1 XLM = 10,000,000 stroops). For other tokens, check the token's decimal precision.
:::

## Step 5: Check Your Prediction Status

```typescript
async function getUserPredictions(userAddress: string, offset = 0, limit = 10) {
  const result = await contract.call('get_user_predictions', {
    user: userAddress,
    offset: nativeToScVal(offset, { type: 'u32' }),
    limit: nativeToScVal(limit, { type: 'u32' })
  });
  
  return result.map((pred: any) => ({
    poolId: pred.pool_id,
    amount: pred.amount,
    outcome: pred.user_outcome,
    poolResolved: pred.pool_resolved,
    poolOutcome: pred.pool_outcome
  }));
}

const predictions = await getUserPredictions(keypair.publicKey());
console.log('Your predictions:', predictions);
```

## Complete Example

Here's a complete working example:

```typescript
import { 
  Contract, 
  Networks, 
  Keypair, 
  Server,
  TransactionBuilder,
  nativeToScVal 
} from '@stellar/stellar-sdk';

const network = Networks.TESTNET;
const server = new Server('https://horizon-testnet.stellar.org');
const contractId = 'YOUR_CONTRACT_ID';
const contract = new Contract(contractId);

// Load your wallet
const keypair = Keypair.fromSecret('YOUR_SECRET_KEY');
const account = await server.loadAccount(keypair.publicKey());

// Get pool info
const pool = await contract.call('get_pool', {
  pool_id: nativeToScVal(1, { type: 'u64' })
});

console.log(`Pool: ${pool.description}`);
console.log(`Ends at: ${new Date(pool.end_time * 1000)}`);

// Place prediction
const tx = new TransactionBuilder(account, {
  fee: '100',
  networkPassphrase: network
})
  .addOperation(
    contract.call('place_prediction', {
      user: keypair.publicKey(),
      pool_id: nativeToScVal(1, { type: 'u64' }),
      amount: nativeToScVal(1000000000, { type: 'i128' }), // 100 tokens
      outcome: nativeToScVal(1, { type: 'u32' }) // "Yes"
    })
  )
  .setTimeout(30)
  .build();

tx.sign(keypair);
const result = await server.submitTransaction(tx);
console.log('Transaction hash:', result.hash);
```

## Next Steps

- Learn about the [Prediction Lifecycle](./prediction-lifecycle.md)
- Explore [Contract Reference](./contract-reference.md)
- Understand [Oracle Resolution](./oracles.md)
- Check [Troubleshooting](./troubleshooting.md) for common issues

:::tip
**Need Help?** Join our [Telegram community](https://t.me/predifi_onchain_build/1) for support and updates.
:::
