/**
 * Shared date helpers for lightweight, deterministic formatting.
 *
 * Keeping this local avoids pulling in a heavier date library for the small
 * amount of date rendering the frontend currently needs.
 */

/**
 * Format a date-like value as `DD-MM-YYYY HH:mm` in UTC.
 *
 * The helper accepts a `Date`, Unix timestamp (milliseconds), or any string
 * the `Date` constructor can parse.
 */
export function formatUtcDateTime(value: string | number | Date): string {
  const date = value instanceof Date ? value : new Date(value);

  if (Number.isNaN(date.getTime())) {
    return "";
  }

  const day = String(date.getUTCDate()).padStart(2, "0");
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const year = date.getUTCFullYear();
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");

  return `${day}-${month}-${year} ${hours}:${minutes}`;
}
