import React from "react";
import { act, render, screen } from "@testing-library/react";
import { CountdownTimer } from "@/components/pool/CountdownTimer";

describe("CountdownTimer", () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("renders the remaining time correctly", () => {
    const nowMs = 1_700_000_000_000; // fixed epoch ms
    jest.setSystemTime(nowMs);

    const endTime = Math.floor(nowMs / 1000) + 90_061; // 1d 1h 1m 1s
    render(<CountdownTimer endTime={endTime} />);

    const timer = screen.getByLabelText(
      /time remaining 1 days 1 hours 1 minutes 1 seconds/i,
    );
    expect(timer).toBeInTheDocument();
    expect(timer).toHaveTextContent("01");
    expect(timer).toHaveTextContent("d");
    expect(timer).toHaveTextContent("h");
    expect(timer).toHaveTextContent("m");
    expect(timer).toHaveTextContent("s");
  });

  it("ticks down as time advances", () => {
    const nowMs = 1_700_000_000_000;
    jest.setSystemTime(nowMs);

    const endTime = Math.floor(nowMs / 1000) + 5;
    render(<CountdownTimer endTime={endTime} />);

    const initialLabel = screen.getByLabelText(/time remaining/i).getAttribute(
      "aria-label",
    );
    expect(initialLabel).toMatch(/5 seconds/);

    act(() => {
      jest.setSystemTime(nowMs + 2_000);
      jest.advanceTimersByTime(1_000);
    });

    const updatedLabel = screen.getByLabelText(/time remaining/i).getAttribute(
      "aria-label",
    );
    expect(updatedLabel).toMatch(/3 seconds/);
  });

  it("shows the ended state once the target time has passed", () => {
    const nowMs = 1_700_000_000_000;
    jest.setSystemTime(nowMs);

    const endTime = Math.floor(nowMs / 1000) + 2;
    render(<CountdownTimer endTime={endTime} />);

    expect(screen.getByLabelText(/time remaining/i)).toBeInTheDocument();

    act(() => {
      jest.setSystemTime(nowMs + 3_000);
      jest.advanceTimersByTime(1_000);
    });

    expect(screen.getByText("Market closed")).toBeInTheDocument();
    expect(screen.queryByLabelText(/time remaining/i)).not.toBeInTheDocument();
  });
});
