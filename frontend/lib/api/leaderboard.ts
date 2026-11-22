/**
 * Leaderboard API client.
 *
 * Thin wrapper around `GET /api/v1/leaderboard` and
 * `GET /api/v1/pools/{id}/leaderboard`.
 */
import { API_BASE_URL } from "@/lib/api/pools";

export type LeaderboardRankBy = "volume" | "winnings" | "win_rate" | "streak";
export type LeaderboardPeriod = "week" | "month" | "all";

/** A single leaderboard row (volume / win_rate / streak ranking). */
export interface LeaderboardEntry {
  user_address: string;
  total_volume: number;
  prediction_count: number;
  wins: number;
  settled_count: number;
  win_rate: number;
  current_streak: number;
  rank: number;
}

/** A single leaderboard row for the legacy dollar-winnings ranking. */
export interface WinningsLeaderboardEntry {
  user_address: string;
  total_winnings: number;
  winning_predictions: number;
  total_predictions: number;
  win_rate: number;
  rank: number;
}

interface LeaderboardResponse<T> {
  leaderboard: T[];
  rank_by: string;
  period?: string;
  limit: number;
  offset: number;
}

/** Type guard to validate LeaderboardEntry shape. */
function isLeaderboardEntry(obj: unknown): obj is LeaderboardEntry {
  if (!obj || typeof obj !== "object") return false;
  const entry = obj as Record<string, unknown>;
  return (
    typeof entry.user_address === "string" &&
    typeof entry.total_volume === "number" &&
    typeof entry.prediction_count === "number" &&
    typeof entry.wins === "number" &&
    typeof entry.settled_count === "number" &&
    typeof entry.win_rate === "number" &&
    typeof entry.current_streak === "number" &&
    typeof entry.rank === "number"
  );
}

/** Type guard to validate WinningsLeaderboardEntry shape. */
function isWinningsLeaderboardEntry(obj: unknown): obj is WinningsLeaderboardEntry {
  if (!obj || typeof obj !== "object") return false;
  const entry = obj as Record<string, unknown>;
  return (
    typeof entry.user_address === "string" &&
    typeof entry.total_winnings === "number" &&
    typeof entry.winning_predictions === "number" &&
    typeof entry.total_predictions === "number" &&
    typeof entry.win_rate === "number" &&
    typeof entry.rank === "number"
  );
}

/** Type guard to validate generic LeaderboardResponse shape. */
function isLeaderboardResponse<T>(
  obj: unknown,
  itemValidator: (item: unknown) => item is T,
): obj is LeaderboardResponse<T> {
  if (!obj || typeof obj !== "object") return false;
  const response = obj as Record<string, unknown>;
  return (
    Array.isArray(response.leaderboard) &&
    response.leaderboard.every(itemValidator) &&
    typeof response.rank_by === "string" &&
    typeof response.limit === "number" &&
    typeof response.offset === "number"
  );
}

export interface LeaderboardQuery {
  rankBy?: LeaderboardRankBy;
  period?: LeaderboardPeriod;
  limit?: number;
  offset?: number;
}

async function unwrap<T>(
  res: Response,
  validator: (obj: unknown) => obj is T,
): Promise<T> {
  if (!res.ok) {
    throw new Error(`Leaderboard request failed (HTTP ${res.status})`);
  }
  const body = await res.json();

  // Handle wrapped response (ApiEnvelope pattern)
  const data =
    body && typeof body === "object" && "data" in body && body.data ? body.data : body;

  // Validate response shape at boundary
  if (!validator(data)) {
    throw new Error(`Invalid leaderboard response shape (HTTP ${res.status})`);
  }

  return data as T;
}

/** Type validator for global leaderboard responses. */
function isGlobalLeaderboardResponse(
  obj: unknown,
): obj is LeaderboardResponse<LeaderboardEntry | WinningsLeaderboardEntry> {
  return (
    isLeaderboardResponse(obj, (item: unknown) => {
      return isLeaderboardEntry(item) || isWinningsLeaderboardEntry(item);
    })
  );
}

/** Type validator for pool leaderboard responses. */
function isPoolLeaderboardResponse(obj: unknown): obj is LeaderboardResponse<LeaderboardEntry> {
  return isLeaderboardResponse(obj, isLeaderboardEntry);
}

/** Fetch the global leaderboard. */
export async function fetchLeaderboard(
  query: LeaderboardQuery = {},
): Promise<LeaderboardResponse<LeaderboardEntry | WinningsLeaderboardEntry>> {
  const params = new URLSearchParams();
  params.set("rank_by", query.rankBy ?? "volume");
  if (query.period) params.set("period", query.period);
  if (query.limit != null) params.set("limit", String(query.limit));
  if (query.offset != null) params.set("offset", String(query.offset));

  const res = await fetch(`${API_BASE_URL}/api/v1/leaderboard?${params}`, {
    headers: { Accept: "application/json" },
  });
  return unwrap(res, isGlobalLeaderboardResponse);
}

/** Fetch the leaderboard scoped to a single pool. */
export async function fetchPoolLeaderboard(
  poolId: number | string,
  query: Omit<LeaderboardQuery, "period"> = {},
): Promise<LeaderboardResponse<LeaderboardEntry>> {
  const params = new URLSearchParams();
  params.set("rank_by", query.rankBy ?? "volume");
  if (query.limit != null) params.set("limit", String(query.limit));
  if (query.offset != null) params.set("offset", String(query.offset));

  const res = await fetch(
    `${API_BASE_URL}/api/v1/pools/${poolId}/leaderboard?${params}`,
    { headers: { Accept: "application/json" } },
  );
  return unwrap(res, isPoolLeaderboardResponse);
}
