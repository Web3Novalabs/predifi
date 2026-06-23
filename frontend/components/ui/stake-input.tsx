"use client";

import * as React from "react";
import { cn } from "@/lib/utils";

export interface StakeInputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "type" | "onChange"> {
  label?: string;
  error?: string;
  helperText?: string;
  token?: string;
  value?: string;
  onChange?: (raw: string, numeric: number | null) => void;
}

/**
 * A controlled numeric input for stake amounts.
 *
 * - Strips non-numeric characters (except a single decimal point).
 * - Limits to 7 decimal places (Soroban stroop precision).
 * - Blocks leading zeros (e.g. "007" → "7").
 * - Displays an optional token symbol suffix.
 * - Calls onChange with both the sanitized string and the parsed numeric value.
 */
const StakeInput = React.forwardRef<HTMLInputElement, StakeInputProps>(
  (
    {
      className,
      label,
      error,
      helperText,
      token = "XLM",
      value = "",
      onChange,
      disabled,
      id,
      placeholder = "0.00",
      ...props
    },
    ref,
  ) => {
    // eslint-disable-next-line
    const inputId = id || React.useId();

    const sanitize = (raw: string): string => {
      // Allow only digits and one decimal point
      let sanitized = raw.replace(/[^0-9.]/g, "");

      // Keep only the first decimal point
      const dotIndex = sanitized.indexOf(".");
      if (dotIndex !== -1) {
        sanitized =
          sanitized.slice(0, dotIndex + 1) +
          sanitized.slice(dotIndex + 1).replace(/\./g, "");
      }

      // Cap to 7 decimal places (stroop precision)
      if (dotIndex !== -1) {
        const [integer, fraction] = sanitized.split(".");
        sanitized = `${integer}.${fraction.slice(0, 7)}`;
      }

      // Remove leading zeros before the decimal (e.g. "007" → "7", "0.5" kept)
      sanitized = sanitized.replace(/^0+(\d)/, "$1");

      return sanitized;
    };

    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      const raw = sanitize(e.target.value);
      const numeric = raw === "" || raw === "." ? null : parseFloat(raw);
      onChange?.(raw, Number.isFinite(numeric) ? numeric : null);
    };

    return (
      <div className="w-full space-y-2">
        {label && (
          <label
            htmlFor={inputId}
            className={cn(
              "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
              error && "text-destructive",
            )}
          >
            {label}
          </label>
        )}
        <div className="relative">
          <input
            id={inputId}
            type="text"
            inputMode="decimal"
            pattern="[0-9]*[.]?[0-9]*"
            value={value}
            onChange={handleChange}
            placeholder={placeholder}
            disabled={disabled}
            className={cn(
              "flex h-10 w-full rounded-md border border-input bg-transparent px-3 py-2 pr-16 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 font-mono",
              error && "border-destructive focus-visible:ring-destructive",
              className,
            )}
            ref={ref}
            aria-invalid={error ? "true" : "false"}
            aria-describedby={
              error
                ? `${inputId}-error`
                : helperText
                  ? `${inputId}-helper`
                  : undefined
            }
            {...props}
          />
          {token && (
            <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs font-medium text-muted-foreground select-none pointer-events-none">
              {token}
            </span>
          )}
        </div>
        {error && (
          <p
            id={`${inputId}-error`}
            className="text-sm font-medium text-destructive"
            role="alert"
          >
            {error}
          </p>
        )}
        {!error && helperText && (
          <p id={`${inputId}-helper`} className="text-sm text-muted-foreground">
            {helperText}
          </p>
        )}
      </div>
    );
  },
);

StakeInput.displayName = "StakeInput";

export { StakeInput };
