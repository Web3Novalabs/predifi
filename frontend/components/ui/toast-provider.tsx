"use client";

/**
 * ToastProvider & hooks
 *
 * Changes from the previous version
 * ───────────────────────────────────
 * • `position` prop on ToastProvider controls where the stack appears.
 *   Supported positions: top-right (default), top-left, top-center,
 *   bottom-right, bottom-left, bottom-center.
 * • `maxToasts` prop caps how many toasts are visible at once (default 5).
 *   Oldest toasts are removed when the cap is exceeded.
 * • `aria-live` is now owned by the ToastList container only.
 *   Individual toasts no longer carry aria-live to avoid double-announcing.
 * • `aria-live="assertive"` for error/warning positions, "polite" for the rest
 *   — driven by the most-urgent variant currently in the stack.
 * • The ToastList container is now responsive: full-width with gutters on
 *   mobile, max-w-sm on ≥sm breakpoints.
 * • Toasts stack newest-on-top by reversing the list before rendering.
 */

import * as React from "react";
import { cn } from "@/lib/utils";
import { Toast, type ToastProps } from "./toast";

// ─── Types ────────────────────────────────────────────────────────────────────

export type ToastPosition =
  | "top-right"
  | "top-left"
  | "top-center"
  | "bottom-right"
  | "bottom-left"
  | "bottom-center";

type ToastActions = {
  addToast: (toast: Omit<ToastProps, "id" | "onClose">) => void;
  removeToast: (id: string) => void;
};

// ─── Contexts ─────────────────────────────────────────────────────────────────
// Split so addToast/removeToast callers never re-render on list changes.

const ToastStateContext = React.createContext<ToastProps[] | undefined>(
  undefined,
);
const ToastActionsContext = React.createContext<ToastActions | undefined>(
  undefined,
);

// ─── Position class map ───────────────────────────────────────────────────────

const positionClasses: Record<ToastPosition, string> = {
  "top-right":    "top-4 right-4 items-end",
  "top-left":     "top-4 left-4  items-start",
  "top-center":   "top-4 left-1/2 -translate-x-1/2 items-center",
  "bottom-right": "bottom-4 right-4 items-end",
  "bottom-left":  "bottom-4 left-4  items-start",
  "bottom-center":"bottom-4 left-1/2 -translate-x-1/2 items-center",
};

// ─── Provider ─────────────────────────────────────────────────────────────────

export interface ToastProviderProps {
  children: React.ReactNode;
  /**
   * Where the toast stack appears on-screen.
   * @default "top-right"
   */
  position?: ToastPosition;
  /**
   * Maximum number of toasts visible simultaneously.
   * When the cap is reached the oldest toast is removed before adding the new one.
   * @default 5
   */
  maxToasts?: number;
}

export function ToastProvider({
  children,
  position = "top-right",
  maxToasts = 5,
}: ToastProviderProps) {
  const [toasts, setToasts] = React.useState<ToastProps[]>([]);

  const removeToast = React.useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const addToast = React.useCallback(
    (toast: Omit<ToastProps, "id" | "onClose">) => {
      const id = crypto.randomUUID
        ? crypto.randomUUID()
        : Math.random().toString(36).slice(2, 11);

      setToasts((prev) => {
        // Drop the oldest toast when the cap is reached
        const trimmed =
          prev.length >= maxToasts ? prev.slice(-(maxToasts - 1)) : prev;
        return [...trimmed, { ...toast, id }];
      });
    },
    [maxToasts],
  );

  const actions = React.useMemo(
    () => ({ addToast, removeToast }),
    [addToast, removeToast],
  );

  return (
    <ToastActionsContext.Provider value={actions}>
      <ToastStateContext.Provider value={toasts}>
        {children}
        <ToastList
          toasts={toasts}
          removeToast={removeToast}
          position={position}
        />
      </ToastStateContext.Provider>
    </ToastActionsContext.Provider>
  );
}

// ─── ToastList ────────────────────────────────────────────────────────────────

function ToastList({
  toasts,
  removeToast,
  position,
}: {
  toasts: ToastProps[];
  removeToast: (id: string) => void;
  position: ToastPosition;
}) {
  // Use "assertive" if any visible toast is an error or warning
  const urgency =
    toasts.some((t) => t.variant === "error" || t.variant === "warning")
      ? "assertive"
      : "polite";

  // Render newest on top for top-* positions, newest at bottom for bottom-*
  const ordered = position.startsWith("bottom") ? toasts : [...toasts].reverse();

  if (ordered.length === 0) return null;

  return (
    <div
      aria-live={urgency}
      aria-label="Notifications"
      className={cn(
        // Base: fixed, full-bleed on mobile then capped at sm breakpoint
        "fixed z-50 flex flex-col gap-2 pointer-events-none",
        "w-[calc(100%-2rem)] sm:w-full sm:max-w-sm",
        positionClasses[position],
      )}
    >
      {ordered.map((toast) => (
        <Toast
          key={toast.id}
          {...toast}
          onClose={() => removeToast(toast.id)}
        />
      ))}
    </div>
  );
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

/**
 * useToastActions
 *
 * Returns stable `addToast` and `removeToast` functions.
 * Components that only fire toasts (never render them) should prefer this hook
 * over `useToast` because it does NOT cause re-renders when the toast list changes.
 */
export function useToastActions(): ToastActions {
  const context = React.useContext(ToastActionsContext);
  if (!context) {
    throw new Error("useToastActions must be used within a ToastProvider");
  }
  return context;
}

/**
 * useToast
 *
 * Returns the live toasts array plus the action callbacks.
 * Use this only when you need to read the active toast list (e.g. for a custom
 * renderer). Prefer `useToastActions` everywhere else.
 */
export function useToast() {
  const toasts = React.useContext(ToastStateContext);
  const actions = React.useContext(ToastActionsContext);

  if (toasts === undefined || actions === undefined) {
    throw new Error("useToast must be used within a ToastProvider");
  }

  return { toasts, ...actions };
}
