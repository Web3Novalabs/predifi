"use client";

import * as React from "react";
import { X, CheckCircle2, AlertCircle, AlertTriangle, Info } from "lucide-react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const toastVariants = cva(
  "pointer-events-auto relative flex w-full items-center justify-between space-x-4 overflow-hidden rounded-md border p-4 pr-8 shadow-lg transition-all",
  {
    variants: {
      variant: {
        success: "border-success/50 bg-success/10 text-success",
        error: "border-destructive/50 bg-destructive/10 text-destructive",
        warning: "border-warning/50 bg-warning/10 text-warning",
        info: "border-info/50 bg-info/10 text-info",
      },
    },
    defaultVariants: {
      variant: "info",
    },
  }
);

const icons = {
  success: CheckCircle2,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

export interface ToastProps extends VariantProps<typeof toastVariants> {
  id: string;
  title?: string;
  description?: string;
  onClose?: () => void;
  duration?: number;
}

const Toast = React.forwardRef<HTMLDivElement, ToastProps>(
  ({ id, variant = "info", title, description, onClose, duration = 5000 }, ref) => {
    const Icon = icons[variant || "info"];

    React.useEffect(() => {
      if (duration && duration > 0) {
        const timer = setTimeout(() => {
          onClose?.();
        }, duration);

        return () => clearTimeout(timer);
      }
    }, [duration, onClose]);

    return (
      <div
        ref={ref}
        data-id={id}
        className={cn(toastVariants({ variant }))}
        role="alert"
        aria-live="assertive"
        aria-atomic="true"
      >
        <div className="flex items-start gap-3 flex-1">
          <Icon className="h-5 w-5 shrink-0 mt-0.5" />
          <div className="flex flex-col gap-1 flex-1">
            {title && <div className="text-sm font-semibold">{title}</div>}
            {description && (
              <div className="text-sm opacity-90">{description}</div>
            )}
          </div>
        </div>
        <button
          onClick={onClose}
          className="absolute right-2 top-2 rounded-md p-1 opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
          aria-label="Close"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    );
  }
);

Toast.displayName = "Toast";

export { Toast, toastVariants };
