"use client";

import type { ReactNode } from "react";
import { useNetworkGuard } from "@/lib/hooks/useNetworkGuard";
import { NetworkSwitchModal } from "@/components/modals/NetworkSwitchModal";

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
      <NetworkSwitchModal
        isOpen={isWrongNetwork}
        currentChainName={currentChainName}
        onSwitch={switchNetwork}
        switchError={switchError}
        switchRecoveryAction={switchRecoveryAction}
      />
    </>
  );
}
