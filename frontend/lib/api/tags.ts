/**
 * Pool tags API client.
 *
 * Wraps `GET /api/v1/tags` (distinct tags in use, for filter dropdowns) and
 * `PATCH /api/v1/pools/:id/tags` (creator-only tag updates).
 */

import { API_BASE_URL, ApiError } from "@/lib/api/pools";

interface ApiEnvelope<T> {
  data?: T;
  [key: string]: unknown;
}

export function tagsUrl(): string {
  return `${API_BASE_URL}/api/v1/tags`;
}

export async function fetchTags(url: string): Promise<string[]> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });

  if (!res.ok) {
    throw new ApiError(`Failed to load tags (HTTP ${res.status})`, res.status);
  }

  const body = (await res.json()) as ApiEnvelope<{ tags: string[] }>;
  return body.data?.tags ?? [];
}

export async function updatePoolTags(
  poolId: number,
  creator: string,
  tags: string[],
): Promise<void> {
  const res = await fetch(`${API_BASE_URL}/api/v1/pools/${poolId}/tags`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ creator, tags }),
  });

  if (!res.ok) {
    throw new ApiError(`Failed to update tags (HTTP ${res.status})`, res.status);
  }
}
