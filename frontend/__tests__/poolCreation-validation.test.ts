import { validateCreatePool, type CreatePoolFormValues } from "@/lib/validations/poolCreation";

describe("Pool Creation Validation Rules", () => {
  const futureCloseTime = new Date(Date.now() + 3600 * 1000 * 24).toISOString().slice(0, 16);

  const validFormValues: CreatePoolFormValues = {
    name: "Will Premier League final be won by Arsenal?",
    description: "Prediction pool for Premier League championship.",
    category: "Sports",
    outcomes: ["Yes", "No"],
    minStake: "10",
    maxStake: "1000",
    closeTime: futureCloseTime,
    token: "XLM",
    termsAccepted: true,
  };

  it("passes validation with valid input", () => {
    const errors = validateCreatePool(validFormValues);
    expect(errors).toEqual({});
    expect(Object.keys(errors)).toHaveLength(0);
  });

  it("fails validation when title (name) is empty", () => {
    const errors = validateCreatePool({
      ...validFormValues,
      name: "",
    });
    expect(errors.name).toBe("Pool name is required.");
  });

  it("fails validation when close time (end time) is in the past", () => {
    const pastCloseTime = new Date(Date.now() - 3600 * 1000).toISOString().slice(0, 16);
    const errors = validateCreatePool({
      ...validFormValues,
      closeTime: pastCloseTime,
    });
    expect(errors.closeTime).toBe("Close time must be in the future.");
  });

  it("fails validation when too few outcome options are provided", () => {
    const errors = validateCreatePool({
      ...validFormValues,
      outcomes: ["Only One Outcome"],
    });
    expect(errors.outcomes).toBe("A pool needs at least 2 outcomes.");
  });

  it("fails validation when minimum stake is below the minimum allowed limit", () => {
    const errors = validateCreatePool({
      ...validFormValues,
      token: "XLM",
      minStake: "0.5",
    });
    expect(errors.minStake).toBe("Minimum stake must be at least 1 XLM.");
  });
});
