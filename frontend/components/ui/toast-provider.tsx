"use client";

import * as React from "react";
import { Toast, type ToastProps } from "./toast";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ToastActions = {
  addToast: (toast: Omit<ToastProps, "id" | "onClose">) => void;
  removeToast: (id: string) => void;
};

// ---------------------------------------------------------------------------
// Contexts
//
// Split into two contexts so components that only call addToast/removeToast
// are not re-rendered when the toasts[] array changes.
// ---------------------------------------------------------------------------

/**
 * ToastStateContext — holds the list of active toasts.
 * Only the toast renderer (inside ToastProvider) subscribes to this.
 */
const ToastStateContext = React.createContext<ToastProps[] | undefined>(
  undefined
);

/**
 * ToastActionsContext — holds stable addToast / removeToast callbacks.
 * These are memoized with useCallback so this context value never changes,
 * meaning consumers of useToastActions() are never triggered to re-render
 * by a toast being added or removed.
 */
const ToastActionsContext = React.createContext<ToastActions | undefined>(
  undefined
);

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = React.useState<ToastProps[]>([]);

  const removeToast = React.useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const addToast = React.useCallback(
    (toast: Omit<ToastProps, "id" | "onClose">) => {
      const id = Math.random().toString(36).slice(2, 9);
      setToasts((prev) => [...prev, { ...toast, id }]);
    },
    []
  );

  // Stable object — only recreated if addToast/removeToast refs change (they won't).
  const actions = React.useMemo(
    () => ({ addToast, removeToast }),
    [addToast, removeToast]
  );

  return (
    <ToastActionsContext.Provider value={actions}>
      <ToastStateContext.Provider value={toasts}>
        {children}
        <ToastList toasts={toasts} removeToast={removeToast} />
      </ToastStateContext.Provider>
    </ToastActionsContext.Provider>
  );
}

// ---------------------------------------------------------------------------
// Internal renderer — isolated so only it re-renders when toasts[] changes
// ---------------------------------------------------------------------------

function ToastList({
  toasts,
  removeToast,
}: {
  toasts: ToastProps[];
  removeToast: (id: string) => void;
}) {
  return (
    <div
      className="fixed top-4 right-4 z-50 flex flex-col gap-2 w-full max-w-md pointer-events-none"
      aria-live="polite"
    >
      {toasts.map((toast) => (
        <Toast
          key={toast.id}
          {...toast}
          onClose={() => removeToast(toast.id)}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

/**
 * useToastActions — subscribe only to actions (addToast, removeToast).
 * This hook does NOT cause re-renders when the toast list changes.
 * Prefer this in components that trigger toasts but don't render them.
 */
export function useToastActions(): ToastActions {
  const context = React.useContext(ToastActionsContext);
  if (!context) {
    throw new Error("useToastActions must be used within a ToastProvider");
  }
  return context;
}

/**
 * useToast — backward-compatible hook that returns the full context
 * (toasts state + actions).
 *
 * @deprecated Prefer useToastActions() in components that only fire toasts.
 * Use this only when you need to read the toasts[] list directly.
 */
export function useToast() {
  const toasts = React.useContext(ToastStateContext);
  const actions = React.useContext(ToastActionsContext);

  if (toasts === undefined || actions === undefined) {
    throw new Error("useToast must be used within a ToastProvider");
  }

  return { toasts, ...actions };
}
