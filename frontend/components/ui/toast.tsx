"use client";

/**
 * Toast component
 *
 * Improvements over the previous version
 * ────────────────────────────────────────
 * • Smooth enter/exit animations via CSS keyframes (slide-in from right on
 *   enter, fade+slide-out on exit) driven by a local `visible` state.
 * • Animated progress bar that drains over `duration` ms so users know how
 *   long they have before the toast auto-dismisses.
 * • Optional `action` slot for an "Undo" or CTA button inside the toast.
 * • `persistent` prop (or duration ≤ 0) disables auto-dismiss and hides the
 *   progress bar.
 * • Correct ARIA: role="status" for non-error variants, role="alert" for
 *   errors/warnings. The outer list container owns aria-live, so individual
 *   toasts no longer carry their own aria-live attribute.
 * • Pause on hover — the countdown is frozen while the user's cursor is on
 *   the toast, giving them time to read or interact.
 */

import * as React from "react";
import { X, CheckCircle2, AlertCircle, AlertTriangle, Info } from "lucide-react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// ─── Variants ─────────────────────────────────────────────────────────────────

const toastVariants = cva(
  [
    "pointer-events-auto relative w-full overflow-hidden rounded-lg border shadow-lg",
    "flex items-start gap-3 p-4 pr-10",
    // Enter animation applied immediately; exit animation swapped in by state
    "data-[state=open]:animate-toast-in",
    "data-[state=closed]:animate-toast-out",
  ].join(" "),
  {
    variants: {
      variant: {
        success: "border-emerald-500/30 bg-zinc-900 text-emerald-400",
        error:   "border-red-500/30   bg-zinc-900 text-red-400",
        warning: "border-yellow-500/30 bg-zinc-900 text-yellow-400",
        info:    "border-[#37B7C3]/30  bg-zinc-900 text-[#7DE3EC]",
      },
    },
    defaultVariants: { variant: "info" },
  },
);

const progressVariants = cva(
  "absolute bottom-0 left-0 h-[2px] w-full origin-left",
  {
    variants: {
      variant: {
        success: "bg-emerald-500",
        error:   "bg-red-500",
        warning: "bg-yellow-500",
        info:    "bg-[#37B7C3]",
      },
    },
    defaultVariants: { variant: "info" },
  },
);

const icons = {
  success: CheckCircle2,
  error:   AlertCircle,
  warning: AlertTriangle,
  info:    Info,
} as const;

// ─── Types ────────────────────────────────────────────────────────────────────

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface ToastProps extends VariantProps<typeof toastVariants> {
  /** Unique identifier managed by ToastProvider — not required when calling addToast. */
  id: string;
  title?: string;
  description?: string;
  /** Optional CTA button rendered inside the toast (e.g. "Undo"). */
  action?: ToastAction;
  /** Called by ToastProvider when the toast should be removed. */
  onClose?: () => void;
  /**
   * Auto-dismiss delay in ms.
   * Pass 0 or set `persistent` to true to disable auto-dismiss.
   * @default 5000
   */
  duration?: number;
  /**
   * Prevent auto-dismiss entirely.  Overrides `duration`.
   * The progress bar is hidden when persistent.
   */
  persistent?: boolean;
}

// ─── Component ────────────────────────────────────────────────────────────────

const Toast = React.forwardRef<HTMLDivElement, ToastProps>(
  (
    {
      variant = "info",
      title,
      description,
      action,
      onClose,
      duration = 5000,
      persistent = false,
    },
    ref,
  ) => {
    const Icon = icons[variant ?? "info"];
    const autoDismiss = !persistent && duration > 0;

    // `open` drives the animation data-state attribute
    const [open, setOpen] = React.useState(true);
    // Pause the countdown while hovered
    const [paused, setPaused] = React.useState(false);

    // ── Auto-dismiss with pause support ──────────────────────────────────────
    const remainingRef = React.useRef(duration);
    const startRef = React.useRef<number>(Date.now());
    const timerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);

    const clearTimer = React.useCallback(() => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    }, []);

    const scheduleClose = React.useCallback(
      (delay: number) => {
        clearTimer();
        timerRef.current = setTimeout(() => {
          setOpen(false);
        }, delay);
        startRef.current = Date.now();
      },
      [clearTimer],
    );

    React.useEffect(() => {
      if (!autoDismiss) return;
      scheduleClose(remainingRef.current);
      return clearTimer;
    }, [autoDismiss, scheduleClose, clearTimer]);

    React.useEffect(() => {
      if (!autoDismiss) return;
      if (paused) {
        remainingRef.current -= Date.now() - startRef.current;
        clearTimer();
      } else {
        scheduleClose(Math.max(0, remainingRef.current));
      }
    }, [paused, autoDismiss, scheduleClose, clearTimer]);

    // When the exit animation ends, call onClose to actually remove from the list
    const handleAnimationEnd = (e: React.AnimationEvent) => {
      if (e.animationName === "toast-out") {
        onClose?.();
      }
    };

    const role =
      variant === "error" || variant === "warning" ? "alert" : "status";

    return (
      <div
        ref={ref}
        data-state={open ? "open" : "closed"}
        role={role}
        aria-atomic="true"
        onMouseEnter={() => setPaused(true)}
        onMouseLeave={() => setPaused(false)}
        onAnimationEnd={handleAnimationEnd}
        className={cn(toastVariants({ variant }))}
      >
        {/* Icon */}
        <Icon className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />

        {/* Content */}
        <div className="flex-1 min-w-0 space-y-1">
          {title && (
            <p className="text-sm font-semibold leading-none text-white">
              {title}
            </p>
          )}
          {description && (
            <p className="text-xs leading-relaxed text-zinc-400">
              {description}
            </p>
          )}
          {action && (
            <button
              type="button"
              onClick={() => {
                action.onClick();
                setOpen(false);
              }}
              className="mt-1.5 text-xs font-semibold underline underline-offset-2 opacity-90 hover:opacity-100 transition-opacity focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
            >
              {action.label}
            </button>
          )}
        </div>

        {/* Dismiss button */}
        <button
          type="button"
          onClick={() => setOpen(false)}
          aria-label="Dismiss notification"
          className="absolute right-2 top-2 rounded-md p-1 text-zinc-500 transition-colors hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
        >
          <X className="h-3.5 w-3.5" aria-hidden="true" />
        </button>

        {/* Progress bar */}
        {autoDismiss && (
          <span
            aria-hidden="true"
            className={cn(progressVariants({ variant }))}
            style={{
              animation: `toast-progress ${duration}ms linear forwards`,
              animationPlayState: paused ? "paused" : "running",
            }}
          />
        )}
      </div>
    );
  },
);

Toast.displayName = "Toast";

export { Toast, toastVariants };
