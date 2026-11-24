import { formatUtcDateTime } from "@/lib/date";

describe("formatUtcDateTime", () => {
  it("formats a Date input as DD-MM-YYYY HH:mm in UTC", () => {
    const date = new Date(Date.UTC(2026, 0, 5, 9, 7));
    expect(formatUtcDateTime(date)).toBe("05-01-2026 09:07");
  });

  it("formats a millisecond timestamp", () => {
    const timestamp = Date.UTC(2026, 8, 26, 14, 30);
    expect(formatUtcDateTime(timestamp)).toBe("26-09-2026 14:30");
  });

  it("formats an ISO string", () => {
    expect(formatUtcDateTime("2026-03-15T08:45:00.000Z")).toBe(
      "15-03-2026 08:45"
    );
  });

  it("zero-pads single-digit day, month, hour, and minute", () => {
    const date = new Date(Date.UTC(2026, 2, 4, 3, 5));
    expect(formatUtcDateTime(date)).toBe("04-03-2026 03:05");
  });

  it('returns "" for invalid input', () => {
    expect(formatUtcDateTime("not-a-date")).toBe("");
    expect(formatUtcDateTime(Number.NaN)).toBe("");
  });
});
