import { useMemo } from "react";
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

export function PoolsList({ isLoading = false }: PoolsListProps) {
  const skeletonItems = useMemo(() => 
    Array.from({ length: 4 }).map((_, i) => (
      <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/50">
        <div className="space-y-2">
          <Skeleton className="h-4 w-40" />
          <Skeleton className="h-3 w-24" />
        </div>
        <Skeleton className="h-6 w-16 rounded-full" />
      </div>
    )),
    []
  );

  if (isLoading) {
    return (
      <Card className="bg-[#121212] border-none text-white h-full min-h-[400px]">
        <CardHeader>
          <Skeleton className="h-5 w-32" />
        </CardHeader>
        <CardContent className="space-y-3">
          {skeletonItems}
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
