/**
 * stakeFilters.ts
 *
 * Safe math formatting utilities for stake and payout values.
 *
 * Design goals:
 *  - Never throw — every function returns a fallback string on bad input.
 *  - No floating-point drift — all integer-unit math goes through BigInt.
 *  - No extra runtime dependencies — uses only native JS primitives.
 *
 * Soroban / Stellar tokens use 7 decimal places (stroop = 1e-7 XLM).
 * Raw stake values from the API arrive as integer base-units (stroops).
 * Display helpers accept either the raw integer or an already-scaled
 * floating-point number (e.g. from mock data) via the `StakeInput` union.
 */

/** Accepted input shapes for all formatting helpers. */
export type StakeInput = bigint | number | string | null | undefined;

// ─── Internal helpers ────────────────────────────────────────────────────────

const STROOP_DECIMALS = 7; // 1 XLM = 10_000_000 stroops
const STROOP_FACTOR = BigInt(10 ** STROOP_DECIMALS); // 10_000_000n

/** Coerce any StakeInput to a BigInt of base-units, or null on failure. */
function toBigIntBaseUnits(raw: StakeInput): bigint | null {
  if (raw === null || raw === undefined) return null;

  if (typeof raw === "bigint") return raw;

  if (typeof raw === "number") {
    if (!Number.isFinite(raw) || Number.isNaN(raw)) return null;
    // Treat the number as already-scaled display value → convert to stroops
    return BigInt(Math.round(raw * 10 ** STROOP_DECIMALS));
  }

  if (typeof raw === "string") {
    const trimmed = raw.trim();
    if (trimmed === "") return null;
    try {
      // If the string has a decimal point, scale it
      if (trimmed.includes(".")) {
        const [int, frac = ""] = trimmed.split(".");
        const fracPadded = frac.padEnd(STROOP_DECIMALS, "0").slice(0, STROOP_DECIMALS);
        return BigInt(int) * STROOP_FACTOR + BigInt(fracPadded);
      }
      return BigInt(trimmed);
    } catch {
      return null;
    }
  }

  return null;
}

/** Integer division with remainder, returns [quotient, remainder]. */
function divmod(a: bigint, b: bigint): [bigint, bigint] {
  return [a / b, a % b];
}

// ─── Public API ──────────────────────────────────────────────────────────────

/**
 * Format a raw base-unit stake into a human-readable token string.
 *
 * @param raw    - Stake in base units (bigint/number/string) or display float.
 * @param token  - Token symbol appended to the result (e.g. "XLM", "STRK").
 * @param dp     - Decimal places to show (default: 2).
 *
 * @example
 *   formatStake(100_000_000n, "XLM")   // "10.00 XLM"
 *   formatStake("100 strk", "STRK")    // attempts numeric parse
 *   formatStake(0, "XLM")             // "0.00 XLM"
 *   formatStake(null, "XLM")          // "—"
 */
export function formatStake(
  raw: StakeInput,
  token = "",
  dp = 2,
): string {
  // Handle pre-formatted strings like "100 strk" — extract numeric part
  if (typeof raw === "string") {
    const numeric = raw.match(/^[\d.,]+/)?.[0].replace(/,/g, "");
    if (numeric) {
      const parsed = parseFloat(numeric);
      if (Number.isFinite(parsed)) {
        const suffix = raw.replace(/^[\d.,\s]+/, "").trim() || token;
        return `${parsed.toFixed(dp)} ${suffix}`.trim();
      }
    }
  }

  const units = toBigIntBaseUnits(raw);
  if (units === null) return "—";

  const [whole, frac] = divmod(units < 0n ? -units : units, STROOP_FACTOR);
  const sign = units < 0n ? "-" : "";
  const fracStr = frac.toString().padStart(STROOP_DECIMALS, "0").slice(0, dp);
  const formatted = `${sign}${whole.toLocaleString("en-US")}.${fracStr}`;

  return token ? `${formatted} ${token}` : formatted;
}

/**
 * Abbreviated stake for compact display (e.g. metric cards, badges).
 *
 * Thresholds: ≥1B → "1.2B", ≥1M → "1.2M", ≥1K → "1.2K", else full.
 *
 * @param raw    - Same StakeInput as formatStake.
 * @param token  - Optional token symbol.
 */
export function formatStakeCompact(raw: StakeInput, token = ""): string {
  if (typeof raw === "string") {
    const numeric = raw.match(/^[\d.,]+/)?.[0].replace(/,/g, "");
    if (numeric) {
      const parsed = parseFloat(numeric);
      if (Number.isFinite(parsed)) {
        const suffix = raw.replace(/^[\d.,\s]+/, "").trim() || token;
        return `${compactNumber(parsed)} ${suffix}`.trim();
      }
    }
  }

  const units = toBigIntBaseUnits(raw);
  if (units === null) return "—";

  const display = Number(units) / Number(STROOP_FACTOR);
  return token
    ? `${compactNumber(display)} ${token}`
    : compactNumber(display);
}

/** Format a plain number into a compact string (K/M/B). */
function compactNumber(n: number): string {
  if (!Number.isFinite(n)) return "—";
  const abs = Math.abs(n);
  const sign = n < 0 ? "-" : "";
  if (abs >= 1_000_000_000) return `${sign}${(abs / 1_000_000_000).toFixed(1)}B`;
  if (abs >= 1_000_000) return `${sign}${(abs / 1_000_000).toFixed(1)}M`;
  if (abs >= 1_000) return `${sign}${(abs / 1_000).toFixed(1)}K`;
  return `${sign}${abs.toFixed(2)}`;
}

/**
 * Format a USD dollar amount safely.
 *
 * Accepts a number or numeric string. Handles null/undefined gracefully.
 *
 * @example
 *   formatUsd(15255.25)   // "$15,255.25"
 *   formatUsd("1255.68")  // "$1,255.68"
 *   formatUsd(null)       // "—"
 */
export function formatUsd(raw: StakeInput, dp = 2): string {
  if (raw === null || raw === undefined) return "—";

  let value: number;

  if (typeof raw === "bigint") {
    value = Number(raw) / Number(STROOP_FACTOR);
  } else if (typeof raw === "number") {
    value = raw;
  } else {
    const parsed = parseFloat(String(raw).replace(/,/g, ""));
    if (Number.isNaN(parsed)) return "—";
    value = parsed;
  }

  if (!Number.isFinite(value)) return "—";

  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: dp,
    maximumFractionDigits: dp,
  }).format(value);
}

/**
 * Format a chart tooltip value (integer base units → compact dollar string).
 *
 * @example
 *   formatChartValue(65000)  // "$65.0K"
 *   formatChartValue(0)      // "$0.00"
 */
export function formatChartValue(raw: StakeInput): string {
  if (raw === null || raw === undefined) return "—";

  const value =
    typeof raw === "bigint"
      ? Number(raw)
      : typeof raw === "number"
        ? raw
        : parseFloat(String(raw).replace(/,/g, ""));

  if (!Number.isFinite(value) || Number.isNaN(value)) return "—";

  if (value === 0) return "$0.00";

  const abs = Math.abs(value);
  const sign = value < 0 ? "-" : "";

  if (abs >= 1_000_000_000) return `${sign}$${(abs / 1_000_000_000).toFixed(1)}B`;
  if (abs >= 1_000_000) return `${sign}$${(abs / 1_000_000).toFixed(1)}M`;
  if (abs >= 1_000) return `${sign}$${(abs / 1_000).toFixed(1)}K`;
  return `${sign}$${abs.toFixed(2)}`;
}
