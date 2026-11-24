"use client";

/**
 * Optimistic UI updates for predictions and claims (#1388).
 *
 * Chain writes take seconds to confirm. Rather than freezing the UI behind a
 * spinner, apply the expected result immediately, then reconcile: keep it on
 * success, roll back on failure. State lives in a ref-guarded reducer so a
 * response arriving after the component unmounts cannot set state.
 */
import { useCallback, useEffect, useRef, useState } from "react";

export type OptimisticStatus = "idle" | "pending" | "success" | "error";

export interface UseOptimisticActionOptions<TData, TInput> {
  /** Current confirmed value. */
  data: TData;
  /** Produce the value to show while the action is in flight. */
  applyOptimistic: (current: TData, input: TInput) => TData;
  /** Perform the real write. Resolving with a value replaces the optimistic one. */
  commit: (input: TInput) => Promise<TData | void>;
  /** Called after a successful commit — a good place to revalidate SWR. */
  onSuccess?: (data: TData) => void;
  /** Called after a rollback, with the error that caused it. */
  onError?: (error: Error) => void;
}

export interface UseOptimisticActionResult<TData, TInput> {
  /** Optimistic value while pending, confirmed value otherwise. */
  value: TData;
  status: OptimisticStatus;
  isPending: boolean;
  error: Error | null;
  /** Run the action. Never throws — inspect `status` / `error` instead. */
  run: (input: TInput) => Promise<void>;
  /** Return to `idle` and clear any error (e.g. when dismissing a toast). */
  reset: () => void;
}

export function useOptimisticAction<TData, TInput>({
  data,
  applyOptimistic,
  commit,
  onSuccess,
  onError,
}: UseOptimisticActionOptions<TData, TInput>): UseOptimisticActionResult<
  TData,
  TInput
> {
  const [optimistic, setOptimistic] = useState<TData | null>(null);
  const [status, setStatus] = useState<OptimisticStatus>("idle");
  const [error, setError] = useState<Error | null>(null);

  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const run = useCallback(
    async (input: TInput) => {
      const previous = data;
      const predicted = applyOptimistic(previous, input);
      setOptimistic(predicted);
      setStatus("pending");
      setError(null);

      try {
        const confirmed = (await commit(input)) as TData | undefined;
        if (!mounted.current) return;

        // A returned value is authoritative; otherwise keep the optimistic one
        // until the caller revalidates.
        const settled = confirmed ?? predicted;
        setOptimistic(settled);
        setStatus("success");
        onSuccess?.(settled);
      } catch (caught) {
        if (!mounted.current) return;

        // Roll back: drop the optimistic value so `value` falls through to the
        // last confirmed `data`.
        setOptimistic(null);
        const err =
          caught instanceof Error ? caught : new Error(String(caught));
        setError(err);
        setStatus("error");
        onError?.(err);
      }
    },
    [data, applyOptimistic, commit, onSuccess, onError],
  );

  const reset = useCallback(() => {
    setOptimistic(null);
    setStatus("idle");
    setError(null);
  }, []);

  return {
    value: optimistic ?? data,
    status,
    isPending: status === "pending",
    error,
    run,
    reset,
  };
}
