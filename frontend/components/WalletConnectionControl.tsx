"use client";

import { useWallet } from "@/lib/context";
import { WalletErrorBanner } from "@/components/WalletErrorBanner";
import { Button } from "@/components/ui";

export function WalletConnectionControl() {
  const { address, connect, disconnect, isConnecting, error, clearError } = useWallet();

  return (
    <div className="relative">
      {address ? (
        <Button
          type="button"
          variant="secondary"
          onClick={disconnect}
          aria-label="Disconnect wallet"
        >
          {`${address.slice(0, 5)}...${address.slice(-4)}`}
        </Button>
      ) : (
        <Button
          type="button"
          onClick={() => void connect()}
          disabled={isConnecting}
          aria-busy={isConnecting}
        >
          {isConnecting ? "Connecting..." : "Connect wallet"}
        </Button>
      )}
      {error && (
        <WalletErrorBanner
          error={error}
          onRetry={() => void connect()}
          onDismiss={clearError}
          className="absolute right-0 top-full z-50 mt-2 w-[min(24rem,90vw)]"
        />
      )}
    </div>
  );
}
