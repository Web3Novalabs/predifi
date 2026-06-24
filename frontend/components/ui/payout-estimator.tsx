"use client";

import { useState, useCallback, useMemo, useId } from "react";
import { TrendingUp, Info } from "lucide-react";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PayoutEstimatorProps {
  /** Token symbol shown in the UI (default "XLM"). */
  token?: string;
  className?: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sanitizeNumeric(raw: string): string {
  let s = raw.replace(/[^0-9.]/g, "");
  const dot = s.indexOf(".");
  if (dot !== -1) {
    s = s.slice(0, dot + 1) + s.slice(dot + 1).replace(/\./g, "");
  }
  // cap to 7 dp (stroop precision)
  if (dot !== -1) {
    const [int, frac = ""] = s.split(".");
    s = `${int}.${frac.slice(0, 7)}`;
  }
  // strip leading zeros before decimal
  s = s.replace(/^0+(\d)/, "$1");
  return s;
}

function toNumber(s: string): number {
  const n = parseFloat(s);
  return Number.isFinite(n) ? n : 0;
}

function fmt(n: number, dp = 2): string {
  return n.toLocaleString("en-US", {
    minimumFractionDigits: dp,
    maximumFractionDigits: dp,
  });
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function NumericField({
  id,
  label,
  value,
  onChange,
  placeholder,
  suffix,
  min,
  max,
  step,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  suffix?: string;
  min?: number;
  max?: number;
  step?: number;
}) {
  return (
    <div className="space-y-1.5">
      <label htmlFor={id} className="text-xs font-medium text-zinc-400 uppercase tracking-wider">
        {label}
      </label>
      <div className="relative">
        <input
          id={id}
          type="text"
          inputMode="decimal"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          min={min}
          max={max}
          step={step}
          className={cn(
            "w-full rounded-lg border border-white/10 bg-white/[0.04] px-3 py-2.5 text-sm font-mono text-white placeholder:text-zinc-600",
            "focus:outline-none focus:ring-2 focus:ring-[#37B7C3]/50 focus:border-[#37B7C3]/60",
            "transition-[border-color,box-shadow] duration-200",
            suffix && "pr-14",
          )}
        />
        {suffix && (
          <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs font-medium text-zinc-500 select-none pointer-events-none">
            {suffix}
          </span>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stat row inside the result card
// ---------------------------------------------------------------------------

function Stat({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-white/5 last:border-0">
      <span className="text-xs text-zinc-400">{label}</span>
      <span className={cn("text-sm font-mono font-medium", accent ? "text-[#37B7C3]" : "text-white")}>
        {value}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PayoutEstimator
// ---------------------------------------------------------------------------

export function PayoutEstimator({ token = "XLM", className }: PayoutEstimatorProps) {
  const uid = useId();
  const stakeId = `${uid}-stake`;
  const oddsId = `${uid}-odds`;

  const [stakeRaw, setStakeRaw] = useState("");
  const [oddsRaw, setOddsRaw] = useState("");

  const handleStake = useCallback((v: string) => setStakeRaw(sanitizeNumeric(v)), []);
  const handleOdds = useCallback((v: string) => {
    const s = sanitizeNumeric(v);
    // cap odds display to 2 dp
    const dot = s.indexOf(".");
    if (dot !== -1) {
      const [int, frac = ""] = s.split(".");
      setOddsRaw(`${int}.${frac.slice(0, 2)}`);
    } else {
      setOddsRaw(s);
    }
  }, []);

  const stake = useMemo(() => toNumber(stakeRaw), [stakeRaw]);
  const odds = useMemo(() => toNumber(oddsRaw), [oddsRaw]);

  const payout = useMemo(() => stake * odds, [stake, odds]);
  const profit = useMemo(() => payout - stake, [payout, stake]);

  // Visual bar: ratio of profit to payout, capped at 100%
  const profitRatio = useMemo(() => {
    if (payout <= 0) return 0;
    return Math.min((profit / payout) * 100, 100);
  }, [profit, payout]);

  const hasValues = stake > 0 && odds > 1;
  const isValidOdds = odds >= 1 || oddsRaw === "";

  return (
    <div className={cn("rounded-2xl border border-white/10 bg-[#121212] p-5 space-y-5 w-full", className)}>
      {/* Header */}
      <div className="flex items-center gap-2">
        <div className="w-8 h-8 rounded-lg bg-[#37B7C3]/15 flex items-center justify-center">
          <TrendingUp className="w-4 h-4 text-[#37B7C3]" />
        </div>
        <div>
          <h3 className="text-sm font-semibold text-white">Payout Estimator</h3>
          <p className="text-[10px] text-zinc-500">Estimate your returns before staking</p>
        </div>
      </div>

      {/* Inputs */}
      <div className="grid grid-cols-2 gap-3">
        <NumericField
          id={stakeId}
          label="Stake amount"
          value={stakeRaw}
          onChange={handleStake}
          placeholder="0.00"
          suffix={token}
        />
        <NumericField
          id={oddsId}
          label="Odds (multiplier)"
          value={oddsRaw}
          onChange={handleOdds}
          placeholder="1.50"
          min={1}
          step={0.01}
        />
      </div>

      {!isValidOdds && (
        <p className="flex items-center gap-1.5 text-xs text-yellow-400">
          <Info className="w-3 h-3 flex-shrink-0" />
          Odds must be 1.00 or higher.
        </p>
      )}

      {/* Result card */}
      <div
        className={cn(
          "rounded-xl border p-4 space-y-1 transition-[opacity,transform] duration-300 ease-out",
          hasValues && isValidOdds
            ? "opacity-100 translate-y-0 border-[#37B7C3]/20 bg-[#37B7C3]/[0.05]"
            : "opacity-40 translate-y-1 border-white/5 bg-white/[0.02]",
        )}
      >
        <Stat label={`Stake (${token})`} value={hasValues ? fmt(stake) : "—"} />
        <Stat label="Odds" value={hasValues ? `×${fmt(odds)}` : "—"} />
        <Stat label={`Estimated profit (${token})`} value={hasValues ? fmt(profit) : "—"} accent={hasValues && profit > 0} />
        <Stat label={`Total payout (${token})`} value={hasValues ? fmt(payout) : "—"} accent={hasValues} />
      </div>

      {/* Visual profit bar */}
      <div className="space-y-1.5">
        <div className="flex justify-between text-[10px] text-zinc-500">
          <span>Stake</span>
          <span>Profit</span>
        </div>
        <div className="h-2 w-full rounded-full bg-white/[0.06] overflow-hidden">
          {/* stake portion */}
          <div className="h-full flex">
            <div
              className="h-full bg-zinc-600 transition-[width] duration-500 ease-out"
              style={{ width: hasValues ? `${100 - profitRatio}%` : "0%" }}
            />
            <div
              className="h-full bg-[#37B7C3] transition-[width] duration-500 ease-out"
              style={{ width: hasValues ? `${profitRatio}%` : "0%" }}
            />
          </div>
        </div>
        <div className="flex justify-between text-[10px]">
          <span className="text-zinc-500">{hasValues ? `${fmt(100 - profitRatio, 1)}%` : "0%"}</span>
          <span className={cn("transition-colors", hasValues && profit > 0 ? "text-[#37B7C3]" : "text-zinc-500")}>
            {hasValues ? `${fmt(profitRatio, 1)}%` : "0%"}
          </span>
        </div>
      </div>

      <p className="text-[10px] text-zinc-600 leading-relaxed">
        Estimates are illustrative only. Actual payouts depend on final pool odds at resolution.
      </p>
    </div>
  );
}
