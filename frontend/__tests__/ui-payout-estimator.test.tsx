import React from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PayoutEstimator } from "@/components/ui/payout-estimator";

describe("PayoutEstimator", () => {
  describe("Typical stake scenarios", () => {
    it("calculates correct payout for a typical stake with reasonable odds", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "100");
      await user.type(oddsInput, "2.5");

      // Payout = 100 * 2.5 = 250
      expect(screen.getByText("250.00")).toBeInTheDocument();

      // Profit = 250 - 100 = 150
      expect(screen.getByText("150.00")).toBeInTheDocument();
    });

    it("calculates correct payout for decimal stake amounts", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "50.75");
      await user.type(oddsInput, "1.8");

      // Payout = 50.75 * 1.8 = 91.35
      expect(screen.getByText("91.35")).toBeInTheDocument();

      // Profit = 91.35 - 50.75 = 40.60
      expect(screen.getByText("40.60")).toBeInTheDocument();
    });

    it("handles minimum valid odds of 1.00", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "100");
      await user.type(oddsInput, "1.00");

      // Payout = 100 * 1.00 = 100
      expect(screen.getByText("100.00")).toBeInTheDocument();

      // Profit = 100 - 100 = 0
      expect(screen.getByText("0.00")).toBeInTheDocument();
    });
  });

  describe("Only stake on one outcome", () => {
    it("calculates payout when user would be the only stake", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      // High odds scenario (only one betting on this outcome)
      await user.type(stakeInput, "50");
      await user.type(oddsInput, "15.00");

      // Payout = 50 * 15 = 750
      expect(screen.getByText("750.00")).toBeInTheDocument();

      // Profit = 750 - 50 = 700
      expect(screen.getByText("700.00")).toBeInTheDocument();
    });

    it("handles extremely high odds correctly", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "10");
      await user.type(oddsInput, "100.00");

      // Payout = 10 * 100 = 1000
      expect(screen.getByText("1000.00")).toBeInTheDocument();

      // Profit = 1000 - 10 = 990
      expect(screen.getByText("990.00")).toBeInTheDocument();
    });
  });

  describe("Zero-stake pool scenarios", () => {
    it("shows dashes when no stake is entered", () => {
      render(<PayoutEstimator token="XLM" />);

      // Should show dashes for all monetary values when no input
      const dashes = screen.getAllByText("—");
      expect(dashes.length).toBeGreaterThanOrEqual(4); // stake, odds, profit, payout
    });

    it("shows dashes when stake is zero", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "0");
      await user.type(oddsInput, "2.5");

      // Even with odds, zero stake should show dashes
      const dashes = screen.getAllByText("—");
      expect(dashes.length).toBeGreaterThan(0);
    });

    it("shows dashes when odds is zero or empty", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);

      await user.type(stakeInput, "100");

      // With stake but no odds, should still show dashes
      const dashes = screen.getAllByText("—");
      expect(dashes.length).toBeGreaterThan(0);
    });
  });

  describe("Rounding at smallest supported unit", () => {
    it("handles 7 decimal place precision (stroop level)", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      // Enter stake with 7 decimal places
      await user.type(stakeInput, "1.0000001");
      await user.type(oddsInput, "2.00");

      // Should accept up to 7 decimal places
      expect(stakeInput).toHaveValue("1.0000001");

      // Payout = 1.0000001 * 2 = 2.0000002
      expect(screen.getByText("2.00")).toBeInTheDocument(); // Display rounds to 2 dp
    });

    it("caps stake input to 7 decimal places", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);

      // Try to enter more than 7 decimal places
      await user.type(stakeInput, "1.123456789");

      // Should cap at 7 decimal places
      expect(stakeInput).toHaveValue("1.1234567");
    });

    it("caps odds input to 2 decimal places for display", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      // Try to enter more than 2 decimal places in odds
      await user.type(oddsInput, "2.999");

      // Should cap odds display to 2 decimal places
      expect(oddsInput).toHaveValue("2.99");
    });

    it("rounds payout display to 2 decimal places", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "10.123");
      await user.type(oddsInput, "1.15");

      // Payout = 10.123 * 1.15 = 11.64145, should round to 11.64
      expect(screen.getByText("11.64")).toBeInTheDocument();
    });

    it("handles very small stake amounts correctly", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "0.0000001");
      await user.type(oddsInput, "2.00");

      // Smallest unit * 2 should still calculate
      // 0.0000001 * 2 = 0.0000002, displayed as 0.00
      expect(screen.getByText("0.00")).toBeInTheDocument();
    });
  });

  describe("Odds validation", () => {
    it("shows warning when odds is less than 1.00", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(oddsInput, "0.5");

      // Should show validation message
      expect(
        screen.getByText(/odds must be 1\.00 or higher/i)
      ).toBeInTheDocument();
    });

    it("does not show warning when odds is valid", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(oddsInput, "2.5");

      // Should not show validation message
      expect(
        screen.queryByText(/odds must be 1\.00 or higher/i)
      ).not.toBeInTheDocument();
    });

    it("visual result card is dimmed when odds invalid", async () => {
      const user = userEvent.setup();
      const { container } = render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "100");
      await user.type(oddsInput, "0.8");

      // Result card should have opacity-40 class when invalid
      const resultCard = container.querySelector(".opacity-40");
      expect(resultCard).toBeInTheDocument();
    });
  });

  describe("Input sanitization", () => {
    it("strips non-numeric characters from stake", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);

      await user.type(stakeInput, "abc123xyz");

      expect(stakeInput).toHaveValue("123");
    });

    it("strips non-numeric characters from odds", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(oddsInput, "abc2.5xyz");

      expect(oddsInput).toHaveValue("2.5");
    });

    it("allows only one decimal point in stake", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);

      await user.type(stakeInput, "12.34.56");

      expect(stakeInput).toHaveValue("12.3456");
    });

    it("removes leading zeros from stake", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);

      await user.type(stakeInput, "0007.5");

      expect(stakeInput).toHaveValue("7.5");
    });
  });

  describe("Visual profit bar", () => {
    it("updates profit bar width based on profit ratio", async () => {
      const user = userEvent.setup();
      const { container } = render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "100");
      await user.type(oddsInput, "2.00");

      // With 2x odds: profit is 100, payout is 200, so profit ratio is 50%
      const profitBar = container.querySelector(".bg-\\[\\#37B7C3\\]");
      expect(profitBar).toBeInTheDocument();

      // Should show 50.0% for profit
      expect(screen.getByText("50.0%")).toBeInTheDocument();
    });

    it("shows correct percentages for stake vs profit", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "100");
      await user.type(oddsInput, "3.00");

      // Payout = 300, Profit = 200
      // Stake portion = 100/300 = 33.3%, Profit portion = 200/300 = 66.7%
      expect(screen.getByText("33.3%")).toBeInTheDocument();
      expect(screen.getByText("66.7%")).toBeInTheDocument();
    });
  });

  describe("Custom token display", () => {
    it("displays custom token symbol throughout the component", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="USDC" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      // Token should appear multiple times in the component
      const tokenElements = screen.getAllByText(/USDC/);
      expect(tokenElements.length).toBeGreaterThan(0);

      await user.type(stakeInput, "100");
      await user.type(oddsInput, "2.00");

      // Results should also show USDC
      expect(screen.getByText(/stake.*usdc/i)).toBeInTheDocument();
      expect(screen.getByText(/payout.*usdc/i)).toBeInTheDocument();
    });
  });

  describe("Accessibility", () => {
    it("provides proper labels for all inputs", () => {
      render(<PayoutEstimator token="XLM" />);

      expect(screen.getByLabelText(/stake amount/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/odds.*multiplier/i)).toBeInTheDocument();
    });

    it("has a descriptive heading", () => {
      render(<PayoutEstimator token="XLM" />);

      expect(screen.getByText("Payout Estimator")).toBeInTheDocument();
      expect(
        screen.getByText(/estimate your returns before staking/i)
      ).toBeInTheDocument();
    });

    it("provides helpful disclaimer text", () => {
      render(<PayoutEstimator token="XLM" />);

      expect(
        screen.getByText(/estimates are illustrative only/i)
      ).toBeInTheDocument();
    });
  });

  describe("Edge cases and boundary conditions", () => {
    it("handles transition from valid to invalid odds", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      // Start with valid values
      await user.type(stakeInput, "100");
      await user.type(oddsInput, "2.00");

      // Should show payout
      expect(screen.getByText("200.00")).toBeInTheDocument();

      // Clear and enter invalid odds
      await user.clear(oddsInput);
      await user.type(oddsInput, "0.5");

      // Should show warning
      expect(
        screen.getByText(/odds must be 1\.00 or higher/i)
      ).toBeInTheDocument();
    });

    it("handles clearing inputs after calculation", async () => {
      const user = userEvent.setup();
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/stake amount/i);
      const oddsInput = screen.getByLabelText(/odds.*multiplier/i);

      await user.type(stakeInput, "100");
      await user.type(oddsInput, "2.00");

      // Should show results
      expect(screen.getByText("200.00")).toBeInTheDocument();

      // Clear stake
      await user.clear(stakeInput);

      // Should return to dashes
      const dashes = screen.getAllByText("—");
      expect(dashes.length).toBeGreaterThan(0);
    });
  });
});
