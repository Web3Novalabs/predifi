"use client";

import { useMemo, useState } from "react";
import { cn } from "@/lib/utils";
import { formatUtcDateTime } from "@/lib/date";
import { formatChartValue } from "@/lib/stakeFilters";
import { useCurrentUserAddress } from "@/lib/hooks/useCurrentUserAddress";
import { useProfile } from "@/lib/hooks/useProfile";
import { NotificationBell } from "@/components/notifications/NotificationBell";
import { PerformanceCharts } from "@/components/profile/PerformanceCharts";
import { Skeleton } from "@/components/ui";
import type { ClaimStatus } from "@/lib/api/profile";

type PredictionStatus = "Active" | "Won" | "Lost" | "Pending";

const STATUS_STYLES: Record<PredictionStatus, string> = {
  Active: "bg-blue-500/20 text-blue-400",
  Won: "bg-emerald-500/20 text-emerald-400",
  Lost: "bg-red-500/20 text-red-400",
  Pending: "bg-yellow-500/20 text-yellow-400",
};

type Tab = "All" | PredictionStatus;
const TABS: Tab[] = ["All", "Active", "Won", "Lost", "Pending"];

function predictionStatus(claim: ClaimStatus): PredictionStatus {
  if (claim.pool_state === "active") return "Active";
  if (claim.pool_state !== "settled" || claim.is_winner === null) return "Pending";
  return claim.is_winner ? "Won" : "Lost";
}

function ClaimBadge({ claim }: { claim: ClaimStatus }) {
  if (claim.is_winner !== true) {
    return <span className="text-xs text-zinc-600">—</span>;
  }
  if (claim.claimed) {
    return <span className="text-xs font-medium text-emerald-400">Claimed</span>;
  }
  if (claim.claim_expired) {
    return <span className="text-xs font-medium text-red-400">Expired</span>;
  }
  if (claim.claim_window_expires_at) {
    return (
      <span className="text-xs font-medium text-yellow-400">
        Claim by {formatUtcDateTime(claim.claim_window_expires_at)}
      </span>
    );
  }
  return <span className="text-xs font-medium text-yellow-400">Unclaimed</span>;
}

function truncateAddress(address: string): string {
  if (address.length <= 10) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

export function PredictionHistoryClient() {
  const address = useCurrentUserAddress();
  const { profile, isLoading, isError, refresh } = useProfile(address);
  const [activeTab, setActiveTab] = useState<Tab>("All");

  const claims = profile?.claims ?? [];

  const filtered = useMemo(() => {
    if (activeTab === "All") return claims;
    return claims.filter((c) => predictionStatus(c) === activeTab);
  }, [claims, activeTab]);

  const stats = profile?.stats;

  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8 space-y-8">
      {/* Profile header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-5">
        <div className="flex items-center gap-5">
          <div className="w-16 h-16 rounded-full bg-gradient-to-br from-[#37B7C3]/40 to-indigo-500/40 border border-white/10 flex-shrink-0" />
          <div>
            <h1 className="text-2xl font-bold text-white">My Profile</h1>
            <p className="text-zinc-400 text-xs mt-0.5 font-mono">
              {address ? truncateAddress(address) : "Not connected"}
            </p>
          </div>
        </div>
        <NotificationBell address={address} />
      </div>

      {isError ? (
        <div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900 py-12 text-zinc-500">
          <p>Couldn&apos;t load your profile.</p>
          <button
            type="button"
            onClick={refresh}
            className="text-sm font-medium text-[#37B7C3] hover:underline"
          >
            Try again
          </button>
        </div>
      ) : (
        <>
          {/* Stats row */}
          <div className="grid grid-cols-2 sm:grid-cols-5 gap-4">
            {isLoading || !stats
              ? Array.from({ length: 5 }).map((_, i) => (
                  <div key={i} className="rounded-xl border border-zinc-800 bg-zinc-900 p-4 space-y-2">
                    <Skeleton className="h-3 w-20" />
                    <Skeleton className="h-5 w-16" />
                  </div>
                ))
              : [
                  { label: "Total Predictions", value: stats.total_predictions },
                  { label: "Win Rate", value: `${stats.win_rate.toFixed(0)}%` },
                  { label: "Total Staked", value: formatChartValue(stats.total_staked) },
                  { label: "Total Earned", value: formatChartValue(stats.total_earnings) },
                  { label: "Active Positions", value: stats.active_positions },
                ].map(({ label, value }) => (
                  <div key={label} className="rounded-xl border border-zinc-800 bg-zinc-900 p-4 space-y-1">
                    <p className="text-[10px] text-zinc-500 uppercase tracking-wider">{label}</p>
                    <p className="text-lg font-bold text-white font-mono">{value}</p>
                  </div>
                ))}
          </div>

          {/* Performance charts */}
          {!isLoading && stats && (
            <PerformanceCharts stats={stats} performance={profile?.performance ?? []} />
          )}

          {/* Predictions history */}
          <div className="space-y-4">
            <h2 className="text-base font-semibold text-white">Prediction History</h2>

            {/* Tabs */}
            <div className="flex items-center gap-1 border-b border-zinc-800 overflow-x-auto">
              {TABS.map((tab) => (
                <button
                  key={tab}
                  type="button"
                  onClick={() => setActiveTab(tab)}
                  className={cn(
                    "px-4 py-2.5 text-sm font-medium transition-colors relative whitespace-nowrap",
                    activeTab === tab ? "text-[#37B7C3]" : "text-zinc-500 hover:text-zinc-300",
                  )}
                >
                  {tab}
                  {activeTab === tab && (
                    <span className="absolute bottom-0 left-0 w-full h-0.5 bg-[#37B7C3]" />
                  )}
                </button>
              ))}
            </div>

            {/* Table */}
            {isLoading ? (
              <div className="space-y-2">
                {Array.from({ length: 4 }).map((_, i) => (
                  <Skeleton key={i} className="h-14 w-full rounded-lg" />
                ))}
              </div>
            ) : filtered.length === 0 ? (
              <p className="text-center py-10 text-zinc-500 text-sm">No predictions found.</p>
            ) : (
              <div className="rounded-xl border border-zinc-800 overflow-hidden">
                {/* Header */}
                <div className="hidden sm:grid grid-cols-7 px-4 py-2.5 bg-zinc-900 text-[10px] text-zinc-500 uppercase tracking-wider border-b border-zinc-800">
                  <span className="col-span-2">Pool</span>
                  <span>Outcome</span>
                  <span>Stake</span>
                  <span>Status</span>
                  <span className="col-span-2">Claim</span>
                </div>
                {/* Rows */}
                {filtered.map((claim) => {
                  const status = predictionStatus(claim);
                  return (
                    <div
                      key={claim.prediction_id}
                      className="grid grid-cols-2 sm:grid-cols-7 items-center gap-2 px-4 py-3 border-b border-zinc-800/50 last:border-0 hover:bg-white/[0.02] transition-colors"
                    >
                      <div className="col-span-2 sm:col-span-2 min-w-0">
                        <p className="text-sm text-white truncate">{claim.pool_name}</p>
                      </div>
                      <span className="text-sm text-zinc-300 hidden sm:block">
                        Outcome {claim.outcome}
                      </span>
                      <span className="text-sm font-mono text-zinc-300 hidden sm:block">
                        {formatChartValue(claim.amount)}
                      </span>
                      <span
                        className={cn(
                          "text-[10px] font-semibold px-2 py-1 rounded-full w-fit",
                          STATUS_STYLES[status],
                        )}
                      >
                        {status}
                      </span>
                      <div className="col-span-2 hidden sm:block">
                        <ClaimBadge claim={claim} />
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
