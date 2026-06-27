"use client";

import * as React from "react";
import { cn } from "@/lib/utils";
import {
  Copy,
  Check,
  ExternalLink,
  LogOut,
  Wallet,
} from "lucide-react";

export interface WalletDropdownProps {
  /** Connected wallet address. If null, copy/explorer actions are disabled. */
  address: string | null;
  /** Base URL used to build the explorer link. Defaults to Etherscan-style URL. */
  explorerBaseUrl?: string;
  /** Called when user clicks Disconnect Wallet. */
  onDisconnect: () => void;
  /** Optional extra class names for the trigger container. */
  className?: string;
}

function truncateAddress(addr: string): string {
  const normalized = addr.trim();
  if (normalized.length <= 10) return normalized;
  // Preserve prefix (0x) style while showing tail.
  if (normalized.startsWith("0x") || normalized.startsWith("0X")) {
    return `0x...${normalized.slice(-4)}`;
  }
  return `${normalized.slice(0, 2)}...${normalized.slice(-4)}`;
}

function getAvatarLabel(addr: string): string {
  const t = addr.trim();
  if (t.length === 0) return "W";
  const suffix = t.slice(-4).replace(/[^a-zA-Z0-9]/g, "");
  const seed = suffix.slice(-2) || t.slice(0, 2);
  return seed.toUpperCase();
}

function safeCopy(text: string): Promise<void> {
  return new Promise((resolve, reject) => {
    if (!navigator.clipboard?.writeText) {
      // Fallback
      try {
        const textarea = document.createElement("textarea");
        textarea.value = text;
        textarea.setAttribute("readonly", "true");
        textarea.style.position = "absolute";
        textarea.style.left = "-9999px";
        document.body.appendChild(textarea);
        textarea.select();
        const ok = document.execCommand("copy");
        document.body.removeChild(textarea);
        ok ? resolve() : reject(new Error("Copy failed"));
      } catch (e) {
        reject(e);
      }
      return;
    }

    navigator.clipboard
      .writeText(text)
      .then(() => resolve())
      .catch(reject);
  });
}

export function WalletDropdown({
  address,
  explorerBaseUrl = "https://etherscan.io/address/",
  onDisconnect,
  className,
}: WalletDropdownProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const [copied, setCopied] = React.useState(false);

  const buttonRef = React.useRef<HTMLButtonElement | null>(null);
  const panelRef = React.useRef<HTMLDivElement | null>(null);
  const closeTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(
    null
  );

  const close = React.useCallback(() => setIsOpen(false), []);
  const toggle = React.useCallback(() => setIsOpen((v) => !v), []);

  React.useEffect(() => {
    const onMouseDown = (event: MouseEvent) => {
      const target = event.target as Node | null;
      if (!target) return;

      const panelEl = panelRef.current;
      const buttonEl = buttonRef.current;

      if (panelEl && panelEl.contains(target)) return;
      if (buttonEl && buttonEl.contains(target)) return;

      close();
    };

    document.addEventListener("mousedown", onMouseDown);
    return () => document.removeEventListener("mousedown", onMouseDown);
  }, [close]);

  React.useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };

    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [close]);

  React.useEffect(() => {
    return () => {
      if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    };
  }, []);

  const handleCopy = async () => {
    if (!address) return;
    try {
      await safeCopy(address);
      setCopied(true);
      if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
      closeTimerRef.current = setTimeout(() => setCopied(false), 1500);
    } catch {
      // Silent: UI requirement only asks for temp checkmark on success.
    }
  };

  const explorerUrl = address
    ? `${explorerBaseUrl.replace(/\/$/, "")}/${address}`
    : null;

  const truncated = address ? truncateAddress(address) : "—";
  const avatarLabel = address ? getAvatarLabel(address) : "W";

  return (
    <div className={cn("relative inline-block", className)}>
      <button
        ref={buttonRef}
        type="button"
        onClick={toggle}
        aria-haspopup="menu"
        aria-expanded={isOpen ? "true" : "false"}

        className={cn(
          "group inline-flex items-center gap-2 rounded-full border border-input bg-background/60 px-3 py-2.5 text-left",
          "shadow-sm backdrop-blur transition-colors",
          "hover:bg-accent/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
          "active:bg-accent/80"
        )}
      >
        <span
          aria-hidden="true"
          className={cn(
            "flex h-7 w-7 items-center justify-center rounded-full",
            "bg-primary/10 text-primary",
            "border border-primary/20"
          )}
        >
          {address ? (
            <span className="text-[10px] font-semibold tracking-wide">
              {avatarLabel}
            </span>
          ) : (
            <Wallet className="h-4 w-4" />
          )}
        </span>

        <span className="flex min-w-0 items-center gap-2">
          <span className="truncate text-sm font-medium text-foreground">
            {address ? truncated : "Connect wallet"}
          </span>
          <span
            aria-hidden="true"
            className={cn(
              "ml-0.5 inline-block h-2 w-2 rounded-full",
              "bg-emerald-400 shadow-[0_0_0_3px_rgba(52,211,153,0.18)]",
              !address && "bg-zinc-500 shadow-[0_0_0_3px_rgba(113,113,122,0.18)]"
            )}
          />
        </span>
      </button>

      <div
        ref={panelRef}
        className={cn(
          "absolute right-0 mt-2 w-[16rem] sm:w-64 origin-top-right",
          "rounded-xl border border-input bg-background/90 shadow-xl backdrop-blur",
          "overflow-hidden",
          "transition-all duration-200 ease-in-out",
          isOpen
            ? "opacity-100 translate-y-0 scale-100 visible"
            : "opacity-0 translate-y-1 scale-[0.98] invisible pointer-events-none"
        )}
        role="menu"
        aria-label="Wallet actions"
      >
        <div className="p-3">
          <div className="flex items-center gap-3">
            <div
              aria-hidden="true"
              className={cn(
                "flex h-10 w-10 items-center justify-center rounded-full",
                address ? "bg-primary/10 text-primary" : "bg-zinc-500/10 text-zinc-400",
                "border border-primary/20"
              )}
            >
              {address ? (
                <span className="text-[11px] font-semibold tracking-wide">
                  {avatarLabel}
                </span>
              ) : (
                <Wallet className="h-5 w-5" />
              )}
            </div>

            <div className="min-w-0">
              <div className="text-xs font-medium text-muted-foreground">
                Connected Wallet
              </div>
              <div className="truncate text-sm font-semibold text-foreground">
                {address ? truncated : "Not connected"}
              </div>
            </div>
          </div>
        </div>

        <div className="px-2 pb-2">
          <div className="grid gap-1">
            <button
              type="button"
              role="menuitem"
              onClick={handleCopy}
              disabled={!address}

              className={cn(
                "group flex w-full items-center justify-between rounded-lg px-3 py-2",
                "transition-colors duration-150",
                "hover:bg-accent/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                "disabled:cursor-not-allowed disabled:opacity-60",
                "bg-transparent"
              )}
            >
              <span className="flex items-center gap-2">
                {copied ? (
                  <Check className="h-4 w-4 text-emerald-400 transition-all duration-200" />
                ) : (
                  <Copy className="h-4 w-4 text-muted-foreground transition-all duration-200 group-hover:text-foreground" />
                )}
                <span className="text-sm font-medium">
                  {copied ? "Copied" : "Copy Address"}
                </span>
              </span>

              <span className="text-xs text-muted-foreground">
                {address ? "Ctrl+C" : "—"}
              </span>
            </button>

            <a
              role="menuitem"
              aria-label="View in explorer"
              href={explorerUrl ?? undefined}

              target="_blank"
              rel="noreferrer noopener"
              onClick={(e) => {
                if (!explorerUrl) e.preventDefault();
                close();
              }}
              className={cn(
                "group flex items-center justify-between rounded-lg px-3 py-2",
                "transition-colors duration-150",
                "hover:bg-accent/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                "bg-transparent",
                !explorerUrl && "pointer-events-none opacity-60"
              )}
            >
              <span className="flex items-center gap-2">
                <ExternalLink className="h-4 w-4 text-muted-foreground transition-all duration-200 group-hover:text-foreground" />
                <span className="text-sm font-medium">View in Explorer</span>
              </span>
            </a>
          </div>
        </div>

        <div className="px-2 pb-2">
          <div className="mt-2 h-px bg-border" />

          <button
            type="button"
            role="menuitem"
            onClick={() => {
              close();
              onDisconnect();
            }}
            className={cn(
              "mt-2 flex w-full items-center justify-center gap-2 rounded-lg px-3 py-2.5",
              "bg-destructive/10 text-destructive",
              "hover:bg-destructive/20 hover:text-destructive",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
              "transition-colors duration-150"
            )}
          >
            <LogOut className="h-4 w-4" aria-hidden="true" />
            <span className="text-sm font-semibold">Disconnect Wallet</span>
          </button>
        </div>
      </div>
    </div>
  );
}

