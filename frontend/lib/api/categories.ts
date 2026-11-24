/**
 * Pool categories API client.
 *
 * Wraps `GET /api/v1/categories` (categories with active-pool counts, for the
 * category filter) and `PATCH /api/v1/pools/:id/metadata` (creator/admin
 * category + tag updates).
 */

import { API_BASE_URL, ApiError } from "@/lib/api/pools";

/** A category and the number of active pools in it. */
export interface CategoryCount {
  category: string;
  count: number;
}

/** Baseline categories shown until (or if) the API responds. */
export const DEFAULT_CATEGORIES: readonly string[] = [
  "crypto",
  "sports",
  "politics",
  "finance",
  "entertainment",
  "technology",
  "science",
  "gaming",
  "other",
];

export function categoriesUrl(): string {
  return `${API_BASE_URL}/api/v1/categories`;
}

/** Display form of a stored (lower-case) category, e.g. `"pop culture"` → `"Pop Culture"`. */
export function formatCategory(category: string): string {
  return category
    .split(/\s+/)
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function isCategoryCount(obj: unknown): obj is CategoryCount {
  if (!obj || typeof obj !== "object") return false;
  const c = obj as Record<string, unknown>;
  return typeof c.category === "string" && typeof c.count === "number";
}

export async function fetchCategories(url: string): Promise<CategoryCount[]> {
  const res = await fetch(url, { headers: { Accept: "application/json" } });
  if (!res.ok) {
    throw new ApiError(`Failed to load categories (HTTP ${res.status})`, res.status);
  }

  const body: unknown = await res.json();
  const data =
    body && typeof body === "object" && "data" in body && (body as { data: unknown }).data
      ? (body as { data: unknown }).data
      : body;

  const categories = (data as { categories?: unknown } | null)?.categories;
  if (!Array.isArray(categories) || !categories.every(isCategoryCount)) {
    throw new ApiError("Invalid categories response shape", 500);
  }
  return categories;
}

/**
 * Update a pool's category and/or tags. Requires an access token for the
 * pool's creator (or an admin key via `adminKey`).
 */
export async function updatePoolMetadata(
  poolId: number,
  changes: { category?: string; tags?: string[] },
  auth: { accessToken?: string; adminKey?: string },
): Promise<void> {
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (auth.accessToken) headers.Authorization = `Bearer ${auth.accessToken}`;
  if (auth.adminKey) headers["x-admin-api-key"] = auth.adminKey;

  const res = await fetch(`${API_BASE_URL}/api/v1/pools/${poolId}/metadata`, {
    method: "PATCH",
    headers,
    body: JSON.stringify(changes),
  });

  if (!res.ok) {
    throw new ApiError(`Failed to update pool (HTTP ${res.status})`, res.status);
  }
}
