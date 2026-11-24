"use client";

import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";

export interface AnimatedNumberProps {
  value: number;
  /** Fraction digits for display. */
  decimals?: number;
  className?: string;
  /** Prefix / suffix (e.g. token ticker). */
  prefix?: string;
  suffix?: string;
  /** Animation duration in ms. */
  durationMs?: number;
}

/**
 * Smoothly interpolates toward `value` and flashes teal/rose on change.
 */
export function AnimatedNumber({
  value,
  decimals = 0,
  className,
  prefix = "",
  suffix = "",
  durationMs = 450,
}: AnimatedNumberProps) {
  const [display, setDisplay] = useState(value);
  const [flash, setFlash] = useState<"up" | "down" | null>(null);
  const fromRef = useRef(value);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    const from = fromRef.current;
    const to = value;
    if (from === to) return;

    setFlash(to > from ? "up" : "down");
    const start = performance.now();

    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / durationMs);
      // easeOutCubic
      const eased = 1 - Math.pow(1 - t, 3);
      setDisplay(from + (to - from) * eased);
      if (t < 1) {
        rafRef.current = requestAnimationFrame(tick);
      } else {
        fromRef.current = to;
        setDisplay(to);
        setTimeout(() => setFlash(null), 350);
      }
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => {
      if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
    };
  }, [value, durationMs]);

  const formatted = display.toLocaleString("en-US", {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });

  return (
    <span
      className={cn(
        "tabular-nums transition-colors duration-300",
        flash === "up" && "text-[#37B7C3]",
        flash === "down" && "text-rose-400",
        className,
      )}
    >
      {prefix}
      {formatted}
      {suffix}
    </span>
  );
}
