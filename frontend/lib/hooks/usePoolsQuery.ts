"use client";

import { useCallback, useMemo } from "react";
import { useRouter, usePathname, useSearchParams } from "next/navigation";
import type { PoolsQuery } from "@/lib/api/pools";
import { useDebounce } from "@/lib/hooks/useDebounce";

/**
 * Valid sort options exposed in the UI.
 * Mirrors the `sort_by` field of {@link PoolsQuery}.
 */
export type SortBy = NonNullable<PoolsQuery["sort_by"]>;

/**
 * Valid lifecycle status options.
 * Mirrors the `status` field of {@link PoolsQuery}.
 */
export type PoolStatus = NonNullable<PoolsQuery["status"]>;

/** The shape of the query state managed by {@link usePoolsQuery}. */
export interface PoolsQueryState {
  /** Raw (un-debounced) search string entered by the user. */
  search: string;
  /** Debounced search string — safe to pass to {@link usePools}. */
  debouncedSearch: string;
  /** Current sort order. */
  sortBy: SortBy;
  /** Current lifecycle status filter. */
  status: PoolStatus;
  /** Current category filter (empty string = all categories). */
  category: string;
  /** Current page index (0-based). */
  page: number;
  /** Number of items per page. */
  pageSize: number;
}

/** Actions returned alongside the state to update it. */
export interface PoolsQueryActions {
  setSearch: (value: string) => void;
  setSortBy: (value: SortBy) => void;
  setStatus: (value: PoolStatus) => void;
  setCategory: (value: string) => void;
  /** Navigate to a specific page (0-based). */
  setPage: (page: number) => void;
  setPageSize: (size: number) => void;
  /** Reset all filters back to their defaults. */
  reset: () => void;
  /**
   * The {@link PoolsQuery} object ready to be passed to {@link usePools}.
   * Uses the debounced search to avoid a request on every keystroke.
   */
  poolsQuery: PoolsQuery;
}

/** Default configuration values for the query state. */
export interface UsePoolsQueryOptions {
  defaultSortBy?: SortBy;
  defaultStatus?: PoolStatus;
  defaultCategory?: string;
  defaultPageSize?: number;
  /** Debounce delay for the search field in milliseconds. Default: 350 ms. */
  searchDebounceMs?: number;
  /** Whether to sync query state to the URL as search params. Default: true. */
  syncUrl?: boolean;
}

const DEFAULT_SORT_BY: SortBy = "new";
const DEFAULT_STATUS: PoolStatus = "active";
const DEFAULT_PAGE_SIZE = 20;

/**
 * usePoolsQuery — centralised SWR query-parameter state for pool lists.
 *
 * Manages filter/sort/pagination state and (optionally) keeps it in sync with
 * the URL so that searches are bookmarkable and the browser back-button works.
 * Returns both the raw state and the derived {@link PoolsQuery} object ready
 * to hand to {@link usePools}.
 *
 * @example
 * const { poolsQuery, search, setSearch, setSortBy, setPage } = usePoolsQuery();
 * const { pools, total, isLoading } = usePools(poolsQuery);
 */
export function usePoolsQuery(options: UsePoolsQueryOptions = {}): PoolsQueryState & PoolsQueryActions {
  const {
    defaultSortBy = DEFAULT_SORT_BY,
    defaultStatus = DEFAULT_STATUS,
    defaultCategory = "",
    defaultPageSize = DEFAULT_PAGE_SIZE,
    searchDebounceMs = 350,
    syncUrl = true,
  } = options;

  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  // ---------------------------------------------------------------------------
  // Read current state from URL (or fall back to defaults)
  // ---------------------------------------------------------------------------
  const search = syncUrl ? (searchParams.get("q") ?? "") : "";
  const sortBy = (syncUrl ? (searchParams.get("sort") as SortBy | null) : null) ?? defaultSortBy;
  const status = (syncUrl ? (searchParams.get("status") as PoolStatus | null) : null) ?? defaultStatus;
  const category = syncUrl ? (searchParams.get("category") ?? defaultCategory) : defaultCategory;
  const page = syncUrl ? Number(searchParams.get("page") ?? 0) : 0;
  const pageSize = syncUrl ? Number(searchParams.get("limit") ?? defaultPageSize) : defaultPageSize;

  // Debounce the raw search value so we don't fire a new SWR request on every
  // keystroke — only after the user pauses for `searchDebounceMs`.
  const debouncedSearch = useDebounce(search, searchDebounceMs);

  // ---------------------------------------------------------------------------
  // URL helper — builds a new URLSearchParams replacing only the changed key
  // ---------------------------------------------------------------------------
  const buildParams = useCallback(
    (overrides: Record<string, string | number | null>): string => {
      const next = new URLSearchParams(searchParams.toString());
      for (const [key, value] of Object.entries(overrides)) {
        if (value === null || value === "" || value === 0) {
          next.delete(key);
        } else {
          next.set(key, String(value));
        }
      }
      return next.toString();
    },
    [searchParams],
  );

  const navigate = useCallback(
    (params: Record<string, string | number | null>) => {
      if (!syncUrl) return;
      const qs = buildParams(params);
      router.replace(`${pathname}${qs ? `?${qs}` : ""}`, { scroll: false });
    },
    [syncUrl, buildParams, router, pathname],
  );

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------
  const setSearch = useCallback(
    (value: string) => navigate({ q: value || null, page: null }),
    [navigate],
  );

  const setSortBy = useCallback(
    (value: SortBy) => navigate({ sort: value === defaultSortBy ? null : value, page: null }),
    [navigate, defaultSortBy],
  );

  const setStatus = useCallback(
    (value: PoolStatus) => navigate({ status: value === defaultStatus ? null : value, page: null }),
    [navigate, defaultStatus],
  );

  const setCategory = useCallback(
    (value: string) => navigate({ category: value || null, page: null }),
    [navigate],
  );

  const setPage = useCallback(
    (p: number) => navigate({ page: p > 0 ? p : null }),
    [navigate],
  );

  const setPageSize = useCallback(
    (size: number) => navigate({ limit: size !== defaultPageSize ? size : null, page: null }),
    [navigate, defaultPageSize],
  );

  const reset = useCallback(() => {
    if (syncUrl) {
      router.replace(pathname, { scroll: false });
    }
  }, [syncUrl, router, pathname]);

  // ---------------------------------------------------------------------------
  // Derived PoolsQuery — uses the *debounced* search to prevent request spam
  // ---------------------------------------------------------------------------
  const poolsQuery = useMemo<PoolsQuery>(() => {
    const q: PoolsQuery = {
      sort_by: sortBy,
      status,
      limit: pageSize,
      offset: page * pageSize,
    };
    if (category) q.category = category;
    // NOTE: The backend `/api/v1/pools` endpoint does not yet support a `q`
    // (text search) filter. We keep `debouncedSearch` in state so callers can
    // use it for client-side filtering until the backend adds full-text search.
    return q;
  }, [sortBy, status, category, page, pageSize]);

  return {
    // State
    search,
    debouncedSearch,
    sortBy,
    status,
    category,
    page,
    pageSize,
    // Actions
    setSearch,
    setSortBy,
    setStatus,
    setCategory,
    setPage,
    setPageSize,
    reset,
    poolsQuery,
  };
}
