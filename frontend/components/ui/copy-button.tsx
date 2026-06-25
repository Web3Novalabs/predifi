"use client";

import * as React from "react";
import { Copy, CheckCircle2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { useCopyToClipboard } from "@/lib/hooks/useCopyToClipboard";
import type { CopyToClipboardOptions } from "@/lib/hooks/useCopyToClipboard";

export interface CopyButtonProps
  extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "onClick"> {
  /** The text to copy to the clipboard. */
  text: string;
  /**
   * Visual size of the icons.
   * @default "sm"
   */
  size?: "xs" | "sm" | "md";
  /** Override the default toast / reset-delay options. */
  copyOptions?: CopyToClipboardOptions;
}

const sizeClasses = {
  xs: "w-3 h-3",
  sm: "w-3.5 h-3.5",
  md: "w-4 h-4",
};

/**
 * CopyButton
 *
 * An icon-only button that copies `text` to the clipboard and fires a toast
 * notification confirming the action.
 *
 * The icon swaps from Copy → CheckCircle2 for `resetDelay` ms (default 2 s)
 * so the user gets immediate visual confirmation in addition to the toast.
 *
 * @example
 * // Minimal
 * <CopyButton text={address} />
 *
 * // With custom toast message and aria label
 * <CopyButton
 *   text={referralId}
 *   aria-label="Copy referral ID"
 *   copyOptions={{ successDescription: `${referralId} copied` }}
 * />
 */
export const CopyButton = React.forwardRef<HTMLButtonElement, CopyButtonProps>(
  ({ text, size = "sm", copyOptions, className, disabled, "aria-label": ariaLabel, ...props }, ref) => {
    const { copy, copied, isPending } = useCopyToClipboard(copyOptions);

    const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
      e.stopPropagation();
      void copy(text);
    };

    const iconClass = sizeClasses[size];
    const label = ariaLabel ?? (copied ? "Copied" : "Copy to clipboard");

    return (
      <button
        ref={ref}
        type="button"
        onClick={handleClick}
        disabled={disabled || isPending}
        aria-label={label}
        aria-pressed={copied}
        className={cn(
          "inline-flex items-center justify-center rounded transition-colors",
          "text-zinc-500 hover:text-white",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
          "disabled:pointer-events-none disabled:opacity-50",
          copied && "text-emerald-400 hover:text-emerald-300",
          className,
        )}
        {...props}
      >
        {copied ? (
          <CheckCircle2 className={cn(iconClass, "transition-all duration-200")} aria-hidden="true" />
        ) : (
          <Copy className={cn(iconClass, "transition-all duration-200")} aria-hidden="true" />
        )}
      </button>
    );
  },
);

CopyButton.displayName = "CopyButton";
