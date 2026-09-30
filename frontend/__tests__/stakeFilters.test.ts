/**
 * Unit tests for lib/stakeFilters.ts
 *
 * The module documents that no function should ever throw — every test
 * that exercises bad input also asserts the call does not throw.
 *
 * Stellar uses 7 decimal places: 1 XLM = 10_000_000 stroops (base units).
 */

import {
  formatStake,
  formatStakeCompact,
  formatUsd,
  formatChartValue,
} from "@/lib/stakeFilters";

// ─── formatStake ─────────────────────────────────────────────────────────────

describe("formatStake", () => {
  it("formats bigint base units (stroops)", () => {
    // 100_000_000 stroops = 10 XLM
    expect(formatStake(100_000_000n, "XLM")).toBe("10.00 XLM");
  });

  it("formats a plain number as a display value", () => {
    // A plain number is treated as already-scaled display units
    expect(formatStake(10, "XLM")).toBe("10.00 XLM");
  });

  it("formats a decimal-string input as base units", () => {
    // No decimal point → treated as raw stroops
    expect(formatStake("100000000", "XLM")).toBe("10.00 XLM");
  });

  it("formats an integer-string input with a pre-formatted suffix", () => {
    // Strings with a leading numeric part are parsed and re-formatted,
    // keeping the trailing non-numeric suffix.
    expect(formatStake("100 strk", "STRK")).toBe("100.00 strk");
  });

  it("formats a decimal string with a dot", () => {
    // "10.5" → scaled to stroops by padEnd(7, '0'), i.e. 105_000_000
    expect(formatStake("10.5", "XLM")).toBe("10.50 XLM");
  });

  it("returns em dash for null", () => {
    expect(formatStake(null, "XLM")).toBe("—");
  });

  it("returns em dash for undefined", () => {
    expect(formatStake(undefined, "XLM")).toBe("—");
  });

  it("returns em dash for a non-numeric string", () => {
    expect(formatStake("not-a-number", "XLM")).toBe("—");
  });

  it("returns em dash for NaN", () => {
    expect(formatStake(NaN, "XLM")).toBe("—");
  });

  it("returns em dash for Infinity", () => {
    expect(formatStake(Infinity, "XLM")).toBe("—");
  });

  it("handles zero", () => {
    expect(formatStake(0, "XLM")).toBe("0.00 XLM");
  });

  it("handles negative values", () => {
    // -100_000_000 stroops = -10 XLM
    expect(formatStake(-100_000_000n, "XLM")).toBe("-10.00 XLM");
  });

  it("respects the dp argument", () => {
    // 12_345_678 stroops = 1.2345678 XLM; with dp=4 → 1.2345
    expect(formatStake(12_345_678n, "XLM", 4)).toBe("1.2345 XLM");
  });

  it("omits the token when token is empty", () => {
    expect(formatStake(100_000_000n)).toBe("10.00");
  });

  it("never throws on any input shape", () => {
    const inputs = [
      100_000_000n,
      10,
      "100000000",
      "10.5",
      "100 strk",
      "not-a-number",
      "",
      "   ",
      null,
      undefined,
      NaN,
      Infinity,
      -Infinity,
      -100_000_000n,
      0,
    ];
    for (const input of inputs) {
      expect(() => formatStake(input as never, "XLM")).not.toThrow();
    }
  });
});

// ─── formatStakeCompact ──────────────────────────────────────────────────────

describe("formatStakeCompact", () => {
  it("formats bigint base units into K/M/B scale", () => {
    // 1_000_000 * 10^7 stroops = 1,000,000 XLM → "1.0M XLM"
    expect(formatStakeCompact(10_000_000_000_000n, "XLM")).toBe("1.0M XLM");
  });

  it("formats a plain number input", () => {
    expect(formatStakeCompact(1500, "XLM")).toBe("1.5K XLM");
  });

  it("formats a decimal-string input", () => {
    // "1500 strk" → parsed as 1500 → "1.5K strk"
    expect(formatStakeCompact("1500 strk", "STRK")).toBe("1.5K strk");
  });

  it("formats an integer-string input as stroops", () => {
    // 15_000_000_000 stroops = 1500 XLM → "1.5K XLM"
    expect(formatStakeCompact("15000000000", "XLM")).toBe("1.5K XLM");
  });

  it("returns em dash for null", () => {
    expect(formatStakeCompact(null, "XLM")).toBe("—");
  });

  it("returns em dash for undefined", () => {
    expect(formatStakeCompact(undefined, "XLM")).toBe("—");
  });

  it("returns em dash for a non-numeric string", () => {
    expect(formatStakeCompact("abc", "XLM")).toBe("—");
  });

  it("returns a full number for small values", () => {
    expect(formatStakeCompact(100_000_000n, "XLM")).toBe("10.00 XLM");
  });

  it("handles billions", () => {
    // 2_000_000_000 * 10^7 stroops = 2B XLM
    expect(formatStakeCompact(20_000_000_000_000_000n, "XLM")).toBe("2.0B XLM");
  });

  it("omits the token when empty", () => {
    expect(formatStakeCompact(1500)).toBe("1.5K");
  });

  it("never throws on any input shape", () => {
    const inputs = [
      10_000_000_000_000n,
      1500,
      "15000000000",
      "1500 strk",
      "abc",
      "",
      null,
      undefined,
      NaN,
      Infinity,
      0,
    ];
    for (const input of inputs) {
      expect(() => formatStakeCompact(input as never, "XLM")).not.toThrow();
    }
  });
});

// ─── formatUsd ───────────────────────────────────────────────────────────────

describe("formatUsd", () => {
  it("formats a plain number", () => {
    expect(formatUsd(15255.25)).toBe("$15,255.25");
  });

  it("formats a decimal-string input", () => {
    expect(formatUsd("1255.68")).toBe("$1,255.68");
  });

  it("formats a string with comma separators", () => {
    expect(formatUsd("1,255.68")).toBe("$1,255.68");
  });

  it("formats an integer-string input", () => {
    expect(formatUsd("1000")).toBe("$1,000.00");
  });

  it("formats a bigint as stroops", () => {
    // 100_000_000 stroops = 10 XLM-equivalent → $10.00
    expect(formatUsd(100_000_000n)).toBe("$10.00");
  });

  it("returns em dash for null", () => {
    expect(formatUsd(null)).toBe("—");
  });

  it("returns em dash for undefined", () => {
    expect(formatUsd(undefined)).toBe("—");
  });

  it("returns em dash for a non-numeric string", () => {
    expect(formatUsd("not-a-number")).toBe("—");
  });

  it("returns em dash for NaN", () => {
    expect(formatUsd(NaN)).toBe("—");
  });

  it("returns em dash for Infinity", () => {
    expect(formatUsd(Infinity)).toBe("—");
  });

  it("respects the dp argument", () => {
    expect(formatUsd(10, 0)).toBe("$10");
  });

  it("handles zero", () => {
    expect(formatUsd(0)).toBe("$0.00");
  });

  it("never throws on any input shape", () => {
    const inputs = [
      15255.25,
      "1255.68",
      "1,255.68",
      "1000",
      100_000_000n,
      "abc",
      "",
      null,
      undefined,
      NaN,
      Infinity,
      0,
    ];
    for (const input of inputs) {
      expect(() => formatUsd(input as never)).not.toThrow();
    }
  });
});

// ─── formatChartValue ────────────────────────────────────────────────────────

describe("formatChartValue", () => {
  it("formats a plain number into K scale", () => {
    expect(formatChartValue(65000)).toBe("$65.0K");
  });

  it("formats a bigint as stroops", () => {
    // The function does NOT divide bigint by STROOP_FACTOR — it uses
    // Number(raw) directly. So 65000n → "$65.0K".
    expect(formatChartValue(65000n)).toBe("$65.0K");
  });

  it("formats a decimal-string input", () => {
    expect(formatChartValue("65000.5")).toBe("$65.0K");
  });

  it("formats an integer-string input", () => {
    expect(formatChartValue("65000")).toBe("$65.0K");
  });

  it("returns em dash for null", () => {
    expect(formatChartValue(null)).toBe("—");
  });

  it("returns em dash for undefined", () => {
    expect(formatChartValue(undefined)).toBe("—");
  });

  it("returns em dash for a non-numeric string", () => {
    expect(formatChartValue("abc")).toBe("—");
  });

  it("returns em dash for NaN", () => {
    expect(formatChartValue(NaN)).toBe("—");
  });

  it("returns zero dollar for value 0", () => {
    expect(formatChartValue(0)).toBe("$0.00");
  });

  it("handles millions", () => {
    expect(formatChartValue(1_500_000)).toBe("$1.5M");
  });

  it("handles billions", () => {
    expect(formatChartValue(2_500_000_000)).toBe("$2.5B");
  });

  it("handles small values with two decimals", () => {
    expect(formatChartValue(12.345)).toBe("$12.35");
  });

  it("handles negative values", () => {
    expect(formatChartValue(-65000)).toBe("-$65.0K");
  });

  it("never throws on any input shape", () => {
    const inputs = [
      65000,
      65000n,
      "65000",
      "65000.5",
      "abc",
      "",
      null,
      undefined,
      NaN,
      Infinity,
      0,
      -65000,
    ];
    for (const input of inputs) {
      expect(() => formatChartValue(input as never)).not.toThrow();
    }
  });
});
