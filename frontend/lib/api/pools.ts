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

type ApiPool = components["schemas"]["PoolListResponse"]["pools"][number] & {
  tags: string[];
};
type ApiPoolDetail = components["schemas"]["PoolWithOddsDoc"];
type RuntimePoolDetail = Omit<ApiPoolDetail, "end_time"> & {
  end_time: string | number;
};

/** A single prediction-market pool, with backend timestamps normalized to Unix seconds. */
export type Pool = Omit<ApiPool, "end_time"> & { end_time: number };

/** Per-outcome stake and implied odds from `GET /api/v1/pools/:id`. */
export type OutcomeOdds = components["schemas"]["OutcomeOddsDoc"];

/** Detailed pool payload including live odds. */
export type PoolDetail = Omit<ApiPoolDetail, "end_time"> & {
  end_time: number;
  /** Prediction count derived client-side from live WS updates when available. */
  prediction_count?: number;
};

/** Response body of `GET /api/v1/pools`. */
export type PoolsResponse = Omit<
  components["schemas"]["PoolListResponse"],
  "pools" | "total"
> & { pools: Pool[]; total: number; q?: string | null };

/** Filters accepted by `GET /api/v1/pools`. */
export interface PoolsQuery {
  /** Sort order. Defaults to `"new"` on the backend. */
  sort_by?: "popular" | "ending_soon" | "new";
  /** Category filter, e.g. `"Sports"` or `"Crypto"`. */
  category?: string;
  /** Tag filter — a pool matches if any of its tags overlap this list. */
  tags?: string[];
  /** Full-text search query across pool name, category, and tags. */
  q?: string;
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
  if (query.q && query.q.trim()) params.set("q", query.q.trim());
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

/** Error raised when a request exceeds the timeout. */
export class TimeoutError extends Error {
  constructor(message: string = "Request timeout") {
    super(message);
    this.name = "TimeoutError";
  }
}

/** Default timeout for API requests in milliseconds. */
const DEFAULT_REQUEST_TIMEOUT = 30000; // 30 seconds

/**
 * Execute a fetch with an optional timeout.
 *
 * @param url - The URL to fetch.
 * @param options - Fetch options and timeout configuration.
 * @param timeout - Timeout in milliseconds. Defaults to DEFAULT_REQUEST_TIMEOUT.
 * @throws {TimeoutError} When the request exceeds the timeout.
 * @throws {ApiError} When the response status is not 2xx.
 */
export async function fetchWithTimeout(
  url: string,
  options: RequestInit = {},
  timeout: number = DEFAULT_REQUEST_TIMEOUT,
): Promise<Response> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const res = await fetch(url, {
      ...options,
      signal: controller.signal,
    });
    return res;
  } catch (err) {
    if (err instanceof Error && err.name === "AbortError") {
      throw new TimeoutError(`Request timeout after ${timeout}ms`);
    }
    throw err;
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * SWR fetcher for pool data.
 *
 * @param url - A URL produced by {@link poolsUrl}.
 * @param timeout - Optional timeout in milliseconds. Defaults to DEFAULT_REQUEST_TIMEOUT.
 * @throws {ApiError} When the response status is not 2xx.
 * @throws {TimeoutError} When the request exceeds the timeout.
 */
export async function fetchPools(url: string, timeout?: number): Promise<PoolsResponse> {
  const res = await fetchWithTimeout(url, { headers: { Accept: "application/json" } }, timeout);

  if (!res.ok) {
    throw new ApiError(`Failed to load pools (HTTP ${res.status})`, res.status);
  }

  const body: unknown = await res.json();
  if (!isPoolsResponse(body)) {
    throw new ApiError("Invalid pools response shape", 500);
  }

  return {
    ...body,
    pools: body.pools.map((pool) => ({
      ...pool,
      end_time: Math.floor(new Date(pool.end_time).getTime() / 1000),
    })),
  };
}

/** Type guard to validate PoolsResponse shape. */
function isPoolsResponse(
  obj: unknown,
): obj is Omit<components["schemas"]["PoolListResponse"], "pools" | "total"> & {
  pools: ApiPool[];
  q?: string | null;
  total: number;
} {
  if (!obj || typeof obj !== "object") return false;
  const r = obj as Record<string, unknown>;
  return (
    Array.isArray(r.pools) &&
    r.pools.every(isPoolListing) &&
    typeof r.total === "number" &&
    typeof r.limit === "number" &&
    typeof r.offset === "number" &&
    typeof r.status === "string" &&
    typeof r.sort_by === "string" &&
    (r.category == null || typeof r.category === "string") &&
    (r.q == null || typeof r.q === "string")
  );
}

function isPoolListing(obj: unknown): obj is ApiPool {
  if (!obj || typeof obj !== "object") return false;
  const pool = obj as Record<string, unknown>;
  return (
    typeof pool.pool_id === "number" &&
    typeof pool.name === "string" &&
    typeof pool.category === "string" &&
    Array.isArray(pool.tags) &&
    pool.tags.every((tag) => typeof tag === "string") &&
    typeof pool.total_stake === "number" &&
    typeof pool.end_time === "string" &&
    typeof pool.created_at === "string" &&
    typeof pool.state === "string" &&
    typeof pool.creator === "string" &&
    typeof pool.token === "string" &&
    (pool.result == null || typeof pool.result === "string")
  );
}

function isOutcomeOdds(obj: unknown): obj is OutcomeOdds {
  if (!obj || typeof obj !== "object") return false;
  const odds = obj as Record<string, unknown>;
  return (
    typeof odds.outcome === "number" &&
    typeof odds.stake === "number" &&
    typeof odds.odds === "number"
  );
}

/** Type guard to validate PoolDetail shape. */
function isPoolDetail(obj: unknown): obj is RuntimePoolDetail {
  if (!obj || typeof obj !== "object") return false;
  const pool = obj as Record<string, unknown>;
  return (
    typeof pool.pool_id === "number" &&
    typeof pool.name === "string" &&
    typeof pool.category === "string" &&
    typeof pool.total_stake === "number" &&
    (typeof pool.end_time === "number" || typeof pool.end_time === "string") &&
    typeof pool.created_at === "string" &&
    typeof pool.state === "string" &&
    typeof pool.creator === "string" &&
    typeof pool.token === "string" &&
    (pool.result == null || typeof pool.result === "string") &&
    Array.isArray(pool.odds) &&
    pool.odds.every(isOutcomeOdds) &&
    Array.isArray(pool.outcome_descriptions) &&
    pool.outcome_descriptions.every((description) => typeof description === "string")
  );
}

/** Type guard to validate wrapped pool response. */
function isWrappedPoolDetail(
  obj: unknown,
): obj is { status: string; data: RuntimePoolDetail } {
  if (!obj || typeof obj !== "object") return false;
  const body = obj as Record<string, unknown>;
  return (
    typeof body.status === "string" &&
    "data" in body &&
    isPoolDetail(body.data)
  );
}

/**
 * Fetch a single pool with live odds.
 *
 * @param url - The pool detail URL.
 * @param timeout - Optional timeout in milliseconds. Defaults to DEFAULT_REQUEST_TIMEOUT.
 * @throws {ApiError} When the response status is not 2xx.
 * @throws {TimeoutError} When the request exceeds the timeout.
 */
export async function fetchPoolDetail(url: string, timeout?: number): Promise<PoolDetail> {
  const res = await fetchWithTimeout(url, { headers: { Accept: "application/json" } }, timeout);

  if (!res.ok) {
    throw new ApiError(`Failed to load pool (HTTP ${res.status})`, res.status);
  }

  const body: unknown = await res.json();
  if (!isWrappedPoolDetail(body)) {
    throw new ApiError("Invalid pool response shape", 500);
  }

  const pool = body.data;

  return {
    ...pool,
    end_time:
      typeof pool.end_time === "string"
        ? Math.floor(new Date(pool.end_time).getTime() / 1000)
        : pool.end_time,
    odds: pool.odds ?? [],
  };
}

/** Helper function to fetch a pool detail by ID directly.
 * @param id - The pool ID.
 * @param timeout - Optional timeout in milliseconds.
 */
export async function fetchPoolById(id: string | number, timeout?: number): Promise<PoolDetail> {
  return fetchPoolDetail(poolDetailUrl(id), timeout);
}

/** Live event pushed over `/api/v1/ws`. */
export interface PoolLiveEvent {
  type: string;
  pool_id: number;
  user_address?: string;
  outcome?: number;
  amount?: number;
}
