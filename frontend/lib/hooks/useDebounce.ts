import { useState, useEffect } from "react";

/**
 * useDebounce
 *
 * Returns a debounced version of the provided value that only updates
 * after the specified delay has elapsed without the value changing.
 *
 * This is useful for deferring expensive operations (e.g. API calls)
 * until the user has stopped typing.
 *
 * @param value  - The value to debounce.
 * @param delay  - Debounce delay in milliseconds (default: 300ms).
 * @returns        The debounced value.
 *
 * @example
 * const debouncedQuery = useDebounce(searchQuery, 400);
 *
 * useEffect(() => {
 *   if (debouncedQuery) fetchResults(debouncedQuery);
 * }, [debouncedQuery]);
 */
export function useDebounce<T>(value: T, delay: number = 300): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    // Clear the previous timer whenever value or delay changes,
    // so only the last change within the delay window takes effect.
    return () => clearTimeout(timer);
  }, [value, delay]);

  return debouncedValue;
}
