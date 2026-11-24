/**
 * Pool tags API client.
 *
 * Wraps `GET /api/v1/tags` (distinct tags in use, for filter dropdowns) and
 * `PATCH /api/v1/pools/:id/tags` (creator/admin tag updates).
 */

import { API_BASE_URL, ApiError } from "@/lib/api/pools";

/** Type guard to validate tags response shape. */
function isTagsData(obj: unknown): obj is { tags: string[] } {
  if (!obj || typeof obj !== "object") return false;
  const data = obj as Record<string, unknown>;
  return (
    Array.isArray(data.tags) &&
    data.tags.every((tag: unknown) => typeof tag === "string")
  );
}

export function tagsUrl(): string {
  return `${API_BASE_URL}/api/v1/tags`;
}

export async function fetchTags(url: string): Promise<string[]> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });

  if (!res.ok) {
    throw new ApiError(`Failed to load tags (HTTP ${res.status})`, res.status);
  }

  const body = await res.json();

  // Handle both wrapped and unwrapped responses
  const data =
    body && typeof body === "object" && "data" in body && body.data ? body.data : body;

  // Validate response shape at boundary
  if (!isTagsData(data)) {
    throw new ApiError("Invalid tags response shape", 500);
  }

  return data.tags;
}

/**
 * Replace a pool's tags. Requires the pool creator's access token (or an
 * admin key). The legacy `creator` body field is no longer sent — it was never
 * trusted for authorization.
 */
export async function updatePoolTags(
  poolId: number,
  tags: string[],
  auth: { accessToken?: string; adminKey?: string },
): Promise<void> {
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (auth.accessToken) headers.Authorization = `Bearer ${auth.accessToken}`;
  if (auth.adminKey) headers["x-admin-api-key"] = auth.adminKey;

  const res = await fetch(`${API_BASE_URL}/api/v1/pools/${poolId}/tags`, {
    method: "PATCH",
    headers,
    body: JSON.stringify({ tags }),
  });

  if (!res.ok) {
    throw new ApiError(`Failed to update tags (HTTP ${res.status})`, res.status);
  }
}
