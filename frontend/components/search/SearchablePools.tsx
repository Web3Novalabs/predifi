"use client";

import { useCallback, useMemo } from "react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Skeleton,
  SearchBar,
} from "@/components/ui";
import { SearchResultHighlighter } from "@/components/search/SearchResultHighlighter";
import { usePools } from "@/lib/hooks/usePools";
import { usePoolsQuery, type SortBy, type PoolStatus } from "@/lib/hooks/usePoolsQuery";
import type { Pool } from "@/lib/api/pools";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SearchablePoolsProps {
  /**
   * Override loading state from a parent (e.g. skeleton screens for SSR).
   * Takes priority over the SWR loading state.
   */
  isLoading?: boolean;
  /** Force the skeleton state regardless of fetch status. */
  forceLoading?: boolean;
  /**
   * Default sort order. Passed to {@link usePoolsQuery} as `defaultSortBy`.
   * Defaults to `"new"`.
   */
  defaultSortBy?: SortBy;
  /**
   * Default lifecycle status. Passed to {@link usePoolsQuery} as
   * `defaultStatus`. Defaults to `"active"`.
   */
  defaultStatus?: PoolStatus;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SORT_OPTIONS: { value: SortBy; label: string }[] = [
  { value: "new", label: "Newest" },
  { value: "popular", label: "Popular" },
  { value: "ending_soon", label: "Ending soon" },
];

const STATUS_OPTIONS: { value: PoolStatus; label: string }[] = [
  { value: "active", label: "Active" },
  { value: "closed", label: "Closed" },
  { value: "settled", label: "Settled" },
];

// ---------------------------------------------------------------------------
// SearchablePools
// ---------------------------------------------------------------------------

/**
 * SearchablePools
 *
 * A self-contained pool browser that combines:
 *   - URL-synced filter/sort/pagination state via {@link usePoolsQuery}
 *   - Server-side sorted + filtered pool fetching via {@link usePools}
 *   - Client-side text search applied on top of the server result set,
 *     using the debounced search value from {@link usePoolsQuery} so the
 *     input doesn't stutter while the user types
 *
 * All filter state is reflected in the URL search params (`?q=`, `?sort=`,
 * `?status=`, `?category=`) so searches are bookmarkable and the browser
 * back/forward buttons work as expected.
 */
export function SearchablePools({
  isLoading = false,
  forceLoading = false,
  defaultSortBy = "new",
  defaultStatus = "active",
}: SearchablePoolsProps) {
  // ── Query state ─────────────────────────────────────────────────────────
  const {
    search,
    debouncedSearch,
    sortBy,
    status,
    poolsQuery,
    setSearch,
    setSortBy,
    setStatus,
  } = usePoolsQuery({ defaultSortBy, defaultStatus });

  // ── Data fetching ────────────────────────────────────────────────────────
  const {
    pools,
    total,
    isLoading: isPoolsLoading,
    isError,
    refresh,
  } = usePools(poolsQuery);

  // ── Client-side text filter ──────────────────────────────────────────────
  // We apply an in-memory text filter on top of the server's response.
  // This covers name, category, and token matching. Once the backend adds
  // full-text search, this memo can be removed and `pools` used directly.
  const filteredPools = useMemo(() => {
    const q = debouncedSearch.toLowerCase().trim();
    if (!q) return pools;
    return pools.filter(
      (pool) =>
        pool.name.toLowerCase().includes(q) ||
        pool.category.toLowerCase().includes(q) ||
        pool.token.toLowerCase().includes(q),
    );
  }, [pools, debouncedSearch]);

  // ── Search handler ───────────────────────────────────────────────────────
  const handleSearch = useCallback(
    (value: string) => setSearch(value),
    [setSearch],
  );

  // ── Skeleton items (memoised to prevent re-allocation) ───────────────────
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

  // ── Loading state ────────────────────────────────────────────────────────
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

  // ── Main render ──────────────────────────────────────────────────────────
  return (
    <Card className="bg-[#121212] border-none text-white h-full min-h-[400px]">
      <CardHeader className="space-y-4">
        {/* Title row */}
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-1">
            <CardTitle className="text-lg font-medium">Prediction Pools</CardTitle>
            <p className="text-sm text-zinc-400">
              Search by name, category, or token. Filters are reflected in the URL.
            </p>
          </div>
          <span className="inline-flex items-center justify-center rounded-full bg-[#37B7C3]/10 px-3 py-1 text-xs font-medium text-[#7DE3EC] sm:shrink-0">
            {total.toLocaleString()}{" "}
            {status === "active" ? "active" : status === "closed" ? "closed" : "settled"}
          </span>
        </div>

        {/* Search input */}
        <SearchBar
          placeholder="Search pools by name, category, token…"
          value={search}
          onSearch={handleSearch}
          aria-label="Search pools"
          className="w-full"
        />

        {/* Filter bar — sort + status */}
        <FilterBar
          sortBy={sortBy}
          status={status}
          onSortChange={setSortBy}
          onStatusChange={setStatus}
        />
      </CardHeader>

      <CardContent>
        {isError ? (
          <ErrorState onRetry={refresh} />
        ) : filteredPools.length === 0 ? (
          <EmptyState query={debouncedSearch} />
        ) : (
          <div className="space-y-3">
            {filteredPools.map((pool) => (
              <PoolRow key={pool.pool_id} pool={pool} query={debouncedSearch} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// FilterBar
// ---------------------------------------------------------------------------

interface FilterBarProps {
  sortBy: SortBy;
  status: PoolStatus;
  onSortChange: (value: SortBy) => void;
  onStatusChange: (value: PoolStatus) => void;
}

function FilterBar({ sortBy, status, onSortChange, onStatusChange }: FilterBarProps) {
  return (
    <div
      className="flex flex-wrap gap-2"
      role="group"
      aria-label="Pool filters"
    >
      {/* Sort buttons */}
      <div
        className="flex items-center gap-1 rounded-lg bg-zinc-900 p-1"
        role="group"
        aria-label="Sort order"
      >
        {SORT_OPTIONS.map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => onSortChange(opt.value)}
            aria-pressed={sortBy === opt.value}
            className={[
              "rounded-md px-3 py-1 text-xs font-medium transition-colors",
              sortBy === opt.value
                ? "bg-[#37B7C3]/20 text-[#7DE3EC]"
                : "text-zinc-500 hover:text-zinc-300",
            ].join(" ")}
          >
            {opt.label}
          </button>
        ))}
      </div>

      {/* Status buttons */}
      <div
        className="flex items-center gap-1 rounded-lg bg-zinc-900 p-1"
        role="group"
        aria-label="Pool status"
      >
        {STATUS_OPTIONS.map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => onStatusChange(opt.value)}
            aria-pressed={status === opt.value}
            className={[
              "rounded-md px-3 py-1 text-xs font-medium transition-colors",
              status === opt.value
                ? "bg-[#37B7C3]/20 text-[#7DE3EC]"
                : "text-zinc-500 hover:text-zinc-300",
            ].join(" ")}
          >
            {opt.label}
          </button>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PoolRow
// ---------------------------------------------------------------------------

function PoolRow({ pool, query }: { pool: Pool; query: string }) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-lg bg-zinc-900/50 p-3 transition-colors hover:bg-zinc-900/80">
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
      <PoolStateChip state={pool.state} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// PoolStateChip
// ---------------------------------------------------------------------------

const STATE_STYLES: Record<string, string> = {
  active: "bg-emerald-400/10 text-emerald-400",
  closed: "bg-zinc-400/10 text-zinc-400",
  settled: "bg-violet-400/10 text-violet-400",
};

function PoolStateChip({ state }: { state: string }) {
  const cls = STATE_STYLES[state] ?? "bg-zinc-400/10 text-zinc-400";
  return (
    <span
      className={`shrink-0 rounded-full px-3 py-1 text-xs font-medium capitalize ${cls}`}
    >
      {state}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Empty / Error states
// ---------------------------------------------------------------------------

function EmptyState({ query }: { query: string }) {
  return (
    <div className="flex items-center justify-center h-[300px] text-zinc-600">
      <p>
        {query
          ? `No pools match "${query}".`
          : "No pools are available right now."}
      </p>
    </div>
  );
}

function ErrorState({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center h-[300px] gap-3 text-zinc-500">
      <p>Couldn&apos;t load pools.</p>
      <button
        type="button"
        onClick={onRetry}
        className="text-sm font-medium text-[#37B7C3] hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3] rounded"
      >
        Try again
      </button>
    </div>
  );
}
