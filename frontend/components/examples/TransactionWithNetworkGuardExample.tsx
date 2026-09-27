/**
 * Example component demonstrating proper integration of:
 * 1. Network guard with pre-submission checks
 * 2. Transaction progress with retry functionality
 * 
 * This component shows the complete pattern for safely submitting
 * transactions with network validation and recovery.
 */

"use client";

import { useState } from "react";
import { useNetworkGuard } from "@/lib/hooks/useNetworkGuard";
import { TransactionProgress, type TxStatus } from "@/components/ui/transaction-progress";
import { Button } from "@/components/ui/button";

interface TransactionParams {
  to: string;
  amount: string;
  data?: string;
}

export function TransactionWithNetworkGuardExample() {
  const { checkNetworkBeforeSubmit } = useNetworkGuard();
  
  const [txStatus, setTxStatus] = useState<TxStatus>("idle");
  const [txHash, setTxHash] = useState<string>();
  const [txError, setTxError] = useState<string>();
  const [isRetrying, setIsRetrying] = useState(false);
  
  // Store original transaction parameters for retry
  const [savedTxParams, setSavedTxParams] = useState<TransactionParams | null>(null);

  /**
   * Submit a transaction with proper network validation
   */
  const submitTransaction = async (params: TransactionParams) => {
    // ===================================================================
    // CRITICAL: Check network immediately before submission
    // This catches mid-session network switches
    // ===================================================================
    const networkError = await checkNetworkBeforeSubmit();
    
    if (networkError) {
      // Block submission and show clear error with both networks
      setTxStatus("failed");
      setTxError(networkError.message);
      setTxHash(undefined);
      return;
    }

    // Save parameters for potential retry
    setSavedTxParams(params);
    
    // Reset state for new attempt
    setTxStatus("submitted");
    setTxError(undefined);
    setTxHash(undefined);

    try {
      // Simulate transaction submission
      // Replace with actual wallet/blockchain call
      const hash = await mockSendTransaction(params);
      
      setTxHash(hash);
      setTxStatus("processing");
      
      // Wait for confirmation
      await mockWaitForConfirmation(hash);
      
      setTxStatus("confirmed");
      
      // Clear saved params after successful confirmation
      setSavedTxParams(null);
      
    } catch (error: any) {
      setTxStatus("failed");
      setTxError(error.message || "Transaction failed. Please try again.");
      // Keep savedTxParams for retry
    }
  };

  /**
   * Retry handler - reuses original transaction parameters
   * without requiring user to re-enter form data
   */
  const handleRetry = async () => {
    if (!savedTxParams) {
      console.error("No saved transaction parameters for retry");
      return;
    }

    setIsRetrying(true);
    
    // Re-submit with exact same parameters
    await submitTransaction(savedTxParams);
    
    setIsRetrying(false);
  };

  /**
   * Form submission handler
   */
  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    
    const formData = new FormData(e.currentTarget);
    const params: TransactionParams = {
      to: formData.get("to") as string,
      amount: formData.get("amount") as string,
    };
    
    submitTransaction(params);
  };

  return (
    <div className="max-w-2xl mx-auto p-6 space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-bold text-white">Send Transaction</h2>
        <p className="text-sm text-zinc-400">
          Example showing network validation and retry functionality
        </p>
      </div>

      {/* Transaction Form */}
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="space-y-2">
          <label htmlFor="to" className="text-sm font-medium text-white">
            Recipient Address
          </label>
          <input
            type="text"
            id="to"
            name="to"
            required
            placeholder="0x..."
            className="w-full px-3 py-2 bg-white/5 border border-white/10 rounded-lg text-white placeholder:text-zinc-600 focus:outline-none focus:border-[#37B7C3]"
          />
        </div>

        <div className="space-y-2">
          <label htmlFor="amount" className="text-sm font-medium text-white">
            Amount
          </label>
          <input
            type="text"
            id="amount"
            name="amount"
            required
            placeholder="0.0"
            className="w-full px-3 py-2 bg-white/5 border border-white/10 rounded-lg text-white placeholder:text-zinc-600 focus:outline-none focus:border-[#37B7C3]"
          />
        </div>

        <Button
          type="submit"
          disabled={txStatus === "submitted" || txStatus === "processing"}
          className="w-full h-10 rounded-lg font-medium bg-[#37B7C3] hover:bg-[#37B7C3]/90 text-[#001112] disabled:opacity-50"
        >
          {txStatus === "submitted" || txStatus === "processing" ? "Processing..." : "Send Transaction"}
        </Button>
      </form>

      {/* Transaction Progress with Retry */}
      <TransactionProgress
        status={txStatus}
        txHash={txHash}
        errorMessage={txError}
        onRetry={handleRetry}
        isRetrying={isRetrying}
      />

      {/* Integration Notes */}
      <div className="rounded-lg bg-white/5 border border-white/10 p-4 space-y-2">
        <h3 className="text-sm font-semibold text-white">Integration Notes</h3>
        <ul className="text-xs text-zinc-400 space-y-1 list-disc list-inside">
          <li>Network is checked immediately before each submission</li>
          <li>Submission is blocked on network mismatch with clear error</li>
          <li>Error message includes both current and required network names</li>
          <li>Retry button reuses original parameters (no form re-entry)</li>
          <li>Retry performs fresh network check</li>
          <li>Progress component instance is reused (no duplicate entries)</li>
        </ul>
      </div>
    </div>
  );
}

// ===================================================================
// Mock functions - replace with actual blockchain calls
// ===================================================================

async function mockSendTransaction(params: TransactionParams): Promise<string> {
  await new Promise(resolve => setTimeout(resolve, 1000));
  
  // Simulate occasional failures for testing retry
  if (Math.random() < 0.3) {
    throw new Error("RPC error: insufficient funds for gas");
  }
  
  return `0x${Math.random().toString(16).substring(2)}`;
}

async function mockWaitForConfirmation(txHash: string): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, 2000));
  
  // Simulate occasional confirmation failures
  if (Math.random() < 0.2) {
    throw new Error("Transaction reverted: execution failed");
  }
}
