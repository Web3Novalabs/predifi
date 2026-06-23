"use client";

import { useEffect, useState, useCallback } from "react";

/** Stellar / Soroban — no canonical EVM chain for this app.
 *  Set to the chain you actually target; null means "any chain is wrong
 *  unless ethereum is absent", which lets us show the modal only when a
 *  wallet IS connected but on the wrong chain. */
export const REQUIRED_CHAIN_ID = "0x1"; // Ethereum mainnet placeholder
export const REQUIRED_CHAIN_NAME = "Ethereum Mainnet";

export interface NetworkGuardState {
  /** True when wallet is connected on the wrong chain. */
  isWrongNetwork: boolean;
  /** Human-readable name of the chain the user is currently on. */
  currentChainName: string;
  /** Trigger the programmatic network switch; resolves when done or rejects on user cancel. */
  switchNetwork: () => Promise<void>;
  /** Error message from the last failed switch attempt. */
  switchError: string | null;
}

const CHAIN_NAMES: Record<string, string> = {
  "0x1": "Ethereum Mainnet",
  "0x38": "BNB Smart Chain",
  "0x89": "Polygon",
  "0xa4b1": "Arbitrum One",
  "0xa": "Optimism",
  "0x2105": "Base",
  "0xe708": "Linea",
  "0xaa36a7": "Sepolia Testnet",
  "0x13882": "Polygon Amoy",
};

function chainName(hexId: string): string {
  return CHAIN_NAMES[hexId.toLowerCase()] ?? `Chain ${parseInt(hexId, 16)}`;
}

export function useNetworkGuard(): NetworkGuardState {
  const [currentChainId, setCurrentChainId] = useState<string | null>(null);
  const [switchError, setSwitchError] = useState<string | null>(null);

  useEffect(() => {
    const eth = window.ethereum;
    if (!eth) return;

    // Read current chain on mount
    eth.request({ method: "eth_chainId" }).then(setCurrentChainId).catch(() => {});

    const handler = (chainId: string) => setCurrentChainId(chainId);
    eth.on("chainChanged", handler);
    return () => eth.removeListener("chainChanged", handler);
  }, []);

  const isWrongNetwork =
    currentChainId !== null &&
    currentChainId.toLowerCase() !== REQUIRED_CHAIN_ID.toLowerCase();

  const switchNetwork = useCallback(async () => {
    const eth = window.ethereum;
    if (!eth) return;
    setSwitchError(null);
    try {
      await eth.request({
        method: "wallet_switchEthereumChain",
        params: [{ chainId: REQUIRED_CHAIN_ID }],
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Network switch failed.";
      setSwitchError(msg);
    }
  }, []);

  return {
    isWrongNetwork,
    currentChainName: currentChainId ? chainName(currentChainId) : "",
    switchNetwork,
    switchError,
  };
}
