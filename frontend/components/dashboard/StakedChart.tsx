"use client";

/**
 * StakedChart
 *
 * Responsive bar chart card for the "Total staked" metric.
 *
 * Responsiveness strategy
 * ───────────────────────
 * A ResizeObserver (via useContainerSize) watches the CardContent element and
 * forwards its exact pixel dimensions to StakedChartRenderer.  This means the
 * chart redraws whenever:
 *   - The browser window is resized
 *   - A grid breakpoint changes the column width
 *   - A sidebar or panel opens/closes, changing the available space
 *
 * StakedChartRenderer is still code-split via next/dynamic so the recharts
 * library stays out of the initial bundle.
 */

import { useMemo } from "react";
import dynamic from "next/dynamic";
import { Card, CardContent, CardHeader, CardTitle, Button, Skeleton } from "@/components/ui";
import { ChevronDown } from "lucide-react";
import { useContainerSize } from "@/lib/hooks/useContainerSize";
import type { StakedChartRendererProps } from "@/components/dashboard/StakedChartRenderer";

/**
 * Dynamically import the recharts renderer so the library is code-split into
 * its own chunk.  The loading fallback mirrors the bar-chart skeleton so the
 * transition is seamless.
 */
const StakedChartRenderer = dynamic<StakedChartRendererProps>(
  () =>
    import("@/components/dashboard/StakedChartRenderer").then(
      (mod) => mod.StakedChartRenderer,
    ),
  {
    ssr: false,
    loading: () => (
      <div className="h-full w-full flex items-end gap-2 px-2 pb-2">
        {[55, 70, 55, 45, 90, 25, 75, 40, 70, 100, 70, 85].map((h, i) => (
          <Skeleton
            key={i}
            className="flex-1 rounded-t-sm"
            style={{ height: `${h}%` }}
          />
        ))}
      </div>
    ),
  },
);

interface StakedChartProps {
  isLoading?: boolean;
}

export function StakedChart({ isLoading = false }: StakedChartProps) {
  const skeletonHeights = useMemo(
    () => [55, 70, 55, 45, 90, 25, 75, 40, 70, 100, 70, 85],
    [],
  );

  // Measure the chart content area so we can pass exact pixel dimensions to
  // the recharts renderer — making the chart truly container-responsive.
  const { ref: chartAreaRef, width: chartWidth, height: chartHeight } =
    useContainerSize<HTMLDivElement>();

  if (isLoading) {
    return (
      <Card className="bg-[#121212] border-none text-white h-full">
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <Skeleton className="h-5 w-28" />
          <div className="flex items-center gap-2">
            <Skeleton className="h-7 w-12 rounded-full" />
            <Skeleton className="h-8 w-28 rounded-full" />
          </div>
        </CardHeader>
        <CardContent className="h-[240px] mt-4 flex items-end gap-2 px-6">
          {skeletonHeights.map((h, i) => (
            <Skeleton
              key={i}
              className="flex-1 rounded-t-sm"
              style={{ height: `${h}%` }}
            />
          ))}
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="bg-[#121212] border-none text-white h-full flex flex-col">
      <CardHeader className="flex flex-row items-center justify-between pb-2 shrink-0">
        <CardTitle className="text-lg font-medium">Total staked</CardTitle>
        <div className="flex items-center gap-2">
          <div className="bg-zinc-800/50 px-3 py-1.5 rounded-full text-xs font-medium text-white/80">
            23k
          </div>
          <Button
            className="bg-zinc-900 border-zinc-800 text-zinc-400 hover:text-white hover:bg-zinc-800 h-8 rounded-full text-xs"
            aria-label="Filter by time period"
          >
            This month
            <ChevronDown className="ml-2 w-3 h-3" aria-hidden="true" />
          </Button>
        </div>
      </CardHeader>

      {/*
        flex-1 + min-h-0 allows this div to shrink below its content height
        inside the flex column, giving the ResizeObserver accurate numbers.
        overflow-hidden prevents ResizeObserver feedback loops (scrollbar
        appearing → element grows → scrollbar disappears → repeat).
      */}
      <CardContent
        ref={chartAreaRef}
        className="flex-1 min-h-0 w-full mt-4 overflow-hidden"
      >
        <StakedChartRenderer width={chartWidth} height={chartHeight} />
      </CardContent>
    </Card>
  );
}
