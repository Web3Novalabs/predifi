import { renderHook, waitFor } from "@testing-library/react";
import { useOptimisticAction } from "@/lib/hooks/useOptimisticAction";

describe("useOptimisticAction", () => {
  describe("successful action", () => {
    it("applies optimistic update immediately and confirms on success", async () => {
      const onSuccess = jest.fn();
      const commit = jest.fn().mockResolvedValue(15);

      const { result } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
          onSuccess,
        }),
      );

      // Initial state
      expect(result.current.value).toBe(10);
      expect(result.current.status).toBe("idle");
      expect(result.current.isPending).toBe(false);

      // Run the action
      const runPromise = result.current.run(5);

      // Optimistic value is applied immediately
      expect(result.current.value).toBe(15);
      expect(result.current.status).toBe("pending");
      expect(result.current.isPending).toBe(true);

      await runPromise;

      // After commit, status is success
      await waitFor(() => {
        expect(result.current.status).toBe("success");
        expect(result.current.isPending).toBe(false);
        expect(result.current.value).toBe(15);
        expect(result.current.error).toBeNull();
      });

      expect(commit).toHaveBeenCalledWith(5);
      expect(onSuccess).toHaveBeenCalledWith(15);
    });

    it("uses committed value when returned from commit function", async () => {
      const commit = jest.fn().mockResolvedValue(20);

      const { result } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
        }),
      );

      await result.current.run(5);

      await waitFor(() => {
        // Committed value (20) replaces optimistic value (15)
        expect(result.current.value).toBe(20);
        expect(result.current.status).toBe("success");
      });
    });

    it("keeps optimistic value when commit returns void", async () => {
      const commit = jest.fn().mockResolvedValue(undefined);

      const { result } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
        }),
      );

      await result.current.run(5);

      await waitFor(() => {
        // Optimistic value (15) is kept when commit returns void
        expect(result.current.value).toBe(15);
        expect(result.current.status).toBe("success");
      });
    });
  });

  describe("rejected action with rollback", () => {
    it("rolls back optimistic update on commit failure", async () => {
      const onError = jest.fn();
      const commit = jest.fn().mockRejectedValue(new Error("Network error"));

      const { result } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
          onError,
        }),
      );

      // Initial confirmed value
      expect(result.current.value).toBe(10);

      await result.current.run(5);

      // Optimistic value is rolled back to confirmed value
      await waitFor(() => {
        expect(result.current.value).toBe(10);
        expect(result.current.status).toBe("error");
        expect(result.current.isPending).toBe(false);
        expect(result.current.error).toBeInstanceOf(Error);
        expect(result.current.error?.message).toBe("Network error");
      });

      expect(onError).toHaveBeenCalledWith(expect.any(Error));
    });

    it("converts non-Error rejections to Error instances", async () => {
      const onError = jest.fn();
      const commit = jest.fn().mockRejectedValue("string error");

      const { result } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
          onError,
        }),
      );

      await result.current.run(5);

      await waitFor(() => {
        expect(result.current.error).toBeInstanceOf(Error);
        expect(result.current.error?.message).toBe("string error");
      });

      expect(onError).toHaveBeenCalledWith(expect.any(Error));
    });

    it("can reset after error to return to idle state", async () => {
      const commit = jest.fn().mockRejectedValue(new Error("Failed"));

      const { result } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
        }),
      );

      await result.current.run(5);

      await waitFor(() => {
        expect(result.current.status).toBe("error");
        expect(result.current.error).not.toBeNull();
      });

      // Reset clears error and returns to idle
      result.current.reset();

      expect(result.current.status).toBe("idle");
      expect(result.current.error).toBeNull();
      expect(result.current.value).toBe(10);
    });
  });

  describe("overlapping actions", () => {
    it("handles first action rejecting while second succeeds", async () => {
      let callCount = 0;
      const commit = jest.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.reject(new Error("First action failed"));
        }
        return Promise.resolve(25);
      });

      const { result } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
        }),
      );

      // Start first action (+5)
      const firstRun = result.current.run(5);

      // Optimistically at 15
      expect(result.current.value).toBe(15);
      expect(result.current.status).toBe("pending");

      // Start second action before first completes (+10)
      const secondRun = result.current.run(10);

      // Optimistically at 20 (10 + 10, based on confirmed data)
      expect(result.current.value).toBe(20);

      await firstRun;

      // First action failed and rolled back, but second is still pending
      await waitFor(() => {
        expect(result.current.status).toBe("pending");
      });

      await secondRun;

      // Second action succeeded with final value of 25
      await waitFor(() => {
        expect(result.current.value).toBe(25);
        expect(result.current.status).toBe("success");
        expect(result.current.error).toBeNull();
      });

      expect(commit).toHaveBeenCalledTimes(2);
    });

    it("maintains correct state when both overlapping actions fail", async () => {
      const commit = jest
        .fn()
        .mockRejectedValueOnce(new Error("First failed"))
        .mockRejectedValueOnce(new Error("Second failed"));

      const { result } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
        }),
      );

      // Start both actions
      const firstRun = result.current.run(5);
      const secondRun = result.current.run(10);

      await Promise.all([firstRun, secondRun]);

      // Both failed, rolled back to original confirmed value
      await waitFor(() => {
        expect(result.current.value).toBe(10);
        expect(result.current.status).toBe("error");
        expect(result.current.error?.message).toBe("Second failed");
      });
    });

    it("handles rapid successive actions correctly", async () => {
      let resolveCommit: ((value: number) => void) | null = null;
      const commit = jest.fn().mockImplementation(
        () =>
          new Promise<number>((resolve) => {
            resolveCommit = resolve;
          }),
      );

      const { result } = renderHook(() =>
        useOptimisticAction({
          data: 0,
          applyOptimistic: (current, input: number) => current + input,
          commit,
        }),
      );

      // Fire three actions in rapid succession
      result.current.run(1);
      result.current.run(2);
      result.current.run(3);

      // Optimistic value is from the last action (0 + 3)
      expect(result.current.value).toBe(3);
      expect(result.current.status).toBe("pending");

      // Resolve the last commit
      resolveCommit?.(3);

      await waitFor(() => {
        expect(result.current.status).toBe("success");
        expect(result.current.value).toBe(3);
      });
    });
  });

  describe("unmount safety", () => {
    it("does not update state after unmounting", async () => {
      let resolveCommit: ((value: number) => void) | null = null;
      const onSuccess = jest.fn();
      const commit = jest.fn().mockImplementation(
        () =>
          new Promise<number>((resolve) => {
            resolveCommit = resolve;
          }),
      );

      const { result, unmount } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
          onSuccess,
        }),
      );

      result.current.run(5);

      // Unmount before commit resolves
      unmount();

      // Resolve after unmount
      resolveCommit?.(15);

      // Wait a bit to ensure no state updates attempted
      await new Promise((resolve) => setTimeout(resolve, 100));

      // onSuccess should not be called after unmount
      expect(onSuccess).not.toHaveBeenCalled();
    });

    it("does not update state on error after unmounting", async () => {
      let rejectCommit: ((error: Error) => void) | null = null;
      const onError = jest.fn();
      const commit = jest.fn().mockImplementation(
        () =>
          new Promise<number>((_, reject) => {
            rejectCommit = reject;
          }),
      );

      const { result, unmount } = renderHook(() =>
        useOptimisticAction({
          data: 10,
          applyOptimistic: (current, input: number) => current + input,
          commit,
          onError,
        }),
      );

      result.current.run(5);

      // Unmount before commit rejects
      unmount();

      // Reject after unmount
      rejectCommit?.(new Error("Failed"));

      // Wait a bit to ensure no state updates attempted
      await new Promise((resolve) => setTimeout(resolve, 100));

      // onError should not be called after unmount
      expect(onError).not.toHaveBeenCalled();
    });
  });

  describe("complex data types", () => {
    interface Item {
      id: string;
      count: number;
    }

    it("works with object data", async () => {
      const commit = jest.fn().mockResolvedValue({ id: "a", count: 5 });

      const { result } = renderHook(() =>
        useOptimisticAction<Item, number>({
          data: { id: "a", count: 0 },
          applyOptimistic: (current, input) => ({
            ...current,
            count: current.count + input,
          }),
          commit,
        }),
      );

      await result.current.run(5);

      await waitFor(() => {
        expect(result.current.value).toEqual({ id: "a", count: 5 });
        expect(result.current.status).toBe("success");
      });
    });

    it("works with array data", async () => {
      const commit = jest.fn().mockResolvedValue([1, 2, 3, 4]);

      const { result } = renderHook(() =>
        useOptimisticAction<number[], number>({
          data: [1, 2, 3],
          applyOptimistic: (current, input) => [...current, input],
          commit,
        }),
      );

      await result.current.run(4);

      await waitFor(() => {
        expect(result.current.value).toEqual([1, 2, 3, 4]);
        expect(result.current.status).toBe("success");
      });
    });
  });
});
