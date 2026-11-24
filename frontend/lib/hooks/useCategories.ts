"use client";

import { useMemo } from "react";
import useSWR from "swr";
import {
  DEFAULT_CATEGORIES,
  categoriesUrl,
  fetchCategories,
  type CategoryCount,
} from "@/lib/api/categories";

/**
 * useCategories — pool categories with active-pool counts, for the category
 * filter. Falls back to the default categories (count unknown) while loading
 * or when the request fails, so the filter is never empty.
 */
export function useCategories(): { categories: CategoryCount[]; isLoading: boolean } {
  const { data, isLoading } = useSWR<CategoryCount[]>(categoriesUrl(), fetchCategories);
  const categories = useMemo<CategoryCount[]>(
    () => data ?? DEFAULT_CATEGORIES.map((category) => ({ category, count: 0 })),
    [data],
  );
  return { categories, isLoading };
}
