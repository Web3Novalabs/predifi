"use client";

import { useMemo, useState, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { SearchBar } from "@/components/ui/search-bar";
import { SearchResultHighlighter } from "@/components/search/SearchResultHighlighter";
import { usePools } from "@/lib/hooks/usePools";
import type { Pool } from "@/lib/api/pools";

interface PoolsListProps {
  isLoading?: boolean;
}

function PoolRow({ pool, query }: { pool: Pool; query: string }) {
  return (
    <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/50">
      <div className="space-y-0.5">
        <p className="text-sm font-medium text-white">
          <SearchResultHighlighter text={pool.name} searchQuery={query} />
        </p>
        <p className="text-xs text-zinc-500">
          <SearchResultHighlighter text={pool.category} searchQuery={query} />
        </p>
      </div>
      <span className="text-xs px-2 py-1 rounded-full bg-zinc-800 text-zinc-400">
        {pool.state}
      </span>
    </div>
  );
}

export function PoolsList({ isLoading: forceLoading = false }: PoolsListProps) {
  const [query, setQuery] = useState("");
  const { pools, isLoading, isError, refresh } = usePools();

  const handleSearch = useCallback((value: string) => {
    setQuery(value);
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return pools;
    return pools.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.category.toLowerCase().includes(q)
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
    []
  );

  if (forceLoading || isLoading) {
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
        <CardTitle className="text-lg font-medium">Created Pools</CardTitle>
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
              onClick={refresh}
              className="text-sm font-medium text-[#37B7C3] hover:underline"
            >
              Try again
            </button>
          </div>
        ) : filtered.length === 0 ? (
          <div className="flex items-center justify-center h-[300px] text-zinc-600">
            <p>{query ? "No pools match your search." : "No pools created yet!"}</p>
          </div>
        ) : (
          <div className="space-y-3">
            {filtered.map((pool) => (
              <PoolRow key={pool.pool_id} pool={pool} query={query} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
