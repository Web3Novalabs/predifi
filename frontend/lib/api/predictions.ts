/**
 * User predictions API client.
 *
 * Wraps `GET /api/v1/users/:address/predictions`, which returns each of a
 * wallet's predictions joined with the current state of its pool.
 */

import { API_BASE_URL, ApiError, fetchWithTimeout } from "@/lib/api/pools";

/** A single prediction with its pool's current status. */
export interface UserPrediction {
  prediction_id: number;
  pool_id: number;
  pool_name: string;
  pool_category: string;
  /** Pool lifecycle state, e.g. "active" or "settled". */
  pool_state: string;
  /** ISO-8601 timestamp. */
  pool_end_time: string;
  /** Total stake in the pool (stroops). */
  pool_total_stake: number;
  pool_result: string | null;
  user_outcome: number;
  /** The user's stake (stroops). */
  user_amount: number;
  /** ISO-8601 timestamp. */
  prediction_created_at: string;
  is_winning_outcome: boolean | null;
}

/** Response body of `GET /api/v1/users/:address/predictions`. */
export interface UserPredictionsResponse {
  address: string;
  predictions: UserPrediction[];
  limit: number;
  offset: number;
  total_predictions: number;
}

/** SWR cache key / request URL for a wallet's predictions. */
export function userPredictionsUrl(
  address: string,
  query: { limit?: number; offset?: number } = {},
): string {
  const params = new URLSearchParams();
  if (query.limit != null) params.set("limit", String(query.limit));
  if (query.offset != null) params.set("offset", String(query.offset));
  const qs = params.toString();
  return `${API_BASE_URL}/api/v1/users/${encodeURIComponent(address)}/predictions${qs ? `?${qs}` : ""}`;
}

function isUserPrediction(obj: unknown): obj is UserPrediction {
  if (!obj || typeof obj !== "object") return false;
  const p = obj as Record<string, unknown>;
  return (
    typeof p.prediction_id === "number" &&
    typeof p.pool_id === "number" &&
    typeof p.pool_name === "string" &&
    typeof p.pool_state === "string" &&
    typeof p.pool_end_time === "string" &&
    typeof p.user_amount === "number"
  );
}

/**
 * SWR fetcher for a wallet's predictions.
 *
 * @param url - A URL produced by {@link userPredictionsUrl}.
 * @throws {ApiError} On a non-2xx status or an unexpected response shape.
 */
export async function fetchUserPredictions(
  url: string,
): Promise<UserPredictionsResponse> {
  const res = await fetchWithTimeout(url, {
    headers: { Accept: "application/json" },
  });

  if (!res.ok) {
    throw new ApiError(`Failed to load predictions (HTTP ${res.status})`, res.status);
  }

  const body: unknown = await res.json();
  const data =
    body && typeof body === "object" && "data" in body && body.data ? body.data : body;
  const r = data as Record<string, unknown> | null;
  if (
    !r ||
    typeof r !== "object" ||
    !Array.isArray(r.predictions) ||
    !r.predictions.every(isUserPrediction)
  ) {
    throw new ApiError("Invalid predictions response shape", 500);
  }
  return data as UserPredictionsResponse;
}

/**
 * Whether a prediction belongs in the "active" tab: its pool is still open and
 * its end time has not passed. Everything else is "past".
 */
export function isActivePrediction(
  prediction: UserPrediction,
  now: number = Date.now(),
): boolean {
  if (prediction.pool_state !== "active") return false;
  const end = new Date(prediction.pool_end_time).getTime();
  return Number.isNaN(end) || end > now;
}
