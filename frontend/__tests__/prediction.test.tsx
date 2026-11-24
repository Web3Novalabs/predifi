import React from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { StakeInput } from "@/components/ui/stake-input";
import { PayoutEstimator } from "@/components/ui/payout-estimator";

describe("Prediction Placement Components & Calculations", () => {
  describe("StakeInput", () => {
    it("sanitizes numeric input and limits precision to 7 decimal places", () => {
      const mockOnChange = jest.fn();
      render(<StakeInput label="Stake Amount" value="" onChange={mockOnChange} token="XLM" />);

      const input = screen.getByLabelText("Stake Amount");

      // Enter string with invalid characters and multi decimals
      fireEvent.change(input, { target: { value: "abc10.123456789" } });

      // Sanitized string: "10.1234567" (7 decimal places)
      expect(mockOnChange).toHaveBeenCalledWith("10.1234567", 10.1234567);
    });

    it("displays error state when error prop is provided", () => {
      render(
        <StakeInput
          label="Stake Amount"
          value="100"
          error="Insufficient balance in wallet"
        />
      );

      expect(screen.getByRole("alert")).toHaveTextContent("Insufficient balance in wallet");
      expect(screen.getByLabelText("Stake Amount")).toHaveAttribute("aria-invalid", "true");
    });
  });

  describe("PayoutEstimator", () => {
    it("calculates estimated payout and profit correctly", () => {
      render(<PayoutEstimator token="XLM" />);

      const stakeInput = screen.getByLabelText(/Stake amount/i);
      const oddsInput = screen.getByLabelText(/Odds/i);

      // Enter stake of 100 XLM with 2.5 multiplier
      fireEvent.change(stakeInput, { target: { value: "100" } });
      fireEvent.change(oddsInput, { target: { value: "2.5" } });

      // Total Payout = 100 * 2.5 = 250.00
      // Estimated Profit = 250 - 100 = 150.00
      expect(screen.getByText("250.00")).toBeInTheDocument();
      expect(screen.getByText("150.00")).toBeInTheDocument();
    });

    it("shows warning when odds are less than 1", () => {
      render(<PayoutEstimator token="XLM" />);

      const oddsInput = screen.getByLabelText(/Odds/i);
      fireEvent.change(oddsInput, { target: { value: "0.5" } });

      expect(screen.getByText(/Odds must be 1.00 or higher/i)).toBeInTheDocument();
    });
  });
});
