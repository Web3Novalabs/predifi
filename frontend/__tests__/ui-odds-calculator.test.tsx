import React from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { OddsCalculator } from "@/components/ui/odds-calculator";

describe("OddsCalculator", () => {
  describe("Even pools (50/50 split)", () => {
    it("calculates correct odds for an even 50/50 split", () => {
      render(<OddsCalculator token="XLM" />);

      // Default state should be 50/50
      expect(screen.getByText("Outcome A — 50%")).toBeInTheDocument();
      expect(screen.getByText("Outcome B — 50%")).toBeInTheDocument();

      // For 50% probability, odds should be 2.00 (1 / 0.5 = 2)
      const oddsElements = screen.getAllByText(/×2\.00/);
      expect(oddsElements).toHaveLength(2); // Both outcomes should show ×2.00
    });

    it("calculates correct payout for even split with stake", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const stakeInput = screen.getByLabelText(/your stake/i);
      await user.clear(stakeInput);
      await user.type(stakeInput, "100");

      // With 50/50 odds (2x) and 100 XLM stake, payout should be 200 XLM
      const payouts = screen.getAllByText(/200\.00 XLM/);
      expect(payouts.length).toBeGreaterThan(0);
    });
  });

  describe("Heavily skewed pools", () => {
    it("calculates correct odds for heavily skewed 90/10 split", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const slider = screen.getByLabelText(/probability split/i);
      
      // Move slider to 90%
      await user.clear(slider);
      await user.type(slider, "90");

      expect(screen.getByText("Outcome A — 90%")).toBeInTheDocument();
      expect(screen.getByText("Outcome B — 10%")).toBeInTheDocument();

      // Outcome A: 90% probability = 1/0.9 = 1.11 odds
      expect(screen.getByText(/×1\.11/)).toBeInTheDocument();
      
      // Outcome B: 10% probability = 1/0.1 = 10.00 odds
      expect(screen.getByText(/×10\.00/)).toBeInTheDocument();
    });

    it("calculates correct payout for heavily skewed split", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const slider = screen.getByLabelText(/probability split/i);
      const stakeInput = screen.getByLabelText(/your stake/i);

      // Set 95% probability for outcome A
      await user.clear(slider);
      await user.type(slider, "95");

      await user.clear(stakeInput);
      await user.type(stakeInput, "50");

      // Outcome A: 50 * (1/0.95) = 50 * 1.0526 ≈ 52.63
      expect(screen.getByText(/52\.63 XLM/)).toBeInTheDocument();

      // Outcome B: 50 * (1/0.05) = 50 * 20 = 1000.00
      expect(screen.getByText(/1000\.00 XLM/)).toBeInTheDocument();
    });

    it("handles extreme skew of 99/1", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const slider = screen.getByLabelText(/probability split/i);
      await user.clear(slider);
      await user.type(slider, "99");

      expect(screen.getByText("Outcome A — 99%")).toBeInTheDocument();
      expect(screen.getByText("Outcome B — 1%")).toBeInTheDocument();

      // Outcome A: 1/0.99 ≈ 1.01
      expect(screen.getByText(/×1\.01/)).toBeInTheDocument();

      // Outcome B: 1/0.01 = 100.00
      expect(screen.getByText(/×100\.00/)).toBeInTheDocument();
    });
  });

  describe("No stake on one side (boundary conditions)", () => {
    it("handles 1% probability (near-zero stake on outcome A)", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const slider = screen.getByLabelText(/probability split/i);
      await user.clear(slider);
      await user.type(slider, "1");

      expect(screen.getByText("Outcome A — 1%")).toBeInTheDocument();
      expect(screen.getByText("Outcome B — 99%")).toBeInTheDocument();

      // Outcome A: 1/0.01 = 100.00 odds
      expect(screen.getByText(/×100\.00/)).toBeInTheDocument();

      // Outcome B: 1/0.99 ≈ 1.01 odds
      expect(screen.getByText(/×1\.01/)).toBeInTheDocument();
    });

    it("displays zero odds when probability would be zero", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      // The component constrains slider to min=1, max=99
      // So we can't actually reach 0% or 100%, but verify the bounds work
      const slider = screen.getByLabelText(/probability split/i);
      
      // Verify min bound (1%)
      await user.clear(slider);
      await user.type(slider, "1");
      expect(screen.getByText("Outcome A — 1%")).toBeInTheDocument();

      // Verify max bound (99%)
      await user.clear(slider);
      await user.type(slider, "99");
      expect(screen.getByText("Outcome A — 99%")).toBeInTheDocument();
    });

    it("shows dash for payout when stake is zero or empty", () => {
      render(<OddsCalculator token="XLM" />);

      // Without any stake, payouts should show dashes
      const dashes = screen.getAllByText("—");
      // Should have at least 2 dashes (one for each outcome's payout)
      expect(dashes.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe("Transition as stake is added", () => {
    it("updates payout display when stake is added incrementally", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const stakeInput = screen.getByLabelText(/your stake/i);

      // Initially no stake - should show dashes
      expect(screen.getAllByText("—").length).toBeGreaterThan(0);

      // Add first amount
      await user.type(stakeInput, "10");
      expect(screen.getByText(/20\.00 XLM/)).toBeInTheDocument(); // 10 * 2 for 50/50

      // Add more (total 100)
      await user.clear(stakeInput);
      await user.type(stakeInput, "100");
      expect(screen.getByText(/200\.00 XLM/)).toBeInTheDocument(); // 100 * 2

      // Clear stake - should return to dashes
      await user.clear(stakeInput);
      expect(screen.getAllByText("—").length).toBeGreaterThan(0);
    });

    it("updates payout when probability slider changes with existing stake", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const slider = screen.getByLabelText(/probability split/i);
      const stakeInput = screen.getByLabelText(/your stake/i);

      // Set initial stake
      await user.type(stakeInput, "100");
      
      // At 50/50: 100 * 2 = 200
      expect(screen.getByText(/200\.00 XLM/)).toBeInTheDocument();

      // Change to 80/20
      await user.clear(slider);
      await user.type(slider, "80");

      // Outcome A: 100 * (1/0.8) = 125
      expect(screen.getByText(/125\.00 XLM/)).toBeInTheDocument();

      // Outcome B: 100 * (1/0.2) = 500
      expect(screen.getByText(/500\.00 XLM/)).toBeInTheDocument();
    });

    it("handles decimal stake amounts correctly", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const stakeInput = screen.getByLabelText(/your stake/i);
      
      await user.type(stakeInput, "12.50");

      // At 50/50: 12.50 * 2 = 25.00
      expect(screen.getByText(/25\.00 XLM/)).toBeInTheDocument();
    });

    it("handles very small stake amounts", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const stakeInput = screen.getByLabelText(/your stake/i);
      
      await user.type(stakeInput, "0.01");

      // At 50/50: 0.01 * 2 = 0.02
      expect(screen.getByText(/0\.02 XLM/)).toBeInTheDocument();
    });
  });

  describe("Input sanitization", () => {
    it("strips non-numeric characters from stake input", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const stakeInput = screen.getByLabelText(/your stake/i);
      
      await user.type(stakeInput, "abc123def");
      
      // Should only keep the numeric part
      expect(stakeInput).toHaveValue("123");
    });

    it("allows only one decimal point", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const stakeInput = screen.getByLabelText(/your stake/i);
      
      await user.type(stakeInput, "12.34.56");
      
      // Should keep only first decimal point
      expect(stakeInput).toHaveValue("12.3456");
    });

    it("removes leading zeros", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="XLM" />);

      const stakeInput = screen.getByLabelText(/your stake/i);
      
      await user.type(stakeInput, "007");
      
      expect(stakeInput).toHaveValue("7");
    });
  });

  describe("Custom token display", () => {
    it("displays custom token symbol in input and payouts", async () => {
      const user = userEvent.setup();
      render(<OddsCalculator token="USDC" />);

      const stakeInput = screen.getByLabelText(/your stake/i);
      
      // Token suffix should show USDC
      expect(screen.getByText("USDC")).toBeInTheDocument();

      await user.type(stakeInput, "100");

      // Payout should also show USDC
      expect(screen.getByText(/200\.00 USDC/)).toBeInTheDocument();
    });
  });

  describe("Accessibility", () => {
    it("provides proper labels for all inputs", () => {
      render(<OddsCalculator token="XLM" />);

      expect(screen.getByLabelText(/probability split/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/your stake/i)).toBeInTheDocument();
    });

    it("has a descriptive heading", () => {
      render(<OddsCalculator token="XLM" />);

      expect(screen.getByText("Odds Calculator")).toBeInTheDocument();
      expect(
        screen.getByText(/adjust the probability split to see implied odds/i)
      ).toBeInTheDocument();
    });
  });
});
