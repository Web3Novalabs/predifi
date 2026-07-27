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

/** WebSocket base derived from the HTTP API base. */
export function wsBaseUrl(): string {
  const http = API_BASE_URL.replace(/\/$/, "");
  if (http.startsWith("https://")) return http.replace(/^https/, "wss");
  if (http.startsWith("http://")) return http.replace(/^http/, "ws");
  return `ws://${http}`;
}

/** A single prediction-market pool. */
export interface Pool {
  pool_id: number;
  name: string;
  category: string;
  /** Free-form tags assigned by the creator, e.g. `["btc", "price-prediction"]`. */
  tags: string[];
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

/** Per-outcome stake and implied odds from `GET /api/v1/pools/:id`. */
export interface OutcomeOdds {
  outcome: number;
  stake: number;
  odds: number;
}

/** Detailed pool payload including live odds. */
export interface PoolDetail extends Pool {
  odds: OutcomeOdds[];
  outcome_descriptions?: string[];
  /** Prediction count derived client-side from live WS updates when available. */
  prediction_count?: number;
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

  return (await res.json()) as PoolsResponse;
}

/**
 * Fetch a single pool with live odds.
 *
 * Handles both raw `PoolWithOdds` JSON and wrapped `{ data: ... }` API responses.
 */
export async function fetchPoolDetail(url: string): Promise<PoolDetail> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });

  if (!res.ok) {
    throw new ApiError(`Failed to load pool (HTTP ${res.status})`, res.status);
  }

  const body = (await res.json()) as
    | (PoolDetail & { end_time?: number | string })
    | { data?: PoolDetail & { end_time?: number | string }; error?: string };

  if (body && typeof body === "object" && "error" in body && body.error) {
    throw new ApiError(String(body.error), 404);
  }

  const pool =
    body && typeof body === "object" && "data" in body && body.data
      ? body.data
      : (body as PoolDetail & { end_time?: number | string });

  const rawEnd = pool.end_time;
  const endTime =
    typeof rawEnd === "string"
      ? Math.floor(new Date(rawEnd).getTime() / 1000)
      : Number(rawEnd ?? 0);

  return {
    ...pool,
    end_time: endTime,
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
