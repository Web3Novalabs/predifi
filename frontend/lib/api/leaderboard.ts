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

interface ApiEnvelope<T> {
  status: "success" | "error";
  data?: T;
  error?: { code: string; message: string; request_id: string };
}

export interface LeaderboardQuery {
  rankBy?: LeaderboardRankBy;
  period?: LeaderboardPeriod;
  limit?: number;
  offset?: number;
}

async function unwrap<T>(res: Response): Promise<T> {
  if (!res.ok) {
    throw new Error(`Leaderboard request failed (HTTP ${res.status})`);
  }
  const body = (await res.json()) as ApiEnvelope<T> & Partial<T>;
  // `ApiResponse::success` wraps in { status, data }; some legacy handlers
  // (get_leaderboard for "winnings"/"volume") return the same envelope.
  return (body.data ?? (body as unknown as T)) as T;
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
  return unwrap(res);
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
  return unwrap(res);
}
