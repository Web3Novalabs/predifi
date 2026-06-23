/**
 * Dashboard page
 *
 * Performance strategy — above vs. below the fold:
 *
 * Above the fold (eagerly imported):
 *   - MetricCard ×4 — the four KPI cards are the first thing a user reads
 *
 * Below the fold (lazily imported via next/dynamic):
 *   - BalanceSection  — charts section; requires scrolling on most viewports
 *   - StakedChart     — heavy recharts dependency; deferred to keep TTI low
 *   - PredictionList  — "use client" interactive list; below the charts
 *   - PoolsList       — below the charts
 *
 * `next/dynamic` is Next.js's built-in wrapper around React.lazy + Suspense
 * that works correctly in both Server and Client Component trees. Using
 * React.lazy directly in a Server Component is not supported by the App Router,
 * so next/dynamic is the idiomatic equivalent here.
 */

import { Suspense } from "react";
import dynamic from "next/dynamic";
import { Diamond, Box, Activity, ShieldCheck } from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { formatStakeCompact } from "@/lib/stakeFilters";

// ---------------------------------------------------------------------------
// Below-the-fold dashboard components — loaded lazily
// ---------------------------------------------------------------------------

/**
 * BalanceSection — total balance card with withdrawal/claim buttons.
 * Sits in the charts row which is below the metric cards on most screens.
 */
const BalanceSection = dynamic(
  () =>
    import("@/components/dashboard/BalanceSection").then(
      (mod) => mod.BalanceSection,
    ),
  {
    loading: () => (
      <div
        className="h-[320px] w-full animate-pulse bg-zinc-800/50 rounded-xl"
        aria-hidden="true"
      />
    ),
  },
);

/**
 * StakedChart — recharts BarChart; the recharts library is sizeable so
 * deferring it meaningfully reduces the initial JS bundle.
 */
const StakedChart = dynamic(
  () =>
    import("@/components/dashboard/StakedChart").then((mod) => mod.StakedChart),
  {
    loading: () => (
      <div
        className="h-[320px] w-full animate-pulse bg-zinc-800/50 rounded-xl"
        aria-hidden="true"
      />
    ),
  },
);

/**
 * PredictionList — "use client" tabbed list; below the charts section.
 */
const PredictionList = dynamic(
  () =>
    import("@/components/dashboard/PredictionList").then(
      (mod) => mod.PredictionList,
    ),
  {
    loading: () => (
      <div
        className="h-[300px] w-full animate-pulse bg-zinc-800/50 rounded-xl"
        aria-hidden="true"
      />
    ),
  },
);

/**
 * PoolsList — below the charts section; deferred alongside PredictionList.
 */
const PoolsList = dynamic(
  () => import("@/components/dashboard/PoolsList").then((mod) => mod.PoolsList),
  {
    loading: () => (
      <div
        className="h-[400px] w-full animate-pulse bg-zinc-800/50 rounded-xl"
        aria-hidden="true"
      />
    ),
  },
);

// ---------------------------------------------------------------------------

export default function DashboardPage() {
  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8 space-y-8">
      {/* Header */}
      <div className="space-y-1">
        <h1 className="text-3xl font-bold text-white">My Dashboard</h1>
        <p className="text-zinc-400 text-sm">
          Lorem ipsum dolor sit amet consectetur. Non eget non odio lobortis
          odio.
        </p>
      </div>

      {/* Metric Cards — above the fold; eagerly loaded */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          title="Total Earned"
          value={formatStakeCompact(1255)}
          icon={<Diamond />}
          change="65% increase"
          changeType="positive"
        />
        <MetricCard
          title="Active Pool"
          value="75"
          icon={<Box />}
          change="+7 new add"
          changeType="positive"
        />
        <MetricCard
          title="Win Rate"
          value="65%"
          icon={<Activity />}
          change="7.8% Growth"
          changeType="positive"
        />
        <MetricCard
          title="Reputation Score"
          value={
            <span className="flex items-end gap-1">
              <span className="text-[#84CC16]">3.5</span>
              <span className="text-lg text-zinc-500 font-normal mb-1">
                /5.0
              </span>
            </span>
          }
          icon={<ShieldCheck />}
          change="70% accuracy"
          changeType="neutral"
        />
      </div>

      {/* Charts Section — below the fold; lazily loaded */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-1 h-[320px]">
          <Suspense
            fallback={
              <div
                className="h-[320px] w-full animate-pulse bg-zinc-800/50 rounded-xl"
                aria-hidden="true"
              />
            }
          >
            <BalanceSection />
          </Suspense>
        </div>
        <div className="lg:col-span-2 h-[320px]">
          <Suspense
            fallback={
              <div
                className="h-[320px] w-full animate-pulse bg-zinc-800/50 rounded-xl"
                aria-hidden="true"
              />
            }
          >
            <StakedChart />
          </Suspense>
        </div>
      </div>

      {/* Activity Section — below the fold; lazily loaded */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-1">
          <Suspense
            fallback={
              <div
                className="h-[300px] w-full animate-pulse bg-zinc-800/50 rounded-xl"
                aria-hidden="true"
              />
            }
          >
            <PredictionList />
          </Suspense>
        </div>
        <div className="lg:col-span-2">
          <Suspense
            fallback={
              <div
                className="h-[400px] w-full animate-pulse bg-zinc-800/50 rounded-xl"
                aria-hidden="true"
              />
            }
          >
            <PoolsList />
          </Suspense>
        </div>
      </div>
    </div>
  );
}
