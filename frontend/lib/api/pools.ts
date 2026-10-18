/**
 * Prediction pool API client.
 *
 * Thin, typed wrapper around the PrediFi backend's `GET /api/v1/pools`
 * endpoint. The exported {@link fetchPools} function is used as the SWR
 * fetcher (see `lib/hooks/usePools.ts`) so that pool data is cached and
 * deduplicated across the app.
 *
 * HTTP response types come from the generated backend OpenAPI spec.
 */

import type { components } from "@/types/api";

/**
 * Base URL of the PrediFi backend API.
 *
 * Configurable per environment via `NEXT_PUBLIC_API_BASE_URL`; falls back to
 * the local backend default so the dashboard works out of the box in dev.
 */
export const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080";

/** WebSocket base derived from the HTTP API base. */
export function wsBaseUrl(): string {
  const http = API_BASE_URL.replace(/\/$/, "");
  if (http.startsWith("https://")) return http.replace(/^https/, "wss");
  if (http.startsWith("http://")) return http.replace(/^http/, "ws");
  return `ws://${http}`;
}

type ApiPool = components["schemas"]["PoolListingDoc"];
type ApiPoolDetail = components["schemas"]["PoolWithOddsDoc"];

/** A single prediction-market pool, with backend timestamps normalized to Unix seconds. */
export type Pool = Omit<ApiPool, "end_time"> & { end_time: number };

/** Per-outcome stake and implied odds from `GET /api/v1/pools/:id`. */
export type OutcomeOdds = components["schemas"]["OutcomeOddsDoc"];

/** Detailed pool payload including live odds. */
export type PoolDetail = Omit<ApiPoolDetail, "end_time"> & {
  end_time: number;
  outcome_descriptions?: string[];
  /** Prediction count derived client-side from live WS updates when available. */
  prediction_count?: number;
};

/** Response body of `GET /api/v1/pools`. */
export type PoolsResponse = Omit<
  components["schemas"]["PoolListResponse"],
  "pools"
> & { pools: Pool[] };

/** Filters accepted by `GET /api/v1/pools`. */
export interface PoolsQuery {
  /** Sort order. Defaults to `"new"` on the backend. */
  sort_by?: "popular" | "ending_soon" | "new";
  /** Category filter, e.g. `"Sports"` or `"Crypto"`. */
  category?: string;
  /** Tag filter — a pool matches if any of its tags overlap this list. */
  tags?: string[];
  /** Lifecycle filter. Defaults to `"active"` on the backend. */
  status?: "active" | "closed" | "settled";
  limit?: number;
  offset?: number;
}

/**
 * Build the request URL for a pools query.
 *
 * The returned string doubles as the SWR cache key: two calls with equal
 * filters produce an identical URL and therefore share one cache entry.
 */
export function poolsUrl(query: PoolsQuery = {}): string {
  const params = new URLSearchParams();
  if (query.sort_by) params.set("sort_by", query.sort_by);
  if (query.category) params.set("category", query.category);
  if (query.tags && query.tags.length > 0) params.set("tags", query.tags.join(","));
  if (query.status) params.set("status", query.status);
  if (query.limit != null) params.set("limit", String(query.limit));
  if (query.offset != null) params.set("offset", String(query.offset));

  const qs = params.toString();
  return `${API_BASE_URL}/api/v1/pools${qs ? `?${qs}` : ""}`;
}

/** Cache key / URL for a single pool detail fetch. */
export function poolDetailUrl(poolId: number | string): string {
  return `${API_BASE_URL}/api/v1/pools/${poolId}`;
}

/** Error raised when the pools endpoint responds with a non-2xx status. */
export class ApiError extends Error {
  readonly status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

/**
 * SWR fetcher for pool data.
 *
 * @param url - A URL produced by {@link poolsUrl}.
 * @throws {ApiError} When the response status is not 2xx.
 */
export async function fetchPools(url: string): Promise<PoolsResponse> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });

  if (!res.ok) {
    throw new ApiError(`Failed to load pools (HTTP ${res.status})`, res.status);
  }

  const response = (await res.json()) as components["schemas"]["PoolListResponse"];
  return {
    ...response,
    pools: response.pools.map((pool) => ({
      ...pool,
      end_time: Math.floor(new Date(pool.end_time).getTime() / 1000),
    })),
  };
}

/**
 * Fetch a single pool with live odds.
 *
 * Unwraps the typed API envelope and normalizes the backend timestamp.
 */
export async function fetchPoolDetail(url: string): Promise<PoolDetail> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });

  if (!res.ok) {
    throw new ApiError(`Failed to load pool (HTTP ${res.status})`, res.status);
  }

  const body =
    (await res.json()) as components["schemas"]["ApiPoolDetailsResponse"];
  const pool = body.data;

  return {
    ...pool,
    end_time: Math.floor(new Date(pool.end_time).getTime() / 1000),
    odds: pool.odds ?? [],
  };
}

/** Helper function to fetch a pool detail by ID directly. */
export async function fetchPoolById(id: string | number): Promise<PoolDetail> {
  return fetchPoolDetail(poolDetailUrl(id));
}

/** Live event pushed over `/api/v1/ws`. */
export interface PoolLiveEvent {
  type: string;
  pool_id: number;
  user_address?: string;
  outcome?: number;
  amount?: number;
}
