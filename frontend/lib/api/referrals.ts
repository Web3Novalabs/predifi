/**
 * Referral earnings API client.
 *
 * Thin, typed wrapper around `GET /api/v1/users/:address/referrals` — the
 * per-pool referral earnings for a user. The backend responds with `404` when
 * the address has no referral records, which callers should treat as an empty
 * list rather than an error.
 */

import { API_BASE_URL, ApiError } from "@/lib/api/pools";

/** A single pool's aggregated referral earnings for the queried referrer. */
export interface ReferralEarningRow {
  pool_id: number;
  pool_name: string;
  total_earned: number;
  referral_count: number;
}

/** Response body of `GET /api/v1/users/:address/referrals`. */
export interface ReferralEarnings {
  referrer: string;
  total_earned: number;
  pools: ReferralEarningRow[];
}

function isReferralEarningRow(obj: unknown): obj is ReferralEarningRow {
  if (!obj || typeof obj !== "object") return false;
  const row = obj as Record<string, unknown>;
  return (
    typeof row.pool_id === "number" &&
    typeof row.pool_name === "string" &&
    typeof row.total_earned === "number" &&
    typeof row.referral_count === "number"
  );
}

function isReferralEarnings(obj: unknown): obj is ReferralEarnings {
  if (!obj || typeof obj !== "object") return false;
  const earnings = obj as Record<string, unknown>;
  return (
    typeof earnings.referrer === "string" &&
    typeof earnings.total_earned === "number" &&
    Array.isArray(earnings.pools) &&
    earnings.pools.every(isReferralEarningRow)
  );
}

export function referralsUrl(address: string): string {
  return `${API_BASE_URL}/api/v1/users/${encodeURIComponent(address)}/referrals`;
}

export async function fetchReferralEarnings(
  url: string,
): Promise<ReferralEarnings> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });

  // No referral records yet — present as an empty history, not an error.
  if (res.status === 404) {
    return { referrer: "", total_earned: 0, pools: [] };
  }

  if (!res.ok) {
    throw new ApiError(
      `Failed to load referrals (HTTP ${res.status})`,
      res.status,
    );
  }

  const body = await res.json();

  // Handle both wrapped (`{ status, data }`) and unwrapped responses.
  const data =
    body && typeof body === "object" && "data" in body && body.data
      ? body.data
      : body;

  if (!isReferralEarnings(data)) {
    throw new ApiError("Invalid referral earnings response shape", 500);
  }

  return data;
}
