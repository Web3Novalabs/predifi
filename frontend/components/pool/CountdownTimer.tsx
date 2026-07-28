"use client";

import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

export interface CountdownTimerProps {
  /** Unix timestamp (seconds) when the pool closes. */
  endTime: number;
  className?: string;
  onExpire?: () => void;
}

function parts(remaining: number) {
  const s = Math.max(0, remaining);
  const days = Math.floor(s / 86_400);
  const hours = Math.floor((s % 86_400) / 3_600);
  const minutes = Math.floor((s % 3_600) / 60);
  const seconds = s % 60;
  return { days, hours, minutes, seconds, total: s };
}

function pad(n: number) {
  return String(n).padStart(2, "0");
}

/**
 * Live countdown to pool close. Pulses when under 1 hour remains.
 */
export function CountdownTimer({ endTime, className, onExpire }: CountdownTimerProps) {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const remaining = parts(endTime - now);
  const expired = remaining.total <= 0;

  useEffect(() => {
    const id = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1_000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    if (expired) onExpire?.();
  }, [expired, onExpire]);

  if (expired) {
    return (
      <div className={cn("text-sm font-medium text-zinc-500", className)}>
        Market closed
      </div>
    );
  }

  const urgent = remaining.total < 3_600;

  return (
    <div
      className={cn(
        "flex items-center gap-2 font-mono text-sm",
        urgent && "animate-pulse text-amber-400",
        className,
      )}
      aria-live="polite"
      aria-label={`Time remaining ${remaining.days} days ${remaining.hours} hours ${remaining.minutes} minutes ${remaining.seconds} seconds`}
    >
      {remaining.days > 0 && (
        <Unit label="d" value={remaining.days} />
      )}
      <Unit label="h" value={remaining.hours} />
      <Unit label="m" value={remaining.minutes} />
      <Unit label="s" value={remaining.seconds} />
    </div>
  );
}

function Unit({ label, value }: { label: string; value: number }) {
  return (
    <span className="inline-flex items-baseline gap-0.5 rounded-md bg-white/5 px-2 py-1">
      <span className="text-base font-semibold text-white tabular-nums">
        {pad(value)}
      </span>
      <span className="text-[10px] uppercase text-zinc-500">{label}</span>
    </span>
  );
}
