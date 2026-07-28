"use client";

import useSWR from "swr";
import {
  fetchPools,
  poolsUrl,
  type Pool,
  type PoolsQuery,
  type PoolsResponse,
} from "@/lib/api/pools";

/** Return value of {@link usePools}. */
export interface UsePoolsResult {
  /** The fetched pools (empty while loading or on error). */
  pools: Pool[];
  /** Total number of pools matching the query (for pagination). */
  total: number;
  /** Current page limit returned by the backend. */
  limit: number;
  /** Current page offset returned by the backend. */
  offset: number;
  /** The raw response, or `undefined` until the first load resolves. */
  data: PoolsResponse | undefined;
  /** True during the initial load when no cached data is available yet. */
  isLoading: boolean;
  /** True when the most recent request failed. */
  isError: boolean;
  /** The error from the most recent failed request, if any. */
  error: Error | undefined;
  /** Manually revalidate (e.g. a "retry" button). */
  refresh: () => void;
  /**
   * The SWR cache key — the full URL used for the request.
   * Expose it so callers can use it with `useSWRMutation` or `mutate()`.
   */
  cacheKey: string;
}

/**
 * usePools — SWR-cached access to the prediction pool list.
 *
 * Prediction-pool metadata is effectively static between updates, so this hook
 * leans on the cache: identical queries share a single cache entry, concurrent
 * requests are deduplicated, cached data is served instantly on remount, and
 * (via the global {@link SWRProvider} config) the data is not revalidated on
 * window focus or reconnect.
 *
 * Pass a {@link PoolsQuery} — typically obtained from {@link usePoolsQuery} —
 * to fetch a filtered/sorted page. The URL produced by {@link poolsUrl} doubles
 * as the SWR cache key, so two calls with equivalent filter objects share the
 * same cache entry automatically.
 *
 * @example
 * // With usePoolsQuery for URL-synced state:
 * const { poolsQuery, setSearch, setPage } = usePoolsQuery();
 * const { pools, total, isLoading } = usePools(poolsQuery);
 *
 * @example
 * // Standalone usage with inline options:
 * const { pools, isLoading, isError, refresh } = usePools({ status: "active", sort_by: "new" });
 */
export function usePools(query: PoolsQuery = {}): UsePoolsResult {
  const key = poolsUrl(query);

  const { data, error, isLoading, mutate } = useSWR<PoolsResponse>(
    key,
    fetchPools,
  );

  return {
    pools: data?.pools ?? [],
    total: data?.total ?? 0,
    limit: data?.limit ?? 0,
    offset: data?.offset ?? 0,
    data,
    isLoading,
    isError: Boolean(error),
    error: error as Error | undefined,
    refresh: () => {
      void mutate();
    },
    cacheKey: key,
  };
}
