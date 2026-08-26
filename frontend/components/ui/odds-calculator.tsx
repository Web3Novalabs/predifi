"use client";

import { useState, useCallback, useMemo, useId } from "react";
import { Calculator } from "lucide-react";
import { cn } from "@/lib/utils";

export interface OddsCalculatorProps {
  token?: string;
  className?: string;
}

function fmt(n: number, dp = 2): string {
  return n.toLocaleString("en-US", { minimumFractionDigits: dp, maximumFractionDigits: dp });
}

function sanitize(raw: string): string {
  let s = raw.replace(/[^0-9.]/g, "");
  const dot = s.indexOf(".");
  if (dot !== -1) s = s.slice(0, dot + 1) + s.slice(dot + 1).replace(/\./g, "");
  s = s.replace(/^0+(\d)/, "$1");
  return s;
}

export function OddsCalculator({ token = "XLM", className }: OddsCalculatorProps) {
  const uid = useId();
  const [split, setSplit] = useState(50); // probability % for outcome A
  const [stakeRaw, setStakeRaw] = useState("");

  const stake = useMemo(() => parseFloat(stakeRaw) || 0, [stakeRaw]);

  const probA = split / 100;
  const probB = 1 - probA;
  const oddsA = probA > 0 ? 1 / probA : 0;
  const oddsB = probB > 0 ? 1 / probB : 0;
  const payoutA = stake * oddsA;
  const payoutB = stake * oddsB;

  const handleStake = useCallback((v: string) => setStakeRaw(sanitize(v)), []);

  return (
    <div className={cn("rounded-2xl border border-white/10 bg-[#121212] p-5 space-y-5 w-full", className)}>
      {/* Header */}
      <div className="flex items-center gap-2">
        <div className="w-8 h-8 rounded-lg bg-[#37B7C3]/15 flex items-center justify-center">
          <Calculator className="w-4 h-4 text-[#37B7C3]" />
        </div>
        <div>
          <h3 className="text-sm font-semibold text-white">Odds Calculator</h3>
          <p className="text-[10px] text-zinc-500">Adjust the probability split to see implied odds</p>
        </div>
      </div>

      {/* Probability bar */}
      <div className="space-y-2">
        <div className="h-3 w-full rounded-full overflow-hidden flex">
          <div
            className="h-full bg-[#37B7C3] transition-[width] duration-200"
            style={{ width: `${split}%` }}
          />
          <div className="h-full bg-zinc-600 flex-1 transition-[width] duration-200" />
        </div>
        <div className="flex justify-between text-[10px] text-zinc-400">
          <span>Outcome A — {split}%</span>
          <span>Outcome B — {100 - split}%</span>
        </div>
      </div>

      {/* Slider */}
      <input
        id={`${uid}-slider`}
        type="range"
        min={1}
        max={99}
        value={split}
        onChange={(e) => setSplit(Number(e.target.value))}
        className="w-full accent-[#37B7C3] cursor-pointer"
        aria-label="Probability split"
      />

      {/* Stake input */}
      <div className="space-y-1.5">
        <label htmlFor={`${uid}-stake`} className="text-xs font-medium text-zinc-400 uppercase tracking-wider">
          Your stake
        </label>
        <div className="relative">
          <input
            id={`${uid}-stake`}
            type="text"
            inputMode="decimal"
            value={stakeRaw}
            onChange={(e) => handleStake(e.target.value)}
            placeholder="0.00"
            className="w-full rounded-lg border border-white/10 bg-white/[0.04] px-3 py-2.5 pr-14 text-sm font-mono text-white placeholder:text-zinc-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3] transition-[border-color,box-shadow] duration-200"
          />
          <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-zinc-500 pointer-events-none">
            {token}
          </span>
        </div>
      </div>

      {/* Results grid */}
      <div className="grid grid-cols-2 gap-3">
        {[
          { label: "Outcome A", prob: split, odds: oddsA, payout: payoutA, active: true },
          { label: "Outcome B", prob: 100 - split, odds: oddsB, payout: payoutB, active: false },
        ].map(({ label, prob, odds, payout, active }) => (
          <div
            key={label}
            className={cn(
              "rounded-xl border p-3 space-y-2",
              active ? "border-[#37B7C3]/30 bg-[#37B7C3]/[0.05]" : "border-white/5 bg-white/[0.02]",
            )}
          >
            <p className={cn("text-xs font-semibold", active ? "text-[#37B7C3]" : "text-zinc-400")}>{label}</p>
            <div className="space-y-1">
              <div className="flex justify-between text-[10px]">
                <span className="text-zinc-500">Probability</span>
                <span className="text-white font-mono">{prob}%</span>
              </div>
              <div className="flex justify-between text-[10px]">
                <span className="text-zinc-500">Odds</span>
                <span className="text-white font-mono">×{fmt(odds)}</span>
              </div>
              <div className="flex justify-between text-[10px]">
                <span className="text-zinc-500">Payout</span>
                <span className={cn("font-mono", stake > 0 ? (active ? "text-[#37B7C3]" : "text-white") : "text-zinc-600")}>
                  {stake > 0 ? `${fmt(payout)} ${token}` : "—"}
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>

      <p className="text-[10px] text-zinc-600">
        Estimates are illustrative. Actual odds depend on pool participation at resolution.
      </p>
    </div>
  );
}
