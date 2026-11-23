"use client";

import { useCallback, useMemo, useState } from "react";
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
import { useCategories } from "@/lib/hooks/useCategories";
import { formatCategory, type CategoryCount } from "@/lib/api/categories";
import {
  usePoolsQuery,
  type AdvancedFilters,
  type SortBy,
  type PoolStatus,
} from "@/lib/hooks/usePoolsQuery";
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
  { value: "stake_desc", label: "Stake ↓" },
  { value: "stake_asc", label: "Stake ↑" },
  { value: "ending_soon", label: "Ending soon" },
  { value: "ending_latest", label: "Ending last" },
];

const STATUS_OPTIONS: { value: PoolStatus; label: string }[] = [
  { value: "active", label: "Active" },
  { value: "resolved", label: "Resolved" },
  { value: "cancelled", label: "Cancelled" },
];

/** Human label for a status filter value, including legacy aliases. */
function statusLabel(status: PoolStatus): string {
  switch (status) {
    case "settled":
    case "resolved":
      return "resolved";
    case "closed":
    case "cancelled":
      return "cancelled";
    default:
      return "active";
  }
}

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
    advanced,
    page,
    pageSize,
    poolsQuery,
    setAdvanced,
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
  const { categories: availableCategories } = useCategories();

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
    (sortBy !== defaultSortBy ? 1 : 0) +
    countAdvancedFilters(advanced);

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
              {statusLabel(status)}
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
          categories={availableCategories}
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

        {/* Advanced filters: token, creator, stake range, end-time range */}
        <AdvancedFilterPanel value={advanced} onChange={setAdvanced} />
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
                <PoolRow
                  key={pool.pool_id}
                  pool={pool}
                  query={debouncedSearch}
                  onSelectCategory={setCategory}
                  onToggleTag={toggleTag}
                />
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
  categories: CategoryCount[];
  selected: string;
  onSelect: (category: string) => void;
}

function CategoryFilter({ categories, selected, onSelect }: CategoryFilterProps) {
  const selectedKey = selected.toLowerCase();
  const options: { value: string; label: string; count?: number }[] = [
    { value: "", label: "All" },
    ...categories.map((c) => ({
      value: c.category,
      label: formatCategory(c.category),
      count: c.count,
    })),
  ];
  return (
    <div
      className="flex flex-wrap gap-1.5"
      role="group"
      aria-label="Category filter"
    >
      {options.map((opt) => {
        const active = selectedKey === opt.value;
        return (
          <button
            key={opt.value || "all"}
            type="button"
            onClick={() => onSelect(active ? "" : opt.value)}
            aria-pressed={active}
            className={[
              "rounded-full px-3 py-1 text-xs font-medium transition-colors",
              active
                ? "bg-[#37B7C3]/20 text-[#7DE3EC] ring-1 ring-[#37B7C3]/40"
                : "bg-zinc-900 text-zinc-500 hover:text-zinc-300",
            ].join(" ")}
          >
            {opt.label}
            {opt.count ? <span className="pl-1 text-zinc-600">{opt.count}</span> : null}
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
// AdvancedFilterPanel
// ---------------------------------------------------------------------------

function countAdvancedFilters(a: AdvancedFilters): number {
  return (
    (a.token ? 1 : 0) +
    (a.creator ? 1 : 0) +
    (a.minStake != null ? 1 : 0) +
    (a.maxStake != null ? 1 : 0) +
    (a.endsAfter ? 1 : 0) +
    (a.endsBefore ? 1 : 0)
  );
}

interface AdvancedFilterPanelProps {
  value: AdvancedFilters;
  onChange: (changes: Partial<AdvancedFilters>) => void;
}

const FIELD_CLASS =
  "w-full rounded-md bg-zinc-900 px-2.5 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3]";

function toStake(raw: string): number | null {
  if (raw.trim() === "") return null;
  const n = Number(raw);
  return Number.isFinite(n) && n >= 0 ? Math.floor(n) : null;
}

function AdvancedFilterPanel({ value, onChange }: AdvancedFilterPanelProps) {
  const [open, setOpen] = useState(() => countAdvancedFilters(value) > 0);
  const rangeInvalid =
    value.minStake != null && value.maxStake != null && value.minStake > value.maxStake;
  const datesInvalid =
    value.endsAfter !== "" && value.endsBefore !== "" && value.endsAfter > value.endsBefore;

  return (
    <div className="space-y-2">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        aria-controls="advanced-pool-filters"
        className="text-xs font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
      >
        {open ? "Hide" : "Show"} advanced filters
      </button>
      {open && (
        <div
          id="advanced-pool-filters"
          className="grid gap-3 sm:grid-cols-2"
          role="group"
          aria-label="Advanced pool filters"
        >
          <label className="space-y-1 text-xs text-zinc-500">
            Token
            <input
              type="text"
              className={FIELD_CLASS}
              placeholder="Token contract address"
              value={value.token}
              onChange={(e) => onChange({ token: e.target.value })}
            />
          </label>
          <label className="space-y-1 text-xs text-zinc-500">
            Creator
            <input
              type="text"
              className={FIELD_CLASS}
              placeholder="Creator wallet address"
              value={value.creator}
              onChange={(e) => onChange({ creator: e.target.value })}
            />
          </label>
          <label className="space-y-1 text-xs text-zinc-500">
            Min stake
            <input
              type="number"
              min={0}
              inputMode="numeric"
              className={FIELD_CLASS}
              value={value.minStake ?? ""}
              aria-invalid={rangeInvalid}
              onChange={(e) => onChange({ minStake: toStake(e.target.value) })}
            />
          </label>
          <label className="space-y-1 text-xs text-zinc-500">
            Max stake
            <input
              type="number"
              min={0}
              inputMode="numeric"
              className={FIELD_CLASS}
              value={value.maxStake ?? ""}
              aria-invalid={rangeInvalid}
              onChange={(e) => onChange({ maxStake: toStake(e.target.value) })}
            />
          </label>
          <label className="space-y-1 text-xs text-zinc-500">
            Ends on or after
            <input
              type="date"
              className={FIELD_CLASS}
              value={value.endsAfter}
              aria-invalid={datesInvalid}
              onChange={(e) => onChange({ endsAfter: e.target.value })}
            />
          </label>
          <label className="space-y-1 text-xs text-zinc-500">
            Ends on or before
            <input
              type="date"
              className={FIELD_CLASS}
              value={value.endsBefore}
              aria-invalid={datesInvalid}
              onChange={(e) => onChange({ endsBefore: e.target.value })}
            />
          </label>
          {(rangeInvalid || datesInvalid) && (
            <p role="alert" className="text-xs text-red-400 sm:col-span-2">
              {rangeInvalid
                ? "Min stake must not exceed max stake."
                : "The start date must not be after the end date."}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// PoolRow
// ---------------------------------------------------------------------------

interface PoolRowProps {
  pool: Pool;
  query: string;
  onSelectCategory: (category: string) => void;
  onToggleTag: (tag: string) => void;
}

function PoolRow({ pool, query, onSelectCategory, onToggleTag }: PoolRowProps) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-lg bg-zinc-900/50 p-3 transition-colors hover:bg-zinc-900/80">
      <div className="min-w-0 space-y-1">
        <p className="truncate text-sm font-medium text-white">
          <SearchResultHighlighter text={pool.name} searchQuery={query} />
        </p>
        <p className="text-xs text-zinc-500">
          <button
            type="button"
            onClick={() => onSelectCategory(pool.category)}
            className="rounded hover:text-zinc-300 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3]"
            aria-label={`Filter by category ${formatCategory(pool.category)}`}
          >
            <SearchResultHighlighter
              text={formatCategory(pool.category)}
              searchQuery={query}
            />
          </button>
          {" · "}
          {pool.total_stake.toLocaleString()} {pool.token}
          {pool.prediction_count != null && (
            <>
              {" · "}
              {pool.prediction_count.toLocaleString()} prediction
              {pool.prediction_count === 1 ? "" : "s"}
            </>
          )}
        </p>
        {pool.tags.length > 0 && (
          <div className="flex flex-wrap gap-1 pt-0.5">
            {pool.tags.slice(0, 5).map((tag) => (
              <button
                key={tag}
                type="button"
                onClick={() => onToggleTag(tag)}
                aria-label={`Filter by tag ${tag}`}
                className="inline-flex items-center rounded-full bg-zinc-800 px-2 py-px text-[10px] font-medium text-zinc-400 hover:text-zinc-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3]"
              >
                #{tag}
              </button>
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

/** Display names for stored pool states (cancelled pools are stored as closed). */
const STATE_LABELS: Record<string, string> = {
  closed: "cancelled",
  settled: "resolved",
};

function PoolStateChip({ state }: { state: string }) {
  const cls = STATE_STYLES[state] ?? "bg-zinc-400/10 text-zinc-400";
  return (
    <span
      className={`shrink-0 rounded-full px-3 py-1 text-xs font-medium capitalize ${cls}`}
    >
      {STATE_LABELS[state] ?? state}
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
