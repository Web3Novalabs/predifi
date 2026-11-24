"use client";

import { useCallback, useState } from "react";
import {
  classifyWalletError,
  checkNetworkMatch,
  isStellarWalletInstalled,
  makeWalletError,
  withWalletTimeout,
  type WalletError,
  REQUIRED_STELLAR_NETWORK,
  STELLAR_NETWORK_LABELS,
} from "@/lib/walletErrors";

export interface WalletConnectionState {
  address: string | null;
  isConnecting: boolean;
  error: WalletError | null;
  requiredNetworkLabel: string;
  connect: () => Promise<string | null>;
  disconnect: () => void;
  clearError: () => void;
}

type FreighterLike = {
  isConnected?: () => Promise<boolean>;
  getNetwork?: () => Promise<string>;
  getNetworkDetails?: () => Promise<{
    network?: string;
    networkPassphrase?: string;
  }>;
  getPublicKey?: () => Promise<string>;
  requestAccess?: () => Promise<string>;
};

function getFreighterApi(): FreighterLike | null {
  if (typeof window === "undefined") return null;
  const w = window as Window & {
    freighterApi?: FreighterLike;
    freighter?: FreighterLike;
  };
  return w.freighterApi ?? w.freighter ?? null;
}

/**
 * Connect / disconnect with structured error handling for:
 * extension missing, network mismatch, user rejection, timeout.
 */
export function useWalletConnection(): WalletConnectionState {
  const [address, setAddress] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<WalletError | null>(null);

  const clearError = useCallback(() => setError(null), []);

  const disconnect = useCallback(() => {
    setAddress(null);
    setError(null);
  }, []);

  const connect = useCallback(async (): Promise<string | null> => {
    setIsConnecting(true);
    setError(null);

    try {
      if (!isStellarWalletInstalled()) {
        const err = makeWalletError("EXTENSION_NOT_INSTALLED");
        setError(err);
        return null;
      }

      const api = getFreighterApi();
      if (!api) {
        const err = makeWalletError("EXTENSION_NOT_INSTALLED");
        setError(err);
        return null;
      }

      // Network check before requesting access
      try {
        const networkDetails = api.getNetworkDetails
          ? await withWalletTimeout(api.getNetworkDetails(), 60_000)
          : null;
        const network =
          networkDetails?.network ??
          networkDetails?.networkPassphrase ??
          (api.getNetwork ? await withWalletTimeout(api.getNetwork(), 60_000) : null) ??
          null;
        const mismatch = checkNetworkMatch(network);
        if (mismatch) {
          setError(mismatch);
          return null;
        }
      } catch (netErr) {
        setError(classifyWalletError(netErr));
        return null;
      }

      const publicKey = await withWalletTimeout(
        (async () => {
          if (api.requestAccess) {
            return api.requestAccess();
          }
          if (api.getPublicKey) {
            return api.getPublicKey();
          }
          throw makeWalletError("EXTENSION_NOT_INSTALLED");
        })(),
        60_000,
      );

      if (!publicKey || typeof publicKey !== "string") {
        const err = makeWalletError("USER_REJECTED");
        setError(err);
        return null;
      }

      setAddress(publicKey);
      return publicKey;
    } catch (err) {
      setError(classifyWalletError(err));
      return null;
    } finally {
      setIsConnecting(false);
    }
  }, []);

  return {
    address,
    isConnecting,
    error,
    requiredNetworkLabel: STELLAR_NETWORK_LABELS[REQUIRED_STELLAR_NETWORK],
    connect,
    disconnect,
    clearError,
  };
}

/**
 * Classify a transaction submission failure (balance, timeout, reject).
 * Use around contract invoke / payment submits.
 */
export function handleTransactionError(err: unknown): WalletError {
  return classifyWalletError(err);
}
