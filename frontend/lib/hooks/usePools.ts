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
}

/**
 * usePools — SWR-cached access to the prediction pool list.
 *
 * Prediction-pool metadata is effectively static between updates, so this hook
 * leans on the cache: identical queries share a single cache entry, concurrent
 * requests are deduplicated, cached data is served instantly on remount, and
 * (via the global {@link SWRProvider} config) the data is not revalidated on
 * window focus or reconnect. Pass a {@link PoolsQuery} to fetch a
 * filtered/sorted page.
 *
 * @example
 * const { pools, isLoading, isError, refresh } = usePools({ status: "active" });
 */
export function usePools(query: PoolsQuery = {}): UsePoolsResult {
  const { data, error, isLoading, mutate } = useSWR<PoolsResponse>(
    poolsUrl(query),
    fetchPools
  );

  return {
    pools: data?.pools ?? [],
    total: data?.total ?? 0,
    data,
    isLoading,
    isError: Boolean(error),
    error: error as Error | undefined,
    refresh: () => {
      void mutate();
    },
  };
}
