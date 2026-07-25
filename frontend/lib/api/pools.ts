/**
 * Prediction pool API client.
 *
 * Thin, typed wrapper around the PrediFi backend's `GET /api/v1/pools`
 * endpoint. The exported {@link fetchPools} function is used as the SWR
 * fetcher (see `lib/hooks/usePools.ts`) so that pool data is cached and
 * deduplicated across the app.
 *
 * The shapes here mirror the backend OpenAPI schema (`PoolDoc` /
 * `PoolsResponse` in `backend/src/openapi.rs`).
 */

/**
 * Base URL of the PrediFi backend API.
 *
 * Configurable per environment via `NEXT_PUBLIC_API_BASE_URL`; falls back to
 * the local backend default so the dashboard works out of the box in dev.
 */
export const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080";

/** A single prediction-market pool. */
export interface Pool {
  pool_id: number;
  name: string;
  category: string;
  /** Total amount staked across the pool, in the token's base units. */
  total_stake: number;
  /** Pool close time as a Unix timestamp (seconds). */
  end_time: number;
  created_at: string;
  state: string;
  creator: string;
  token: string;
  /** Settled outcome, or `null` while the pool is still open. */
  result: string | null;
}

/** Response body of `GET /api/v1/pools`. */
export interface PoolsResponse {
  pools: Pool[];
  total: number;
  limit: number;
  offset: number;
  status: string;
  category?: string | null;
  sort_by: string;
}

/** Filters accepted by `GET /api/v1/pools`. */
export interface PoolsQuery {
  /** Sort order. Defaults to `"new"` on the backend. */
  sort_by?: "popular" | "ending_soon" | "new";
  /** Category filter, e.g. `"Sports"` or `"Crypto"`. */
  category?: string;
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
  if (query.status) params.set("status", query.status);
  if (query.limit != null) params.set("limit", String(query.limit));
  if (query.offset != null) params.set("offset", String(query.offset));

  const qs = params.toString();
  return `${API_BASE_URL}/api/v1/pools${qs ? `?${qs}` : ""}`;
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

  return (await res.json()) as PoolsResponse;
}
