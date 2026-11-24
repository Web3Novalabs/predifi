"use client";

import { useEffect, useRef } from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui";
import { cn } from "@/lib/utils";
import { REQUIRED_CHAIN_NAME } from "@/lib/hooks/useNetworkGuard";

interface NetworkSwitchModalProps {
  isOpen: boolean;
  currentChainName: string;
  onSwitch: () => Promise<void>;
  switchError: string | null;
  switchRecoveryAction?: string | null;
}

export function NetworkSwitchModal({
  isOpen,
  currentChainName,
  onSwitch,
  switchError,
  switchRecoveryAction,
}: NetworkSwitchModalProps) {
  const switchBtnRef = useRef<HTMLButtonElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;

    const previousFocus =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    switchBtnRef.current?.focus();

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        return;
      }
      if (event.key !== "Tab") return;

      const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
        'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])',
      );
      if (!focusable?.length) {
        event.preventDefault();
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (
        event.shiftKey &&
        (document.activeElement === first ||
          !dialogRef.current?.contains(document.activeElement))
      ) {
        event.preventDefault();
        last.focus();
      } else if (
        !event.shiftKey &&
        (document.activeElement === last ||
          !dialogRef.current?.contains(document.activeElement))
      ) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      previousFocus?.focus();
    };
  }, [isOpen]);

  if (!isOpen) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="nsm-title"
      aria-describedby="nsm-desc"
      ref={dialogRef}
      className={cn(
        "fixed inset-0 z-50 flex items-center justify-center p-4",
        "bg-black/70 backdrop-blur-sm",
        "animate-fade-in"
      )}
    >
      <div
        className={cn(
          "w-full max-w-sm rounded-2xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl",
          "animate-fade-in"
        )}
        // Prevent backdrop click from doing anything (network switch is required)
        onClick={(e) => e.stopPropagation()}
      >
        {/* Icon */}
        <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-yellow-500/10">
          <AlertTriangle className="h-6 w-6 text-yellow-400" aria-hidden="true" />
        </div>

        {/* Heading */}
        <h2 id="nsm-title" className="text-lg font-semibold text-white">
          Wrong Network
        </h2>

        {/* Body */}
        <p id="nsm-desc" className="mt-2 text-sm text-zinc-400">
          Your wallet is connected to{" "}
          <span className="font-medium text-white">
            {currentChainName || "an unsupported network"}
          </span>
          . Please switch to{" "}
          <span className="font-medium text-white">{REQUIRED_CHAIN_NAME}</span>{" "}
          to continue.
        </p>

        {/* Error feedback */}
        {switchError && (
          <div role="alert" className="mt-3 space-y-1">
            <p className="text-xs text-red-400">{switchError}</p>
            {switchRecoveryAction && (
              <p className="text-xs text-zinc-400">{switchRecoveryAction}</p>
            )}
          </div>
        )}

        {/* CTA */}
        <Button
          ref={switchBtnRef}
          onClick={onSwitch}
          size="medium"
          className="mt-5 w-full"
        >
          Switch Network
        </Button>
      </div>
    </div>
  );
}
