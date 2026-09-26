import {
  validateCreatePool,
  MIN_STAKE,
  type CreatePoolFormValues,
} from "@/lib/validations/poolCreation";

describe("Pool Creation Validation Rules", () => {
  const futureCloseTime = new Date(Date.now() + 3600 * 1000 * 48)
    .toISOString()
    .slice(0, 16);

  const validFormValues: CreatePoolFormValues = {
    name: "Will Starknet TPS exceed 100 in 2026?",
    description: "Testing network throughput predictions on Starknet mainnet.",
    category: "Technology",
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

  it("fails when title/name is empty and returns correct error message", () => {
    const emptyNameErrors = validateCreatePool({
      ...validFormValues,
      name: "",
    });
    expect(emptyNameErrors.name).toBe("Pool name is required.");

    const whitespaceNameErrors = validateCreatePool({
      ...validFormValues,
      name: "   ",
    });
    expect(whitespaceNameErrors.name).toBe("Pool name is required.");
  });

  it("fails when end time is in the past and returns correct error message", () => {
    const pastTime = new Date(Date.now() - 3600 * 1000)
      .toISOString()
      .slice(0, 16);
    const pastTimeErrors = validateCreatePool({
      ...validFormValues,
      closeTime: pastTime,
    });
    expect(pastTimeErrors.closeTime).toBe("Close time must be in the future.");
  });

  it("fails when there are too few outcome options and returns correct error message", () => {
    const singleOutcomeErrors = validateCreatePool({
      ...validFormValues,
      outcomes: ["Single Outcome"],
    });
    expect(singleOutcomeErrors.outcomes).toBe(
      "A pool needs at least 2 outcomes.",
    );

    const emptyOutcomesErrors = validateCreatePool({
      ...validFormValues,
      outcomes: [],
    });
    expect(emptyOutcomesErrors.outcomes).toBe(
      "A pool needs at least 2 outcomes.",
    );
  });

  it("fails when stake is below minimum and returns correct error message", () => {
    const belowMinErrors = validateCreatePool({
      ...validFormValues,
      token: "XLM",
      minStake: "0.5",
    });
    expect(belowMinErrors.minStake).toBe(
      `Minimum stake must be at least ${MIN_STAKE["XLM"]} XLM.`,
    );

    const belowMinStrkErrors = validateCreatePool({
      ...validFormValues,
      token: "STRK",
      minStake: "0.00001",
    });
    expect(belowMinStrkErrors.minStake).toBe(
      `Minimum stake must be at least ${MIN_STAKE["STRK"]} STRK.`,
    );
  });
});
