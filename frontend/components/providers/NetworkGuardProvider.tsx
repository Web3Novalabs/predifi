"use client";

import type { ReactNode } from "react";
import dynamic from "next/dynamic";
import { useNetworkGuard } from "@/lib/hooks/useNetworkGuard";

// The modal is only ever visible on the wrong network, so keep it (and the
// UI barrel it pulls in) out of the root layout's initial bundle (#2206).
const NetworkSwitchModal = dynamic(
  () =>
    import("@/components/modals/NetworkSwitchModal").then(
      (mod) => mod.NetworkSwitchModal,
    ),
  { ssr: false },
);

export function NetworkGuardProvider({ children }: { children: ReactNode }) {
  const {
    isWrongNetwork,
    currentChainName,
    switchNetwork,
    switchError,
    switchRecoveryAction,
  } = useNetworkGuard();

  return (
    <>
      {children}
      {isWrongNetwork && (
        <NetworkSwitchModal
          isOpen={isWrongNetwork}
          currentChainName={currentChainName}
          onSwitch={switchNetwork}
          switchError={switchError}
          switchRecoveryAction={switchRecoveryAction}
        />
      )}
    </>
  );
}
