import { renderHook, act } from "@testing-library/react";
import { useDebounce } from "@/lib/hooks/useDebounce";

describe("useDebounce", () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("returns the initial value immediately", () => {
    const { result } = renderHook(() => useDebounce("initial", 500));
    expect(result.current).toBe("initial");
  });

  it("does not update the value before the delay elapses", () => {
    const { result, rerender } = renderHook(
      ({ value, delay }) => useDebounce(value, delay),
      { initialProps: { value: "first", delay: 500 } },
    );

    rerender({ value: "second", delay: 500 });

    act(() => {
      jest.advanceTimersByTime(499);
    });

    expect(result.current).toBe("first");
  });

  it("updates the value once the delay has fully elapsed", () => {
    const { result, rerender } = renderHook(
      ({ value, delay }) => useDebounce(value, delay),
      { initialProps: { value: "first", delay: 500 } },
    );

    rerender({ value: "second", delay: 500 });

    act(() => {
      jest.advanceTimersByTime(500);
    });

    expect(result.current).toBe("second");
  });

  it("resets the timer on rapid successive changes, keeping only the last value", () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 500),
      { initialProps: { value: "a" } },
    );

    rerender({ value: "b" });
    act(() => {
      jest.advanceTimersByTime(300);
    });

    rerender({ value: "c" });
    act(() => {
      jest.advanceTimersByTime(300);
    });
    // Only 300ms have elapsed since the last change ("c"), so no update yet.
    expect(result.current).toBe("a");

    act(() => {
      jest.advanceTimersByTime(200);
    });
    expect(result.current).toBe("c");
  });

  it("uses the default 300ms delay when none is provided", () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value),
      { initialProps: { value: "first" } },
    );

    rerender({ value: "second" });

    act(() => {
      jest.advanceTimersByTime(299);
    });
    expect(result.current).toBe("first");

    act(() => {
      jest.advanceTimersByTime(1);
    });
    expect(result.current).toBe("second");
  });
});
