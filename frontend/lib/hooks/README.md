# Network Guard Hook

## Overview

The `useNetworkGuard` hook provides network validation and switching capabilities to prevent users from submitting transactions on the wrong network.

## Key Features

### 1. Real-time Network Detection
- Monitors the current network connection
- Updates automatically when user switches networks in their wallet

### 2. Pre-submission Network Validation
- **Critical**: Call `checkNetworkBeforeSubmit()` immediately before every transaction submission
- Catches mid-session network switches that could lead to unrecoverable errors
- Returns detailed error information including current and required networks

### 3. Programmatic Network Switching
- Prompts user to switch to the correct network
- Provides structured error handling and recovery actions

## Usage Example

```typescript
import { useNetworkGuard } from "@/lib/hooks/useNetworkGuard";
import { TransactionProgress } from "@/components/ui/transaction-progress";
import { useState } from "react";

function MyTransactionComponent() {
  const { checkNetworkBeforeSubmit } = useNetworkGuard();
  const [txStatus, setTxStatus] = useState("idle");
  const [txError, setTxError] = useState("");
  const [txParams, setTxParams] = useState(null);

  const submitTransaction = async (params) => {
    // CRITICAL: Check network immediately before submission
    const networkError = await checkNetworkBeforeSubmit();
    
    if (networkError) {
      setTxStatus("failed");
      setTxError(networkError.message);
      return;
    }

    // Store params for retry
    setTxParams(params);
    setTxStatus("submitted");

    try {
      // Submit your transaction here
      const txHash = await sendTransaction(params);
      setTxStatus("confirmed");
    } catch (error) {
      setTxStatus("failed");
      setTxError(error.message);
    }
  };

  const handleRetry = () => {
    if (txParams) {
      submitTransaction(txParams);
    }
  };

  return (
    <div>
      <button onClick={() => submitTransaction({ amount: "100" })}>
        Submit Transaction
      </button>
      
      <TransactionProgress
        status={txStatus}
        errorMessage={txError}
        onRetry={handleRetry}
      />
    </div>
  );
}
```

## API Reference

### `checkNetworkBeforeSubmit(): Promise<NetworkMismatchError | null>`

**Returns:**
- `null` if the network is correct
- `NetworkMismatchError` object if there's a mismatch:
  ```typescript
  {
    currentNetwork: string;    // e.g., "Polygon"
    requiredNetwork: string;   // e.g., "Ethereum Mainnet"
    message: string;           // User-friendly error message
  }
  ```

**When to call:**
- Immediately before every transaction submission
- After any user action that triggers a blockchain transaction
- Before any wallet signature request

**Why it's critical:**
- Users can switch networks at any time in their wallet
- Mount-time checks become stale during long sessions
- Network mismatches cause unrecoverable transaction failures
- The user would need to rebuild the entire action from scratch

## TransactionProgress Integration

The `TransactionProgress` component now supports retry functionality:

```typescript
<TransactionProgress
  status={txStatus}
  txHash={txHash}
  errorMessage={errorMessage}
  onRetry={handleRetry}      // Callback to retry with original params
  isRetrying={isRetrying}    // Show loading state during retry
/>
```

### Retry Best Practices

1. **Store transaction parameters** when first submitting
2. **Reuse exact same parameters** on retry - don't re-validate or rebuild
3. **Check network again** in your retry handler (call `checkNetworkBeforeSubmit`)
4. **Don't stack progress components** - reuse the same instance
5. **Clear retry params** only after successful confirmation

## Common Pitfalls

❌ **Wrong**: Only checking network on component mount
```typescript
useEffect(() => {
  checkNetworkBeforeSubmit(); // Stale after mount
}, []);
```

✅ **Correct**: Check immediately before submission
```typescript
const handleSubmit = async () => {
  const error = await checkNetworkBeforeSubmit();
  if (error) { /* handle */ }
  // ... submit transaction
};
```

❌ **Wrong**: No retry functionality
```typescript
// User has to rebuild entire form
```

✅ **Correct**: Store params and allow retry
```typescript
const [savedParams, setSavedParams] = useState(null);

const submit = async (params) => {
  setSavedParams(params);
  // ... attempt transaction
};

const retry = () => submit(savedParams);
```

## Configuration

Edit the target network in `useNetworkGuard.ts`:

```typescript
export const REQUIRED_CHAIN_ID = "0x1";  // Your target chain
export const REQUIRED_CHAIN_NAME = "Ethereum Mainnet";
```
