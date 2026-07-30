/**
 * User profile API client.
 *
 * Thin, typed wrapper around `GET /api/v1/users/:address/profile` — the
 * aggregated payload behind the profile page (prediction history, win/loss
 * stats, total earnings, active positions, and claim status).
 */

import { API_BASE_URL, ApiError } from "@/lib/api/pools";

export interface ProfileStats {
  total_predictions: number;
  wins: number;
  losses: number;
  pending: number;
  /** Win rate as a percentage (0-100) of settled predictions. */
  win_rate: number;
  total_staked: number;
  total_earnings: number;
  active_positions: number;
}

export interface ClaimStatus {
  prediction_id: number;
  pool_id: number;
  pool_name: string;
  outcome: number;
  amount: number;
  pool_state: string;
  pool_result: string | null;
  is_winner: boolean | null;
  claimed: boolean;
  claimed_amount: number;
  claim_window_expires_at: string | null;
  claim_expired: boolean;
}

export interface PerformancePoint {
  day: string;
  staked: number;
  earnings: number;
  predictions: number;
}

export interface UserProfile {
  address: string;
  stats: ProfileStats;
  claims: ClaimStatus[];
  performance: PerformancePoint[];
}

/** Response envelope shared by every PrediFi backend endpoint (`ApiResponse`). */
interface ApiEnvelope<T> {
  data?: T;
  success?: boolean;
  [key: string]: unknown;
}

export function profileUrl(address: string): string {
  return `${API_BASE_URL}/api/v1/users/${encodeURIComponent(address)}/profile`;
}

export async function fetchProfile(url: string): Promise<UserProfile> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });

  if (!res.ok) {
    throw new ApiError(`Failed to load profile (HTTP ${res.status})`, res.status);
  }

  const body = (await res.json()) as ApiEnvelope<UserProfile> | UserProfile;
  return "data" in body && body.data ? body.data : (body as UserProfile);
}
