import { act, renderHook, waitFor } from "@testing-library/react";

import { useOptimisticAction } from "@/lib/hooks/useOptimisticAction";

describe("useOptimisticAction", () => {
  it("applies the optimistic value immediately when the action starts", async () => {
    let resolveAction: (value: string) => void = () => {};
    const commit = jest.fn(
      () =>
        new Promise<string>((resolve) => {
          resolveAction = resolve;
        }),
    );

    const { result } = renderHook(() =>
      useOptimisticAction<string, string>({
        data: "initial",
        applyOptimistic: (_current, input) => input,
        commit,
      }),
    );

    expect(result.current.value).toBe("initial");

    act(() => {
      void result.current.run("optimistic");
    });

    expect(result.current.value).toBe("optimistic");
    expect(result.current.isPending).toBe(true);

    await act(async () => {
      resolveAction("resolved");
    });
  });

  it("keeps the optimistic value when the action resolves successfully", async () => {
    const commit = jest.fn(() => Promise.resolve("resolved"));

    const { result } = renderHook(() =>
      useOptimisticAction<string, string>({
        data: "initial",
        applyOptimistic: (_current, input) => input,
        commit,
      }),
    );

    await act(async () => {
      await result.current.run("optimistic");
    });

    await waitFor(() => {
      expect(result.current.isPending).toBe(false);
    });

    expect(result.current.value).toBe("optimistic");
    expect(result.current.error).toBeNull();
  });

  it("rolls back the optimistic value when the action rejects", async () => {
    const failure = new Error("action failed");
    const commit = jest.fn(() => Promise.reject(failure));

    const { result } = renderHook(() =>
      useOptimisticAction<string, string>({
        data: "initial",
        applyOptimistic: (_current, input) => input,
        commit,
      }),
    );

    await act(async () => {
      await result.current.run("optimistic");
    });

    await waitFor(() => {
      expect(result.current.isPending).toBe(false);
    });

    expect(result.current.value).toBe("initial");
    expect(result.current.error).toBe(failure);
  });
});
