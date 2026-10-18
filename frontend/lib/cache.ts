"use client";

/**
 * Central SWR cache utilities (#1973).
 *
 * Consolidates cache invalidation in one place so components never reach for
 * ad-hoc `mutate(...)` calls or duplicated `refetch()` state. Every cache key
 * in the app is a URL built from {@link API_BASE_URL}, so keys are grouped by
 * API path prefix and invalidated as a group.
 *
 * Usage:
 * ```ts
 * await invalidateCache("notifications");          // one group
 * await invalidateCache(["pools", "user"]);        // several groups
 * await invalidateCache();                         // everything
 * ```
 */
import { mutate } from "swr";
import { API_BASE_URL } from "@/lib/api/pools";

/** Logical cache groups — mirror the backend route structure. */
export type CacheGroup = "pools" | "notifications" | "user" | "tags";

/** Path prefix for each group, relative to {@link API_BASE_URL}. */
const GROUP_PATHS: Record<CacheGroup, string> = {
  pools: "/api/v1/pools",
  notifications: "/api/v1/notifications",
  user: "/api/v1/users/",
  tags: "/api/v1/tags",
};

const BASE = API_BASE_URL.replace(/\/$/, "");

/** True when `key` belongs to `group`. Non-string keys (objects) never match. */
export function isInGroup(key: unknown, group: CacheGroup): boolean {
  if (typeof key !== "string") return false;
  const path = GROUP_PATHS[group];
  return key.startsWith(`${BASE}${path}`) || key.startsWith(path);
}

/**
 * Revalidate every cached entry in the given group(s).
 *
 * Resolves once all matching entries have been refetched, so callers can
 * sequence UI state (e.g. "saved" toasts) after the cache is fresh.
 */
export async function invalidateCache(
  groups: CacheGroup | CacheGroup[] = Object.keys(GROUP_PATHS) as CacheGroup[],
): Promise<void> {
  const list = Array.isArray(groups) ? groups : [groups];
  await mutate(
    (key) => list.some((group) => isInGroup(key, group)),
    undefined,
    { revalidate: true },
  );
}

/** Revalidate every SWR entry regardless of group. Use sparingly. */
export async function invalidateAll(): Promise<void> {
  await mutate(() => true, undefined, { revalidate: true });
}

/**
 * Optimistically write `data` into every entry of a group, then revalidate.
 * Lets a mutation show its result immediately while the server state settles.
 */
export async function setCacheForGroup<T>(
  group: CacheGroup,
  updater: (current: T | undefined) => T | undefined,
): Promise<void> {
  await mutate(
    (key) => isInGroup(key, group),
    (current?: T) => updater(current as T | undefined),
    { revalidate: false },
  );
}
