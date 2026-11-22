import React from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StakeInput } from "@/components/ui/stake-input";

describe("StakeInput", () => {
  describe("basic rendering", () => {
    it("renders with label and placeholder", () => {
      render(
        <StakeInput
          label="Stake Amount"
          placeholder="Enter amount"
          token="XLM"
          onChange={jest.fn()}
        />,
      );

      expect(screen.getByLabelText("Stake Amount")).toBeInTheDocument();
      expect(screen.getByPlaceholderText("Enter amount")).toBeInTheDocument();
      expect(screen.getByText("XLM")).toBeInTheDocument();
    });

    it("displays error message when provided", () => {
      render(
        <StakeInput
          label="Amount"
          error="Insufficient balance"
          onChange={jest.fn()}
        />,
      );

      const errorMessage = screen.getByRole("alert");
      expect(errorMessage).toHaveTextContent("Insufficient balance");
    });

    it("displays helper text when no error", () => {
      render(
        <StakeInput
          label="Amount"
          helperText="Minimum 10 XLM"
          onChange={jest.fn()}
        />,
      );

      expect(screen.getByText("Minimum 10 XLM")).toBeInTheDocument();
    });
  });

  describe("minimum and maximum bounds", () => {
    it("accepts valid minimum value", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "0.0000001");

      expect(onChange).toHaveBeenLastCalledWith("0.0000001", 0.0000001);
    });

    it("accepts large integer values", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "999999999");

      expect(onChange).toHaveBeenLastCalledWith("999999999", 999999999);
    });

    it("allows zero value", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "0");

      expect(onChange).toHaveBeenLastCalledWith("0", 0);
    });

    it("handles empty input", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="100" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.clear(input);

      expect(onChange).toHaveBeenLastCalledWith("", null);
    });
  });

  describe("decimal precision (7 decimals for Soroban stroop)", () => {
    it("accepts exactly 7 decimal places", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "1.2345678");

      // Should truncate to 7 decimals
      expect(onChange).toHaveBeenLastCalledWith("1.2345678", 1.2345678);
    });

    it("truncates precision beyond 7 decimal places", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "1.23456789012345");

      // Should truncate to 7 decimals
      expect(onChange).toHaveBeenLastCalledWith("1.2345678", 1.2345678);
    });

    it("handles typing additional decimals after max precision", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      
      // Type valid 7 decimals
      await user.type(input, "10.1234567");
      expect(onChange).toHaveBeenLastCalledWith("10.1234567", 10.1234567);

      // Try to type more decimals - should be truncated
      await user.type(input, "89");
      expect(onChange).toHaveBeenLastCalledWith("10.1234567", 10.1234567);
    });

    it("allows fewer than 7 decimal places", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "5.12");

      expect(onChange).toHaveBeenLastCalledWith("5.12", 5.12);
    });
  });

  describe("non-numeric input rejection", () => {
    it("strips letters from input", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "abc123def");

      // Only numeric characters remain
      expect(onChange).toHaveBeenLastCalledWith("123", 123);
    });

    it("strips special characters except decimal point", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "12$34@56.78!90");

      // Only digits and decimal remain
      expect(onChange).toHaveBeenLastCalledWith("1234.5678", 1234.5678);
    });

    it("allows only one decimal point", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "12.34.56");

      // Second decimal point is removed
      expect(onChange).toHaveBeenLastCalledWith("12.3456", 12.3456);
    });

    it("handles standalone decimal point", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, ".");

      // Decimal alone returns null
      expect(onChange).toHaveBeenLastCalledWith(".", null);
    });

    it("prevents multiple decimal points in sequence", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "5...5");

      // Only first decimal is kept
      expect(onChange).toHaveBeenLastCalledWith("5.5", 5.5);
    });

    it("removes leading zeros before digits", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "007");

      // Leading zeros stripped
      expect(onChange).toHaveBeenLastCalledWith("7", 7);
    });

    it("preserves leading zero with decimal", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "0.5");

      // 0.5 is valid
      expect(onChange).toHaveBeenLastCalledWith("0.5", 0.5);
    });
  });

  describe("paste of formatted values", () => {
    it("sanitizes pasted value with commas", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox") as HTMLInputElement;
      
      // Simulate paste event
      await user.click(input);
      await user.paste("1,234,567.89");

      // Commas are stripped
      expect(onChange).toHaveBeenLastCalledWith("1234567.89", 1234567.89);
    });

    it("sanitizes pasted value with currency symbols", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox") as HTMLInputElement;
      
      await user.click(input);
      await user.paste("$1,500.00");

      // Currency symbols and commas removed
      expect(onChange).toHaveBeenLastCalledWith("1500.00", 1500);
    });

    it("sanitizes pasted value with spaces", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox") as HTMLInputElement;
      
      await user.click(input);
      await user.paste("10 000.50");

      // Spaces removed
      expect(onChange).toHaveBeenLastCalledWith("10000.50", 10000.5);
    });

    it("truncates pasted value exceeding decimal precision", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox") as HTMLInputElement;
      
      await user.click(input);
      await user.paste("123.456789012345");

      // Truncated to 7 decimals
      expect(onChange).toHaveBeenLastCalledWith("123.4567890", 123.456789);
    });

    it("handles pasted non-numeric text", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox") as HTMLInputElement;
      
      await user.click(input);
      await user.paste("invalid text");

      // All non-numeric stripped, empty result
      expect(onChange).toHaveBeenLastCalledWith("", null);
    });
  });

  describe("controlled component behavior", () => {
    it("updates when value prop changes", () => {
      const { rerender } = render(
        <StakeInput value="10" onChange={jest.fn()} />,
      );

      expect(screen.getByRole("textbox")).toHaveValue("10");

      rerender(<StakeInput value="20" onChange={jest.fn()} />);

      expect(screen.getByRole("textbox")).toHaveValue("20");
    });

    it("calls onChange with both raw string and parsed number", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "42.5");

      expect(onChange).toHaveBeenLastCalledWith("42.5", 42.5);
    });

    it("calls onChange with null for invalid numeric values", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, ".");

      expect(onChange).toHaveBeenLastCalledWith(".", null);
    });
  });

  describe("accessibility", () => {
    it("associates label with input", () => {
      render(
        <StakeInput label="Stake Amount" onChange={jest.fn()} />,
      );

      const input = screen.getByLabelText("Stake Amount");
      expect(input).toBeInTheDocument();
    });

    it("marks input as invalid when error exists", () => {
      render(
        <StakeInput
          label="Amount"
          error="Required field"
          onChange={jest.fn()}
        />,
      );

      const input = screen.getByRole("textbox");
      expect(input).toHaveAttribute("aria-invalid", "true");
    });

    it("associates error message with input", () => {
      render(
        <StakeInput
          label="Amount"
          error="Insufficient balance"
          onChange={jest.fn()}
        />,
      );

      const input = screen.getByRole("textbox");
      const errorId = input.getAttribute("aria-describedby");
      
      expect(errorId).toBeTruthy();
      expect(screen.getByText("Insufficient balance")).toHaveAttribute("id", errorId!);
    });

    it("associates helper text with input when no error", () => {
      render(
        <StakeInput
          label="Amount"
          helperText="Enter stake amount"
          onChange={jest.fn()}
        />,
      );

      const input = screen.getByRole("textbox");
      const helperId = input.getAttribute("aria-describedby");
      
      expect(helperId).toBeTruthy();
      expect(screen.getByText("Enter stake amount")).toHaveAttribute("id", helperId!);
    });
  });

  describe("disabled state", () => {
    it("disables input when disabled prop is true", () => {
      render(
        <StakeInput
          label="Amount"
          disabled
          onChange={jest.fn()}
        />,
      );

      const input = screen.getByRole("textbox");
      expect(input).toBeDisabled();
    });

    it("does not call onChange when disabled", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput
          label="Amount"
          disabled
          onChange={onChange}
        />,
      );

      const input = screen.getByRole("textbox");
      
      // Attempt to type (should be blocked)
      await user.type(input, "123");

      expect(onChange).not.toHaveBeenCalled();
    });
  });

  describe("edge cases that must not reach submit handler", () => {
    it("returns null for empty string preventing invalid submission", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      
      // Type and then delete all
      await user.type(input, "123");
      await user.clear(input);

      // Empty string should give null numeric value
      expect(onChange).toHaveBeenLastCalledWith("", null);
    });

    it("returns null for standalone decimal preventing invalid submission", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, ".");

      // Just a decimal should give null
      expect(onChange).toHaveBeenLastCalledWith(".", null);
    });

    it("does not allow negative numbers", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "-50");

      // Minus sign stripped
      expect(onChange).toHaveBeenLastCalledWith("50", 50);
    });

    it("does not allow scientific notation", async () => {
      const user = userEvent.setup();
      const onChange = jest.fn();

      render(
        <StakeInput value="" onChange={onChange} />,
      );

      const input = screen.getByRole("textbox");
      await user.type(input, "1e5");

      // 'e' is stripped
      expect(onChange).toHaveBeenLastCalledWith("15", 15);
    });
  });
});
