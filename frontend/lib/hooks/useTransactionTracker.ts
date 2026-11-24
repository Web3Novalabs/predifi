"use client";

import { useEffect, useState } from "react";
import {
  REQUIRED_STELLAR_NETWORK,
  type StellarNetwork,
} from "@/lib/walletErrors";

export type TrackedTransactionStatus =
  | "idle"
  | "submitted"
  | "pending"
  | "confirmed"
  | "failed"
  | "timeout";

export interface TransactionTrackerState {
  status: TrackedTransactionStatus;
  elapsedMs: number;
  errorMessage?: string;
}

interface TransactionTrackerOptions {
  network?: StellarNetwork;
  pollIntervalMs?: number;
  timeoutMs?: number;
}

const RPC_ENDPOINTS: Record<StellarNetwork, string> = {
  TESTNET: "https://soroban-testnet.stellar.org",
  PUBLIC: "https://soroban.stellar.org",
  FUTURENET: "https://rpc-futurenet.stellar.org",
};

const STELLAR_RPC_URL =
  process.env.NEXT_PUBLIC_STELLAR_RPC_URL?.replace(/\/$/, "");

type RpcTransaction = {
  status?: string;
  errorResultXdr?: string;
};

async function getTransaction(
  endpoint: string,
  hash: string,
  signal: AbortSignal,
): Promise<RpcTransaction> {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: hash,
      method: "getTransaction",
      params: { hash },
    }),
    signal,
  });

  if (!response.ok) {
    throw new Error(`Transaction status request failed (HTTP ${response.status})`);
  }

  const payload = (await response.json()) as {
    error?: { message?: string };
    result?: RpcTransaction;
  };
  if (payload.error) {
    throw new Error(payload.error.message ?? "Transaction status request failed");
  }
  if (!payload.result || typeof payload.result.status !== "string") {
    throw new Error("Transaction status response was invalid");
  }
  return payload.result;
}

export function useTransactionTracker(
  txHash: string | undefined,
  {
    network = REQUIRED_STELLAR_NETWORK,
    pollIntervalMs = 4_000,
    timeoutMs = 120_000,
  }: TransactionTrackerOptions = {},
): TransactionTrackerState {
  const [state, setState] = useState<TransactionTrackerState>({
    status: txHash ? "submitted" : "idle",
    elapsedMs: 0,
  });

  useEffect(() => {
    if (!txHash) {
      setState({ status: "idle", elapsedMs: 0 });
      return;
    }

    const controller = new AbortController();
    const startedAt = Date.now();
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let elapsedTimer: ReturnType<typeof setInterval> | undefined;
    let timeoutTimer: ReturnType<typeof setTimeout> | undefined;

    const stopTimers = () => {
      if (elapsedTimer) clearInterval(elapsedTimer);
      if (timeoutTimer) clearTimeout(timeoutTimer);
      if (timer) clearTimeout(timer);
    };

    setState({ status: "submitted", elapsedMs: 0 });

    elapsedTimer = setInterval(() => {
      if (active) {
        setState((current) => ({
          ...current,
          elapsedMs: Date.now() - startedAt,
        }));
      }
    }, 1_000);

    timeoutTimer = setTimeout(() => {
      if (!active) return;
      active = false;
      controller.abort();
      stopTimers();
      setState((current) => ({
        ...current,
        status: "timeout",
        elapsedMs: Date.now() - startedAt,
        errorMessage: undefined,
      }));
    }, timeoutMs);

    const poll = async () => {
      while (active) {
        try {
          const transaction = await getTransaction(
            STELLAR_RPC_URL || RPC_ENDPOINTS[network],
            txHash,
            controller.signal,
          );
          if (!active) return;

          const status = transaction.status?.toUpperCase();
          if (status === "SUCCESS") {
            active = false;
            stopTimers();
            setState({
              status: "confirmed",
              elapsedMs: Date.now() - startedAt,
            });
            return;
          }
          if (status === "FAILED") {
            active = false;
            stopTimers();
            setState({
              status: "failed",
              elapsedMs: Date.now() - startedAt,
              errorMessage:
                transaction.errorResultXdr ??
                "The network rejected this transaction.",
            });
            return;
          }

          setState((current) => ({
            ...current,
            status: "pending",
            elapsedMs: Date.now() - startedAt,
            errorMessage: undefined,
          }));
        } catch (error) {
          if (!active || controller.signal.aborted) return;
          setState((current) => ({
            ...current,
            status: "pending",
            elapsedMs: Date.now() - startedAt,
            errorMessage:
              error instanceof Error
                ? "Status lookup is temporarily unavailable; checking again."
                : undefined,
          }));
        }

        await new Promise<void>((resolve) => {
          timer = setTimeout(resolve, pollIntervalMs);
        });
      }
    };

    void poll();
    return () => {
      active = false;
      controller.abort();
      stopTimers();
    };
  }, [network, pollIntervalMs, timeoutMs, txHash]);

  return state;
}
