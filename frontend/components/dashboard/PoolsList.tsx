"use client";

import { useMemo, useState, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle, Skeleton, SearchBar } from "@/components/ui";
import { SearchResultHighlighter } from "@/components/search/SearchResultHighlighter";
import { usePools } from "@/lib/hooks/usePools";
import type { Pool } from "@/lib/api/pools";

interface PoolsListProps {
  isLoading?: boolean;
  forceLoading?: boolean;
}

export function PoolsList({
  isLoading = false,
  forceLoading = false,
}: PoolsListProps) {
  const [query, setQuery] = useState("");
  const {
    pools,
    total,
    isLoading: isPoolsLoading,
    isError,
    refresh,
  } = usePools({
    status: "active",
    sort_by: "new",
  });
  const handleSearch = useCallback((value: string) => {
    setQuery(value.trim());
  }, []);
  const filteredPools = useMemo(() => {
    const normalizedQuery = query.toLowerCase();
    if (!normalizedQuery) return pools;

    return pools.filter(
      (pool) =>
        pool.name.toLowerCase().includes(normalizedQuery) ||
        pool.category.toLowerCase().includes(normalizedQuery),
    );
  }, [pools, query]);
  const skeletonItems = useMemo(
    () =>
      Array.from({ length: 4 }).map((_, i) => (
        <div
          key={i}
          className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/50"
        >
          <div className="space-y-2">
            <Skeleton className="h-4 w-40" />
            <Skeleton className="h-3 w-24" />
          </div>
          <Skeleton className="h-6 w-16 rounded-full" />
        </div>
      )),
    [],
  );

  if (forceLoading || isLoading || isPoolsLoading) {
    return (
      <Card className="bg-[#121212] border-none text-white h-full min-h-[400px]">
        <CardHeader>
          <Skeleton className="h-5 w-32" />
        </CardHeader>
        <CardContent className="space-y-3">{skeletonItems}</CardContent>
      </Card>
    );
  }

  return (
    <Card className="bg-[#121212] border-none text-white h-full min-h-[400px]">
      <CardHeader className="space-y-3">
        <div className="flex items-center justify-between gap-3">
          <CardTitle className="text-lg font-medium">Active Pools</CardTitle>
          <span className="rounded-full bg-[#37B7C3]/10 px-2.5 py-1 text-xs font-medium text-[#7DE3EC]">
            {total.toLocaleString()} active
          </span>
        </div>
        <SearchBar
          placeholder="Search pools…"
          onSearch={handleSearch}
          aria-label="Search pools"
        />
      </CardHeader>
      <CardContent>
        {isError ? (
          <div className="flex flex-col items-center justify-center h-[300px] gap-3 text-zinc-500">
            <p>Couldn&apos;t load pools.</p>
            <button
              type="button"
              onClick={refresh}
              className="text-sm font-medium text-[#37B7C3] hover:underline"
            >
              Try again
            </button>
          </div>
        ) : filteredPools.length === 0 ? (
          <div className="flex items-center justify-center h-[300px] text-zinc-600">
            <p>{query ? "No pools match your search." : "No active pools yet."}</p>
          </div>
        ) : (
          <div className="space-y-3">
            {filteredPools.map((pool) => (
              <PoolRow key={pool.pool_id} pool={pool} query={query} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function PoolRow({ pool, query }: { pool: Pool; query: string }) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-lg bg-zinc-900/50 p-3">
      <div className="min-w-0 space-y-1">
        <p className="truncate text-sm font-medium text-white">
          <SearchResultHighlighter text={pool.name} searchQuery={query} />
        </p>
        <p className="text-xs text-zinc-500">
          <SearchResultHighlighter text={pool.category} searchQuery={query} />
          {" · "}
          {pool.total_stake.toLocaleString()} {pool.token}
        </p>
      </div>
      <span className="shrink-0 rounded-full bg-emerald-400/10 px-3 py-1 text-xs font-medium capitalize text-emerald-400">
        {pool.state}
      </span>
    </div>
  );
}
