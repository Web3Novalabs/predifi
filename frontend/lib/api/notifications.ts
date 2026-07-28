/**
 * Notifications API client.
 *
 * Wraps the `/api/v1/notifications/:address` and `/api/v1/users/:address/interests`
 * endpoints — in-app alerts for pools ending soon, resolutions, expiring claim
 * windows, and new pools matching a user's followed categories/tags.
 */

import { API_BASE_URL, ApiError } from "@/lib/api/pools";

export type NotificationType =
  | "pool_ending_soon"
  | "pool_resolved"
  | "claim_expiring"
  | "new_pool_match";

export interface Notification {
  id: number;
  user_address: string;
  notif_type: NotificationType;
  title: string;
  message: string;
  pool_id: number | null;
  read: boolean;
  created_at: string;
}

export interface NotificationsResponse {
  address: string;
  notifications: Notification[];
  unread_count: number;
  limit: number;
  offset: number;
}

interface ApiEnvelope<T> {
  data?: T;
  [key: string]: unknown;
}

export function notificationsUrl(address: string, unreadOnly = false): string {
  const params = new URLSearchParams();
  if (unreadOnly) params.set("unread_only", "true");
  const qs = params.toString();
  return `${API_BASE_URL}/api/v1/notifications/${encodeURIComponent(address)}${qs ? `?${qs}` : ""}`;
}

export async function fetchNotifications(url: string): Promise<NotificationsResponse> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });

  if (!res.ok) {
    throw new ApiError(`Failed to load notifications (HTTP ${res.status})`, res.status);
  }

  const body = (await res.json()) as ApiEnvelope<NotificationsResponse>;
  return (
    body.data ?? { address: "", notifications: [], unread_count: 0, limit: 0, offset: 0 }
  );
}

export async function markNotificationsRead(
  address: string,
  ids?: number[],
): Promise<void> {
  const res = await fetch(
    `${API_BASE_URL}/api/v1/notifications/${encodeURIComponent(address)}/read`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ids: ids ?? [] }),
    },
  );

  if (!res.ok) {
    throw new ApiError(`Failed to mark notifications read (HTTP ${res.status})`, res.status);
  }
}

export function interestsUrl(address: string): string {
  return `${API_BASE_URL}/api/v1/users/${encodeURIComponent(address)}/interests`;
}

export async function fetchInterests(url: string): Promise<string[]> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });

  if (!res.ok) {
    throw new ApiError(`Failed to load interests (HTTP ${res.status})`, res.status);
  }

  const body = (await res.json()) as ApiEnvelope<{ address: string; interests: string[] }>;
  return body.data?.interests ?? [];
}

export async function setInterests(address: string, interests: string[]): Promise<void> {
  const res = await fetch(interestsUrl(address), {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ interests }),
  });

  if (!res.ok) {
    throw new ApiError(`Failed to save interests (HTTP ${res.status})`, res.status);
  }
}
