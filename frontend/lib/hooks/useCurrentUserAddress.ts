"use client";

/**
 * useCurrentUserAddress — returns the connected wallet address.
 *
 * Migration (issue #1406):
 *   Previously returned a hardcoded placeholder. Now reads the real address
 *   from WalletContext so all consumers automatically receive `undefined` when
 *   disconnected and the real address after the user connects — no callsite
 *   changes required.
 *
 * Falls back gracefully to `undefined` (no throw) when WalletProvider is not
 * mounted — e.g. marketing pages or isolated unit tests.
 */

import { useContext } from "react";
// Import the context object directly (not through the barrel) to avoid a
// circular dependency: barrel → WalletContext → (re-exported) → barrel.
import { WalletContext } from "@/lib/context/WalletContext";

export function useCurrentUserAddress(): string | undefined {
  const ctx = useContext(WalletContext);
  return ctx?.address ?? undefined;
}
