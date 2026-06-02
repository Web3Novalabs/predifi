"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { usePools } from "@/lib/hooks/usePools";
import type { Pool } from "@/lib/api/pools";

interface PoolsListProps {
  /**
   * Force the loading skeleton regardless of fetch state — handy for demos and
   * visual tests. When omitted, the live SWR loading state is used.
   */
  isLoading?: boolean;
}

/** Compact, human-readable rendering of a stake amount. */
function formatStake(totalStake: number): string {
  return totalStake.toLocaleString();
}

/** A single row in the created-pools list. */
function PoolRow({ pool }: { pool: Pool }) {
  return (
    <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/50">
      <div className="space-y-1 min-w-0">
        <p className="text-sm font-medium text-white truncate">{pool.name}</p>
        <p className="text-xs text-zinc-500">
          {pool.category} · {formatStake(pool.total_stake)} {pool.token}
        </p>
      </div>
      <span className="shrink-0 text-xs font-medium px-3 py-1 rounded-full bg-zinc-800 text-zinc-300 capitalize">
        {pool.state}
      </span>
    </div>
  );
}

/**
 * PoolsList — the dashboard's "Created Pools" card.
 *
 * Pool data is fetched through {@link usePools}, which caches the response via
 * SWR. Because pool metadata is largely static, repeated mounts (e.g. navigating
 * back to the dashboard) render instantly from cache instead of refetching.
 */
export function PoolsList({ isLoading: isLoadingOverride }: PoolsListProps = {}) {
  // "Created pools" are static metadata — newest active pools, cached by SWR.
  const { pools, isLoading, isError, refresh } = usePools({
    status: "active",
    sort_by: "new",
  });

  const showSkeleton = isLoadingOverride ?? isLoading;

  if (showSkeleton) {
    return (
      <Card className="bg-[#121212] border-none text-white h-full min-h-[400px]">
        <CardHeader>
          <Skeleton className="h-5 w-32" />
        </CardHeader>
        <CardContent className="space-y-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/50">
              <div className="space-y-2">
                <Skeleton className="h-4 w-40" />
                <Skeleton className="h-3 w-24" />
              </div>
              <Skeleton className="h-6 w-16 rounded-full" />
            </div>
          ))}
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="bg-[#121212] border-none text-white h-full min-h-[400px]">
      <CardHeader>
        <CardTitle className="text-lg font-medium">Created Pools</CardTitle>
      </CardHeader>
      <CardContent>
        {isError ? (
          <div className="flex flex-col items-center justify-center h-[300px] gap-3 text-zinc-500">
            <p>Couldn&apos;t load pools.</p>
            <button
              onClick={refresh}
              className="text-sm font-medium text-[#37B7C3] hover:underline"
            >
              Try again
            </button>
          </div>
        ) : pools.length === 0 ? (
          <div className="flex items-center justify-center h-[300px] text-zinc-600">
            {/* Empty state placeholder */}
            <p>No pools created yet!</p>
          </div>
        ) : (
          <div className="space-y-3">
            {pools.map((pool) => (
              <PoolRow key={pool.pool_id} pool={pool} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
