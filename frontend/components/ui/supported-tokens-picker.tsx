"use client";

import React, { useState, useRef, useEffect } from "react";
import { ChevronDown, Check } from "lucide-react";
import { cn } from "@/lib/utils";

export interface Token {
  id: string;
  symbol: string;
  name: string;
  icon?: React.ReactNode;
}

const DEFAULT_SUPPORTED_TOKENS: Token[] = [
  { id: "XLM", symbol: "XLM", name: "Stellar Lumens" },
  { id: "STRK", symbol: "STRK", name: "Stark" },
];

export interface SupportedTokensPickerProps {
  /** Selected token ID */
  value?: string;
  /** Callback when token is selected */
  onChange?: (token: Token) => void;
  /** Custom list of supported tokens (defaults to [XLM, STRK]) */
  tokens?: Token[];
  /** Custom class names */
  className?: string;
  /** Disabled state */
  disabled?: boolean;
}

export function SupportedTokensPicker({
  value = "XLM",
  onChange,
  tokens = DEFAULT_SUPPORTED_TOKENS,
  className,
  disabled = false,
}: SupportedTokensPickerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedToken = tokens.find((t) => t.id === value) || tokens[0];

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, []);

  return (
    <div className={cn("relative", className)} ref={containerRef}>
      <button
        type="button"
        onClick={() => !disabled && setIsOpen(!isOpen)}
        disabled={disabled}
        className={cn(
          "flex items-center gap-2 rounded-lg border border-input bg-background px-3 py-2 text-sm font-medium transition-all duration-200",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
          "hover:bg-accent",
          disabled && "cursor-not-allowed opacity-50",
          isOpen && "ring-2 ring-ring ring-offset-2"
        )}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-label="Select token"
      >
        {selectedToken.icon}
        <span className="text-foreground">{selectedToken.symbol}</span>
        <ChevronDown
          className={cn(
            "h-4 w-4 text-muted-foreground transition-transform duration-200",
            isOpen && "rotate-180"
          )}
          aria-hidden="true"
        />
      </button>

      {isOpen && (
        <div
          className="absolute top-full left-0 z-50 mt-2 w-48 rounded-lg border border-input bg-background shadow-lg"
          role="listbox"
          aria-label="Supported tokens"
        >
          <div className="py-1">
            {tokens.map((token) => (
              <button
                key={token.id}
                type="button"
                onClick={() => {
                  onChange?.(token);
                  setIsOpen(false);
                }}
                className={cn(
                  "flex w-full items-center gap-3 px-3 py-2 text-sm transition-colors",
                  "hover:bg-accent",
                  "focus-visible:outline-none focus-visible:bg-accent",
                  token.id === value && "bg-accent"
                )}
                role="option"
                aria-selected={token.id === value}
              >
                {token.icon}
                <div className="flex-1 text-left">
                  <div className="font-medium text-foreground">
                    {token.symbol}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {token.name}
                  </div>
                </div>
                {token.id === value && (
                  <Check
                    className="h-4 w-4 text-primary"
                    aria-hidden="true"
                  />
                )}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
