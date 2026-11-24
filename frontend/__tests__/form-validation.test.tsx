import { validateCreatePool, type CreatePoolFormValues } from "@/lib/validations/poolCreation";

describe("Pool Creation Form Validation", () => {
  const validValues: CreatePoolFormValues = {
    name: "Will Bitcoin hit 100k by December?",
    description: "Prediction market pool for BTC price action in 2026.",
    category: "Crypto",
    outcomes: ["Yes", "No"],
    minStake: "5",
    maxStake: "500",
    closeTime: new Date(Date.now() + 3600 * 1000 * 24).toISOString().slice(0, 16),
    token: "XLM",
    termsAccepted: true,
  };

  it("returns no errors for valid form inputs", () => {
    const errors = validateCreatePool(validValues);
    expect(Object.keys(errors)).toHaveLength(0);
  });

  it("validates pool name length rules", () => {
    // Too short
    const shortErrors = validateCreatePool({ ...validValues, name: "BTC" });
    expect(shortErrors.name).toBe("Pool name must be at least 5 characters.");

    // Empty
    const emptyErrors = validateCreatePool({ ...validValues, name: "   " });
    expect(emptyErrors.name).toBe("Pool name is required.");
  });

  it("validates outcomes uniqueness and count", () => {
    // Duplicate outcomes
    const dupErrors = validateCreatePool({ ...validValues, outcomes: ["Yes", "yes"] });
    expect(dupErrors.outcomeErrors).toBeDefined();
    expect(dupErrors.outcomeErrors?.[1]).toBe("Outcome labels must be unique.");
  });

  it("validates stake bounds logic (maxStake >= minStake)", () => {
    const stakeErrors = validateCreatePool({
      ...validValues,
      minStake: "100",
      maxStake: "10",
    });

    expect(stakeErrors.maxStake).toBe("Maximum stake must be greater than or equal to the minimum.");
  });

  it("validates close time must be in the future (at least 30 minutes)", () => {
    const pastTime = new Date(Date.now() - 3600 * 1000).toISOString().slice(0, 16);
    const timeErrors = validateCreatePool({
      ...validValues,
      closeTime: pastTime,
    });

    expect(timeErrors.closeTime).toBe("Close time must be in the future.");
  });

  it("requires terms acceptance", () => {
    const termsErrors = validateCreatePool({
      ...validValues,
      termsAccepted: false,
    });

    expect(termsErrors.termsAccepted).toBe("You must accept the terms to create a pool.");
  });
});
