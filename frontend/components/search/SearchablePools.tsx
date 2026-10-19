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
import { useTags } from "@/lib/hooks/useTags";
import { usePoolsQuery, type SortBy, type PoolStatus } from "@/lib/hooks/usePoolsQuery";
import type { Pool } from "@/lib/api/pools";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SearchablePoolsProps {
  isLoading?: boolean;
  forceLoading?: boolean;
  defaultSortBy?: SortBy;
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

const CATEGORIES = [
  "All",
  "Crypto",
  "Sports",
  "Politics",
  "Finance",
  "Entertainment",
  "Technology",
  "Science",
  "Gaming",
  "Other",
];

// ---------------------------------------------------------------------------
// SearchablePools
// ---------------------------------------------------------------------------

/**
 * SearchablePools
 *
 * A self-contained pool browser that combines:
 *   - URL-synced filter/sort/pagination state via {@link usePoolsQuery}
 *   - Server-side full-text search (`?q=`) and filtering via {@link usePools}
 *   - Category, tag, sort, and status filters
 *   - Offset-based pagination controls
 *
 * All filter state is reflected in the URL search params so searches are
 * bookmarkable and the browser back/forward buttons work as expected.
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
    category,
    tags,
    page,
    pageSize,
    poolsQuery,
    setSearch,
    setSortBy,
    setStatus,
    setCategory,
    toggleTag,
    setPage,
    reset,
  } = usePoolsQuery({ defaultSortBy, defaultStatus });

  // ── Data fetching ────────────────────────────────────────────────────────
  const {
    pools,
    total,
    isLoading: isPoolsLoading,
    isError,
    refresh,
  } = usePools(poolsQuery);

  const { tags: availableTags } = useTags();

  // ── Search handler ───────────────────────────────────────────────────────
  const handleSearch = useCallback(
    (value: string) => setSearch(value),
    [setSearch],
  );

  // ── Pagination ───────────────────────────────────────────────────────────
  const totalPages = Math.ceil(total / pageSize);
  const hasPrev = page > 0;
  const hasNext = page < totalPages - 1;

  // ── Skeleton items ───────────────────────────────────────────────────────
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

  // ── Active filter count badge ────────────────────────────────────────────
  const activeFilters =
    (category ? 1 : 0) +
    tags.length +
    (debouncedSearch ? 1 : 0) +
    (status !== defaultStatus ? 1 : 0) +
    (sortBy !== defaultSortBy ? 1 : 0);

  // ── Main render ──────────────────────────────────────────────────────────
  return (
    <Card className="bg-[#121212] border-none text-white h-full min-h-[400px]">
      <CardHeader className="space-y-4">
        {/* Title row */}
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-1">
            <CardTitle className="text-lg font-medium">Prediction Pools</CardTitle>
            <p className="text-sm text-zinc-400">
              Search by name, category, tag, or token. Filters are bookmarkable via URL.
            </p>
          </div>
          <div className="flex items-center gap-2 sm:shrink-0">
            <span className="inline-flex items-center justify-center rounded-full bg-[#37B7C3]/10 px-3 py-1 text-xs font-medium text-[#7DE3EC]">
              {total.toLocaleString()}{" "}
              {status === "active" ? "active" : status === "closed" ? "closed" : "settled"}
            </span>
            {activeFilters > 0 && (
              <button
                type="button"
                onClick={reset}
                className="inline-flex items-center gap-1 rounded-full bg-zinc-800 px-2.5 py-1 text-xs font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
              >
                Clear {activeFilters} filter{activeFilters !== 1 ? "s" : ""}
              </button>
            )}
          </div>
        </div>

        {/* Search input */}
        <SearchBar
          placeholder="Full-text search by name, category, or tag…"
          value={search}
          onSearch={handleSearch}
          aria-label="Search pools"
          className="w-full"
        />

        {/* Category filter */}
        <CategoryFilter
          selected={category}
          onSelect={setCategory}
        />

        {/* Tag filter (only when tags exist) */}
        {availableTags.length > 0 && (
          <TagFilter
            availableTags={availableTags}
            activeTags={tags}
            onToggleTag={toggleTag}
          />
        )}

        {/* Sort + status filter bar */}
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
        ) : pools.length === 0 ? (
          <EmptyState query={debouncedSearch} />
        ) : (
          <>
            <div className="space-y-3">
              {pools.map((pool) => (
                <PoolRow key={pool.pool_id} pool={pool} query={debouncedSearch} />
              ))}
            </div>

            {/* Pagination controls */}
            {totalPages > 1 && (
              <div className="flex items-center justify-between pt-4 mt-4 border-t border-zinc-800">
                <p className="text-xs text-zinc-500">
                  Page {page + 1} of {totalPages} · {total.toLocaleString()} results
                </p>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => setPage(page - 1)}
                    disabled={!hasPrev}
                    className="rounded-md px-3 py-1.5 text-xs font-medium bg-zinc-900 text-zinc-300 hover:text-white disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                  >
                    ← Prev
                  </button>
                  <button
                    type="button"
                    onClick={() => setPage(page + 1)}
                    disabled={!hasNext}
                    className="rounded-md px-3 py-1.5 text-xs font-medium bg-zinc-900 text-zinc-300 hover:text-white disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                  >
                    Next →
                  </button>
                </div>
              </div>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// CategoryFilter
// ---------------------------------------------------------------------------

interface CategoryFilterProps {
  selected: string;
  onSelect: (category: string) => void;
}

function CategoryFilter({ selected, onSelect }: CategoryFilterProps) {
  return (
    <div
      className="flex flex-wrap gap-1.5"
      role="group"
      aria-label="Category filter"
    >
      {CATEGORIES.map((cat) => {
        const value = cat === "All" ? "" : cat;
        const active = selected === value;
        return (
          <button
            key={cat}
            type="button"
            onClick={() => onSelect(active ? "" : value)}
            aria-pressed={active}
            className={[
              "rounded-full px-3 py-1 text-xs font-medium transition-colors",
              active
                ? "bg-[#37B7C3]/20 text-[#7DE3EC] ring-1 ring-[#37B7C3]/40"
                : "bg-zinc-900 text-zinc-500 hover:text-zinc-300",
            ].join(" ")}
          >
            {cat}
          </button>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// TagFilter
// ---------------------------------------------------------------------------

interface TagFilterProps {
  availableTags: string[];
  activeTags: string[];
  onToggleTag: (tag: string) => void;
}

function TagFilter({ availableTags, activeTags, onToggleTag }: TagFilterProps) {
  return (
    <div
      className="flex flex-wrap gap-1.5"
      role="group"
      aria-label="Tag filter"
    >
      <span className="text-xs text-zinc-600 self-center pr-1">Tags:</span>
      {availableTags.slice(0, 20).map((tag) => {
        const active = activeTags.includes(tag);
        return (
          <button
            key={tag}
            type="button"
            onClick={() => onToggleTag(tag)}
            aria-pressed={active}
            className={[
              "rounded-full px-2.5 py-0.5 text-[11px] font-medium transition-colors",
              active
                ? "bg-indigo-500/20 text-indigo-300 ring-1 ring-indigo-500/40"
                : "bg-zinc-800/80 text-zinc-500 hover:text-zinc-300",
            ].join(" ")}
          >
            #{tag}
          </button>
        );
      })}
    </div>
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
        {pool.tags.length > 0 && (
          <div className="flex flex-wrap gap-1 pt-0.5">
            {pool.tags.slice(0, 5).map((tag) => (
              <span
                key={tag}
                className="inline-flex items-center rounded-full bg-zinc-800 px-2 py-px text-[10px] font-medium text-zinc-400"
              >
                #{tag}
              </span>
            ))}
          </div>
        )}
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
