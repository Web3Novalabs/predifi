"use client";

import useSWR from "swr";
import { fetchTags, tagsUrl } from "@/lib/api/tags";

/** useTags — distinct pool tags in use, for filter-UI dropdowns. */
export function useTags(): { tags: string[]; isLoading: boolean } {
  const { data, isLoading } = useSWR<string[]>(tagsUrl(), fetchTags);
  return { tags: data ?? [], isLoading };
}
