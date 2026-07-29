"use client";

/**
 * DashboardMetrics
 *
 * Re-render fix (issue #1406)
 * ───────────────────────────
 * Previously used a `setTimeout` inside a `useEffect` to simulate a 700 ms
 * loading state, then rendered hardcoded placeholder values. This caused:
 *   • A fake loading flash on every mount (700 ms regardless of network speed)
 *   • `isLoading` state living locally in this component, duplicating what SWR
 *     already tracks for the pools fetch that ActivePoolsMetricCard performs
 *
 * Fix: remove the local setTimeout. Derive `isLoading` from the SWR hook that
 * already serves this data — `usePools` with the same key used by PoolsList
 * and ActivePoolsMetricCard. When pools are still fetching, all four metric
 * cards show their skeleton together; when the fetch completes they all
 * hydrate at once.
 *
 * NOTE: Total Earned, Win Rate, and Reputation Score are still placeholder
 * values — those will be replaced when the profile API is wired to the
 * dashboard (tracked separately). The isLoading state is now correctly derived
 * from real network activity rather than an artificial timer.
 */

import { Activity, Diamond, ShieldCheck } from "lucide-react";
import { ActivePoolsMetricCard } from "@/components/dashboard/ActivePoolsMetricCard";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { usePools } from "@/lib/hooks/usePools";
import { formatStakeCompact } from "@/lib/stakeFilters";

export function DashboardMetrics() {
  // Derive loading state from the SWR fetch that backs ActivePoolsMetricCard
  // (same cache key: { status: "active", sort_by: "new" }). All four metric
  // cards will show their skeleton while the request is in-flight and will
  // hydrate together once it completes — no artificial delay.
  const { isLoading } = usePools({ status: "active", sort_by: "new" });

  return (
    <div
      className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4"
      aria-busy={isLoading}
      aria-live="polite"
      role="status"
    >
      <MetricCard
        title="Total Earned"
        value={formatStakeCompact(1255)}
        icon={<Diamond />}
        change="65% increase"
        changeType="positive"
        isLoading={isLoading}
      />
      {/* ActivePoolsMetricCard reads the same SWR key — no duplicate fetch */}
      <ActivePoolsMetricCard isLoading={isLoading} />
      <MetricCard
        title="Win Rate"
        value="65%"
        icon={<Activity />}
        change="7.8% Growth"
        changeType="positive"
        isLoading={isLoading}
      />
      <MetricCard
        title="Reputation Score"
        value={
          <span className="flex items-end gap-1">
            <span className="text-[#84CC16]">3.5</span>
            <span className="text-lg text-zinc-500 font-normal mb-1">/5.0</span>
          </span>
        }
        icon={<ShieldCheck />}
        change="70% accuracy"
        changeType="neutral"
        isLoading={isLoading}
      />
    </div>
  );
}
