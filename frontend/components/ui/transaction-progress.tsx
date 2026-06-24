"use client";

import { memo } from "react";
import { CheckCircle2, Clock, Loader2, XCircle, Send } from "lucide-react";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type TxStatus = "idle" | "submitted" | "processing" | "confirmed" | "failed";

export interface TransactionProgressProps {
  status: TxStatus;
  txHash?: string;
  errorMessage?: string;
  className?: string;
}

// ---------------------------------------------------------------------------
// Step definitions
// ---------------------------------------------------------------------------

interface Step {
  id: "submitted" | "processing" | "confirmed";
  label: string;
  sublabel: string;
}

const STEPS: Step[] = [
  { id: "submitted", label: "Submitted", sublabel: "Transaction sent to network" },
  { id: "processing", label: "Processing", sublabel: "Awaiting block confirmation" },
  { id: "confirmed", label: "Confirmed", sublabel: "Transaction finalised on-chain" },
];

// ---------------------------------------------------------------------------
// Order used to decide which steps are "done"
// ---------------------------------------------------------------------------

const STATUS_ORDER: Record<TxStatus, number> = {
  idle: -1,
  submitted: 0,
  processing: 1,
  confirmed: 2,
  failed: 1, // show processing as the point of failure
};

// ---------------------------------------------------------------------------
// StepIcon
// ---------------------------------------------------------------------------

type StepState = "done" | "active" | "failed" | "idle";

const StepIcon = memo(function StepIcon({ state }: { state: StepState }) {
  const base = "w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 transition-[background-color,border-color,box-shadow] duration-300";

  if (state === "done") {
    return (
      <span className={cn(base, "bg-[#37B7C3] shadow-[0_0_10px_rgba(55,183,195,0.4)]")}>
        <CheckCircle2 className="w-4 h-4 text-[#001112]" />
      </span>
    );
  }
  if (state === "active") {
    return (
      <span className={cn(base, "border-2 border-[#37B7C3] bg-[#37B7C3]/10 shadow-[0_0_10px_rgba(55,183,195,0.3)]")}>
        <Loader2 className="w-4 h-4 text-[#37B7C3] animate-spin" />
      </span>
    );
  }
  if (state === "failed") {
    return (
      <span className={cn(base, "border-2 border-red-500 bg-red-500/10 shadow-[0_0_10px_rgba(239,68,68,0.3)]")}>
        <XCircle className="w-4 h-4 text-red-400" />
      </span>
    );
  }
  // idle
  return (
    <span className={cn(base, "border-2 border-white/10 bg-white/[0.03]")}>
      <span className="w-2 h-2 rounded-full bg-white/20" />
    </span>
  );
});

StepIcon.displayName = "StepIcon";

// ---------------------------------------------------------------------------
// Connector line between steps
// ---------------------------------------------------------------------------

const Connector = memo(function Connector({ filled, animated }: { filled: boolean; animated?: boolean }) {
  return (
    <div className="flex-1 h-[2px] mx-1 rounded-full bg-white/[0.07] overflow-hidden relative">
      <div
        className={cn(
          "absolute inset-y-0 left-0 rounded-full transition-[width] duration-500 ease-out",
          filled ? "bg-[#37B7C3] w-full" : "w-0",
          animated && !filled && "bg-gradient-to-r from-[#37B7C3]/60 to-transparent animate-pulse w-full",
        )}
      />
    </div>
  );
});

Connector.displayName = "Connector";

// ---------------------------------------------------------------------------
// TransactionProgress
// ---------------------------------------------------------------------------

export const TransactionProgress = memo(function TransactionProgress({
  status,
  txHash,
  errorMessage,
  className,
}: TransactionProgressProps) {
  if (status === "idle") return null;

  const order = STATUS_ORDER[status];
  const isFailed = status === "failed";

  function stepState(step: Step, stepIdx: number): StepState {
    const stepOrder = stepIdx;
    if (isFailed && stepOrder === order) return "failed";
    if (stepOrder < order) return "done";
    if (stepOrder === order) return "active";
    return "idle";
  }

  // Overall progress percentage (0 → 100)
  const progressPct = isFailed
    ? Math.round((order / (STEPS.length - 1)) * 100)
    : Math.round((order / (STEPS.length - 1)) * 100);

  return (
    <div className={cn("rounded-2xl border border-white/10 bg-[#121212] p-5 space-y-5 w-full", className)}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className={cn(
            "w-8 h-8 rounded-lg flex items-center justify-center transition-colors duration-300",
            isFailed ? "bg-red-500/15" : "bg-[#37B7C3]/15",
          )}>
            <Send className={cn("w-4 h-4", isFailed ? "text-red-400" : "text-[#37B7C3]")} />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-white">Transaction Status</h3>
            <p className="text-[10px] text-zinc-500">Stellar network</p>
          </div>
        </div>

        {/* Status badge */}
        <span className={cn(
          "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium",
          status === "confirmed" && "bg-emerald-400/10 text-emerald-400",
          status === "processing" && "bg-[#37B7C3]/10 text-[#37B7C3]",
          status === "submitted" && "bg-yellow-400/10 text-yellow-400",
          status === "failed" && "bg-red-400/10 text-red-400",
        )}>
          {status === "confirmed" && <CheckCircle2 className="w-3 h-3" />}
          {(status === "processing" || status === "submitted") && <Loader2 className="w-3 h-3 animate-spin" />}
          {status === "failed" && <XCircle className="w-3 h-3" />}
          {status.charAt(0).toUpperCase() + status.slice(1)}
        </span>
      </div>

      {/* Linear progress bar */}
      <div className="space-y-1">
        <div className="h-1.5 w-full rounded-full bg-white/[0.06] overflow-hidden">
          <div
            className={cn(
              "h-full rounded-full transition-[width] duration-700 ease-out",
              isFailed ? "bg-red-500" : "bg-[#37B7C3]",
            )}
            style={{ width: `${progressPct}%` }}
          />
        </div>
        <div className="flex justify-between text-[10px] text-zinc-600">
          <span>0%</span>
          <span className={cn(isFailed ? "text-red-400" : "text-[#37B7C3]")}>{progressPct}%</span>
          <span>100%</span>
        </div>
      </div>

      {/* Step indicators */}
      <div className="flex items-center">
        {STEPS.map((step, idx) => (
          <div key={step.id} className="flex items-center flex-1 min-w-0">
            <StepIcon state={stepState(step, idx)} />
            {idx < STEPS.length - 1 && (
              <Connector
                filled={idx < order && !isFailed}
                animated={idx === order - 1 && status === "processing"}
              />
            )}
          </div>
        ))}
      </div>

      {/* Step labels */}
      <div className="flex justify-between">
        {STEPS.map((step, idx) => {
          const state = stepState(step, idx);
          return (
            <div key={step.id} className="flex flex-col items-center text-center" style={{ width: "33.33%" }}>
              <span className={cn(
                "text-xs font-medium transition-colors duration-300",
                state === "done" && "text-[#37B7C3]",
                state === "active" && "text-white",
                state === "failed" && "text-red-400",
                state === "idle" && "text-zinc-600",
              )}>
                {step.label}
              </span>
              <span className="text-[10px] text-zinc-600 mt-0.5 leading-tight hidden sm:block">
                {step.sublabel}
              </span>
            </div>
          );
        })}
      </div>

      {/* TX hash */}
      {txHash && (
        <div className="rounded-lg bg-white/[0.03] border border-white/5 px-3 py-2 flex items-center justify-between gap-2">
          <span className="text-[10px] text-zinc-500 flex-shrink-0">TX Hash</span>
          <span className="text-[10px] font-mono text-zinc-300 truncate">{txHash}</span>
        </div>
      )}

      {/* Error message */}
      {isFailed && errorMessage && (
        <div className="rounded-lg bg-red-500/10 border border-red-500/20 px-3 py-2">
          <p className="text-xs text-red-400">{errorMessage}</p>
        </div>
      )}
    </div>
  );
});

TransactionProgress.displayName = "TransactionProgress";
