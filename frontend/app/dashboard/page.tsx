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
import type { Metadata } from "next";

export const metadata: Metadata = { title: "Dashboard" };
import { DashboardMetrics } from "@/components/dashboard/DashboardMetrics";

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
      <DashboardMetrics />

      {/* Charts Section — below the fold; lazily loaded */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/*
          Balance card: taller on small screens (content needs room), shorter
          on large screens where the layout is side-by-side.
        */}
        <div className="lg:col-span-1 h-[280px] sm:h-[300px] lg:h-[360px]">
          <Suspense
            fallback={
              <div
                className="h-full w-full animate-pulse bg-zinc-800/50 rounded-xl"
                aria-hidden="true"
              />
            }
          >
            <BalanceSection />
          </Suspense>
        </div>
        {/*
          Chart card: more vertical space on large screens so bars are clearly
          readable. On mobile the stacked layout already gives good width so a
          moderate height is fine.
        */}
        <div className="lg:col-span-2 h-[280px] sm:h-[300px] lg:h-[360px]">
          <Suspense
            fallback={
              <div
                className="h-full w-full animate-pulse bg-zinc-800/50 rounded-xl"
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
