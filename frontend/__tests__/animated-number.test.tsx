import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import { AnimatedNumber } from "@/components/pool/AnimatedNumber";

describe("AnimatedNumber", () => {
  it("renders the final value", () => {
    render(<AnimatedNumber value={42} />);
    expect(screen.getByText("42")).toBeInTheDocument();
  });

  it("updates when the value prop changes", async () => {
    const { rerender } = render(
      <AnimatedNumber value={10} durationMs={0} />,
    );
    expect(screen.getByText("10")).toBeInTheDocument();

    rerender(<AnimatedNumber value={25} durationMs={0} />);

    await waitFor(() => {
      expect(screen.getByText("25")).toBeInTheDocument();
    });
  });

  it("renders a plain readable number for 0 (not NaN)", () => {
    render(<AnimatedNumber value={0} />);
    const text = screen.getByText("0").textContent;
    expect(text).toBe("0");
    expect(text).not.toMatch(/NaN/i);
  });

  it("renders a plain readable number for large values (not NaN)", () => {
    render(<AnimatedNumber value={1_234_567_890} />);
    const el = screen.getByText(/1,234,567,890/);
    expect(el).toBeInTheDocument();
    expect(el.textContent).not.toMatch(/NaN/i);
  });
});
