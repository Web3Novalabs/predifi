"use client";

import { useMemo, useState, useCallback } from "react";
import Link from "next/link";
import { cn } from "@/lib/utils";
import { Card, CardContent, SearchBar, Skeleton } from "@/components/ui";
import { SearchResultHighlighter } from "@/components/search/SearchResultHighlighter";
import { OddsCalculator } from "@/components/ui/odds-calculator";
import { usePools } from "@/lib/hooks/usePools";
import { useTags } from "@/lib/hooks/useTags";
import type { Pool } from "@/lib/api/pools";

const CATEGORIES = [
  "All",
  "Crypto",
  "Sports",
  "Politics",
  "Entertainment",
  "Technology",
  "Finance",
  "Science",
  "Gaming",
  "Other",
] as const;

const SORT_OPTIONS = [
  { value: "new", label: "Newest" },
  { value: "popular", label: "Most Staked" },
  { value: "ending_soon", label: "Ending Soon" },
] as const;

const STATUS_OPTIONS = [
  { value: "active", label: "Active" },
  { value: "closed", label: "Closed" },
  { value: "settled", label: "Settled" },
] as const;

type SortBy = (typeof SORT_OPTIONS)[number]["value"];
type PoolStatus = (typeof STATUS_OPTIONS)[number]["value"];

export function PoolMarketClient() {
  const [search, setSearch] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("All");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [sortBy, setSortBy] = useState<SortBy>("new");
  const [status, setStatus] = useState<PoolStatus>("active");
  const [page, setPage] = useState(0);
  const [showCalculator, setShowCalculator] = useState(false);

  const { tags: availableTags } = useTags();

  const queryTags = selectedTags.length > 0 ? selectedTags : undefined;
  const queryCategory = selectedCategory !== "All" ? selectedCategory : undefined;

  const {
    pools,
    total,
    isLoading: isPoolsLoading,
    isError,
    refresh,
  } = usePools({
    sort_by: sortBy,
    status,
    category: queryCategory,
    tags: queryTags,
    limit: 20,
    offset: page * 20,
  });

  const handleSearch = useCallback((value: string) => {
    setSearch(value.trim());
    setPage(0);
  }, []);

  const toggleTag = useCallback((tag: string) => {
    setSelectedTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag],
    );
    setPage(0);
  }, []);

  const setCategory = useCallback((cat: string) => {
    setSelectedCategory(cat);
    setPage(0);
  }, []);

  const changeSortBy = useCallback((s: SortBy) => {
    setSortBy(s);
    setPage(0);
  }, []);

  const changeStatus = useCallback((s: PoolStatus) => {
    setStatus(s);
    setPage(0);
  }, []);

  const filteredPools = useMemo(() => {
    const normalizedQuery = search.toLowerCase();
    if (!normalizedQuery) return pools;
    return pools.filter(
      (pool) =>
        pool.name.toLowerCase().includes(normalizedQuery) ||
        pool.category.toLowerCase().includes(normalizedQuery),
    );
  }, [pools, search]);

  const totalPages = Math.ceil(total / 20);
  const resultCount = search ? filteredPools.length : total;

  return (
    <main
      id="main-content"
      tabIndex={-1}
      className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8 space-y-6"
    >
      {/* Header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="space-y-1">
          <h1 className="text-3xl font-bold text-white">Pool Market</h1>
          <p className="text-zinc-400 text-sm">
            Browse pools by category and tags, or calculate your potential returns.
          </p>
        </div>
        <button
          type="button"
          onClick={() => setShowCalculator(!showCalculator)}
          aria-expanded={showCalculator}
          aria-controls="pool-odds-calculator"
          className={cn(
            "rounded-lg px-4 py-2 text-sm font-medium transition-colors border",
            showCalculator
              ? "bg-[#37B7C3]/15 border-[#37B7C3]/40 text-[#7DE3EC]"
              : "border-zinc-800 text-zinc-400 hover:text-zinc-300 hover:border-zinc-700",
          )}
        >
          {showCalculator ? "Hide Calculator" : "Odds Calculator"}
        </button>
      </div>

      {/* Odds Calculator (togglable) */}
      <div id="pool-odds-calculator" className="mx-auto max-w-3xl">
        {showCalculator && <OddsCalculator />}
      </div>

      {/* Filters */}
      <Card className="bg-[#121212] border-none text-white">
        <CardContent className="pt-6 space-y-4">
          {/* Search */}
          <SearchBar
            placeholder="Search pools by name or category…"
            onSearch={handleSearch}
            aria-label="Search pools"
          />

          {/* Categories */}
          <div className="space-y-2">
            <p id="category-filter-label" className="text-[10px] text-zinc-400 uppercase tracking-wider font-medium">Category</p>
            <div className="flex flex-wrap gap-2" role="group" aria-labelledby="category-filter-label">
              {CATEGORIES.map((cat) => (
                <button
                  key={cat}
                  type="button"
                  onClick={() => setCategory(cat)}
                  aria-pressed={selectedCategory === cat}
                  className={cn(
                    "rounded-full px-3 py-1.5 text-xs font-medium border transition-colors",
                    selectedCategory === cat
                      ? "bg-[#37B7C3]/15 border-[#37B7C3]/40 text-[#7DE3EC]"
                      : "border-zinc-800 text-zinc-400 hover:text-white hover:border-zinc-700",
                  )}
                >
                  {cat}
                </button>
              ))}
            </div>
          </div>

          {/* Tags */}
          {availableTags.length > 0 && (
            <div className="space-y-2">
              <p id="tag-filter-label" className="text-[10px] text-zinc-400 uppercase tracking-wider font-medium">Tags</p>
              <div className="flex flex-wrap gap-1.5" role="group" aria-labelledby="tag-filter-label">
                {availableTags.map((tag) => {
                  const active = selectedTags.includes(tag);
                  return (
                    <button
                      key={tag}
                      type="button"
                      onClick={() => toggleTag(tag)}
                      aria-pressed={active}
                      className={cn(
                        "rounded-full px-2.5 py-1 text-[11px] font-medium border transition-colors",
                        active
                          ? "bg-[#37B7C3]/15 border-[#37B7C3]/40 text-[#7DE3EC]"
                          : "border-zinc-800 text-zinc-400 hover:text-white hover:border-zinc-700",
                      )}
                    >
                      #{tag}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Sort & Status */}
          <div className="flex flex-wrap gap-4 items-center">
            <div className="flex items-center gap-2">
              <span id="pool-sort-label" className="text-xs text-zinc-400">Sort:</span>
              <div className="flex gap-1" role="group" aria-labelledby="pool-sort-label">
                {SORT_OPTIONS.map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => changeSortBy(opt.value)}
                    aria-pressed={sortBy === opt.value}
                    className={cn(
                      "rounded px-2.5 py-1 text-xs font-medium border transition-colors",
                      sortBy === opt.value
                        ? "bg-zinc-700/50 border-zinc-600 text-white"
                        : "border-zinc-800 text-zinc-400 hover:text-white",
                    )}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>
            </div>
            <div className="flex items-center gap-2">
              <span id="pool-status-label" className="text-xs text-zinc-400">Status:</span>
              <div className="flex gap-1" role="group" aria-labelledby="pool-status-label">
                {STATUS_OPTIONS.map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => changeStatus(opt.value)}
                    aria-pressed={status === opt.value}
                    className={cn(
                      "rounded px-2.5 py-1 text-xs font-medium border transition-colors",
                      status === opt.value
                        ? "bg-zinc-700/50 border-zinc-600 text-white"
                        : "border-zinc-800 text-zinc-400 hover:text-white",
                    )}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Pool count */}
      <p className="text-xs text-zinc-400" role="status" aria-live="polite" aria-atomic="true">
        {resultCount} pool{resultCount !== 1 ? "s" : ""} found
        {selectedCategory !== "All" && ` in "${selectedCategory}"`}
      </p>

      {/* Pool list */}
      <section className="space-y-3" aria-label="Pool results" aria-busy={isPoolsLoading}>
        {isPoolsLoading ? (
          <>
            <p className="sr-only" role="status">Loading pools…</p>
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} aria-hidden="true" className="flex items-center justify-between p-4 rounded-xl bg-zinc-900/50">
                <div className="space-y-2">
                  <Skeleton className="h-4 w-48" />
                  <Skeleton className="h-3 w-32" />
                </div>
                <Skeleton className="h-6 w-16 rounded-full" />
              </div>
            ))}
          </>
        ) : isError ? (
          <div role="alert" className="flex flex-col items-center justify-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900 py-16 text-zinc-400">
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
          <div className="flex items-center justify-center py-16 rounded-xl border border-zinc-800 bg-zinc-900 text-zinc-400">
            <p>{search ? "No pools match your search." : "No pools found for the selected filters."}</p>
          </div>
        ) : (
          filteredPools.map((pool) => (
            <PoolRow key={pool.pool_id} pool={pool} query={search} />
          ))
        )}
      </section>

      {/* Pagination */}
      {totalPages > 1 && (
        <nav className="flex items-center justify-center gap-2" aria-label="Pool pages">
          <button
            type="button"
            onClick={() => setPage((p) => Math.max(0, p - 1))}
            disabled={page === 0}
            className="rounded-lg px-3 py-1.5 text-sm font-medium border border-zinc-800 text-zinc-400 hover:text-zinc-300 disabled:opacity-40 disabled:cursor-not-allowed"
          >
            Previous
          </button>
          {Array.from({ length: Math.min(totalPages, 7) }).map((_, i) => {
            const pageNum = Math.max(0, Math.min(page - 3, totalPages - 7)) + i;
            if (pageNum >= totalPages) return null;
            return (
              <button
                key={pageNum}
                type="button"
                onClick={() => setPage(pageNum)}
                aria-current={page === pageNum ? "page" : undefined}
                aria-label={`Page ${pageNum + 1}`}
                className={cn(
                  "w-8 h-8 rounded text-sm font-medium transition-colors",
                  page === pageNum
                    ? "bg-[#37B7C3]/15 text-[#7DE3EC]"
                    : "text-zinc-400 hover:text-white",
                )}
              >
                {pageNum + 1}
              </button>
            );
          })}
          <button
            type="button"
            onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
            disabled={page >= totalPages - 1}
            className="rounded-lg px-3 py-1.5 text-sm font-medium border border-zinc-800 text-zinc-400 hover:text-zinc-300 disabled:opacity-40 disabled:cursor-not-allowed"
          >
            Next
          </button>
        </nav>
      )}
    </main>
  );
}

function PoolRow({ pool, query }: { pool: Pool; query: string }) {
  return (
    <Link
      href={`/user/pool-market/${pool.pool_id}`}
      aria-label={`${pool.name}, ${pool.category}, ${pool.total_stake.toLocaleString()} ${pool.token} staked, ${pool.state}`}
      className="flex items-center justify-between gap-3 rounded-xl bg-zinc-900/50 p-4 transition-colors hover:bg-zinc-900/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3] focus-visible:ring-offset-2 focus-visible:ring-offset-[#0A0A0A]"
    >
      <div className="min-w-0 space-y-1.5">
        <h2 className="truncate text-sm font-medium text-white">
          <SearchResultHighlighter text={pool.name} searchQuery={query} />
        </h2>
        <div className="flex items-center gap-2 text-xs text-zinc-400">
          <span className="rounded bg-zinc-800 px-1.5 py-0.5 font-medium text-zinc-400">
            {pool.category}
          </span>
          <span>{pool.total_stake.toLocaleString()} {pool.token}</span>
        </div>
        {pool.tags.length > 0 && (
          <p className="text-[10px] text-zinc-400 truncate">
            {pool.tags.map((t) => `#${t}`).join(" ")}
          </p>
        )}
      </div>
      <span
        className={cn(
          "shrink-0 rounded-full px-3 py-1 text-xs font-medium capitalize",
          pool.state === "active" && "bg-emerald-400/10 text-emerald-400",
          pool.state === "closed" && "bg-yellow-400/10 text-yellow-400",
          pool.state === "settled" && "bg-blue-400/10 text-blue-400",
        )}
      >
        {pool.state}
      </span>
    </Link>
  );
}