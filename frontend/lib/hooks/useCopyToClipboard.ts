"use client";

import { useCallback, useRef, useState } from "react";
import { useToastActions } from "@/components/ui";

export interface CopyToClipboardOptions {
  /** Toast title shown on success. Defaults to "Copied!". */
  successTitle?: string;
  /** Toast description shown on success. */
  successDescription?: string;
  /** Toast title shown on failure. Defaults to "Copy failed". */
  errorTitle?: string;
  /** Toast description shown on failure. */
  errorDescription?: string;
  /**
   * How long (ms) the `copied` state stays true before resetting.
   * This drives the visual feedback on the button itself (icon swap, etc.).
   * Defaults to 2000.
   */
  resetDelay?: number;
}

export interface UseCopyToClipboardReturn {
  /** Call this with the text to copy. */
  copy: (text: string) => Promise<void>;
  /** True for `resetDelay` ms after a successful copy. */
  copied: boolean;
  /** True while the clipboard write is in-flight. */
  isPending: boolean;
}

/**
 * useCopyToClipboard
 *
 * Copies a string to the clipboard and fires a toast notification to confirm
 * the action to the user.
 *
 * Requires `ToastProvider` to be present in the component tree (it is mounted
 * in the root layout).
 *
 * @example
 * const { copy, copied } = useCopyToClipboard({ successDescription: "Address copied" });
 * return (
 *   <button onClick={() => copy(address)}>
 *     {copied ? <CheckCircle2 /> : <Copy />}
 *   </button>
 * );
 */
export function useCopyToClipboard({
  successTitle = "Copied!",
  successDescription,
  errorTitle = "Copy failed",
  errorDescription = "Could not access the clipboard. Please copy manually.",
  resetDelay = 2000,
}: CopyToClipboardOptions = {}): UseCopyToClipboardReturn {
  const { addToast } = useToastActions();
  const [copied, setCopied] = useState(false);
  const [isPending, setIsPending] = useState(false);
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const copy = useCallback(
    async (text: string) => {
      if (isPending) return;

      setIsPending(true);

      try {
        await navigator.clipboard.writeText(text);

        // Clear any existing reset timer so rapid clicks don't conflict
        if (resetTimer.current) clearTimeout(resetTimer.current);

        setCopied(true);
        addToast({
          variant: "success",
          title: successTitle,
          description: successDescription,
          duration: 3000,
        });

        resetTimer.current = setTimeout(() => {
          setCopied(false);
          resetTimer.current = null;
        }, resetDelay);
      } catch {
        addToast({
          variant: "error",
          title: errorTitle,
          description: errorDescription,
          duration: 5000,
        });
      } finally {
        setIsPending(false);
      }
    },
    [
      isPending,
      addToast,
      successTitle,
      successDescription,
      errorTitle,
      errorDescription,
      resetDelay,
    ],
  );

  return { copy, copied, isPending };
}
