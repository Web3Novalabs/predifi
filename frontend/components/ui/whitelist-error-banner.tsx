import React from "react";
import { X, Lock } from "lucide-react";
import { cn } from "@/lib/utils";

export interface WhitelistErrorBannerProps {
  /** Custom class names */
  className?: string;
  /** Custom message (defaults to "This pool is private. You are not on the whitelist.") */
  message?: string;
  /** Callback when the dismiss button is clicked */
  onDismiss?: () => void;
}

export function WhitelistErrorBanner({
  className,
  message = "This pool is private. You are not on the whitelist.",
  onDismiss,
}: WhitelistErrorBannerProps) {
  return (
    <div
      className={cn(
        "flex items-center gap-3 rounded-lg border border-red-800 bg-red-900/20 p-4",
        className
      )}
      role="alert"
      aria-live="assertive"
    >
      <Lock className="h-5 w-5 text-red-400 flex-shrink-0" aria-hidden="true" />
      <p className="text-sm text-red-400 flex-1">{message}</p>
      {onDismiss && (
        <button
          type="button"
          onClick={onDismiss}
          className="p-1 text-red-400 hover:text-red-300 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500 focus-visible:ring-offset-2"
          aria-label="Dismiss banner"
        >
          <X className="h-4 w-4" aria-hidden="true" />
        </button>
      )}
    </div>
  );
}
