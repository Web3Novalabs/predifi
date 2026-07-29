"use client";

import { Box } from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { usePools } from "@/lib/hooks/usePools";

/**
 * ActivePoolsMetricCard
 *
 * Displays the live count of active pools.
 *
 * Cache-key alignment (issue #1406)
 * ──────────────────────────────────
 * Previously this component called usePools({ status: "active", limit: 1 })
 * to obtain `total`, which produced a *different* SWR cache key from the
 * PoolsList component that calls usePools({ status: "active", sort_by: "new" }).
 * That meant two separate HTTP requests fired for the same underlying data.
 *
 * Fix: request the same key as PoolsList by passing the same default query
 * ({ status: "active", sort_by: "new" }). SWR deduplicates the request —
 * both components share one cache entry and one in-flight fetch.
 *
 * The `total` field returned by the backend reflects the full count
 * regardless of the `limit`/`offset` params, so no information is lost.
 */

interface ActivePoolsMetricCardProps {
  isLoading?: boolean;
}

export function ActivePoolsMetricCard({
  isLoading: forceLoading = false,
}: ActivePoolsMetricCardProps) {
  // Intentionally matches PoolsList's query so both components share one SWR
  // cache entry. Changing this key will re-introduce the duplicate fetch.
  const { total, isLoading, isError } = usePools({
    status: "active",
    sort_by: "new",
  });

  return (
    <MetricCard
      title="Active Pools"
      value={isError ? "—" : total.toLocaleString()}
      icon={<Box />}
      change={isError ? "Count unavailable" : `${total} live now`}
      changeType={isError ? "neutral" : "positive"}
      tooltip="Prediction pools that are currently open for participation."
      isLoading={forceLoading || isLoading}
    />
  );
}
