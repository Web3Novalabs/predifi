"use client";

import { SWRConfig } from "swr";
import type { ReactNode } from "react";

/**
 * SWRProvider — app-wide SWR configuration.
 *
 * Defaults are tuned for the largely-static data PrediFi displays (notably
 * prediction-pool metadata): responses are cached and deduplicated, and we
 * avoid aggressive revalidation that would refetch data the user already has.
 * Individual hooks can still override any of these per call.
 */
export function SWRProvider({ children }: { children: ReactNode }) {
  return (
    <SWRConfig
      value={{
        // Treat fetched data as fresh for 60s: identical keys requested within
        // this window reuse the in-flight/last response instead of refetching.
        dedupingInterval: 60_000,
        // Static data doesn't change while the user tabs away and back.
        revalidateOnFocus: false,
        // Avoid a refetch storm when connectivity flaps.
        revalidateOnReconnect: false,
        // Keep showing the previous page's data while a new query loads.
        keepPreviousData: true,
        // Retry transient network/server errors a couple of times.
        errorRetryCount: 2,
      }}
    >
      {children}
    </SWRConfig>
  );
}
