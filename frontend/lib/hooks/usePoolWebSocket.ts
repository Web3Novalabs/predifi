"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { wsBaseUrl, type PoolLiveEvent } from "@/lib/api/pools";

export type WsStatus = "connecting" | "open" | "closed" | "error";

export interface UsePoolWebSocketOptions {
  /** When set, only events for this pool are forwarded to `onEvent`. */
  poolId: number;
  /** JWT for the backend WS upgrade. Optional in permissive local setups. */
  token?: string | null;
  /** Called for every matching live event. */
  onEvent?: (event: PoolLiveEvent) => void;
  /** Disable the socket (e.g. while loading). Default true. */
  enabled?: boolean;
  /**
   * Maximum consecutive reconnect attempts before giving up and leaving
   * `status: "error"`. Reset after a successful `onopen`. Default 8.
   */
  maxReconnectAttempts?: number;
}

/** First reconnect delay in milliseconds. */
export const RECONNECT_BASE_DELAY_MS = 1_000;
/** Upper bound for a single reconnect delay in milliseconds. */
export const RECONNECT_MAX_DELAY_MS = 30_000;
/** Default cap on consecutive reconnect attempts. */
export const DEFAULT_MAX_RECONNECT_ATTEMPTS = 8;

/**
 * Exponential backoff with jitter: 1 s, 2 s, 4 s, ... capped at 30 s.
 *
 * The result is spread across [50%, 100%] of the exponential delay so many
 * tabs on one network do not reconnect in lockstep.
 *
 * @param attempt - Zero-based reconnect attempt number.
 * @param random - Injectable RNG returning [0, 1), for tests.
 */
export function reconnectDelay(
  attempt: number,
  random: () => number = Math.random,
): number {
  const exp = Math.min(
    RECONNECT_MAX_DELAY_MS,
    RECONNECT_BASE_DELAY_MS * 2 ** attempt,
  );
  return Math.round(exp / 2 + (random() * exp) / 2);
}

export interface UsePoolWebSocketResult {
  status: WsStatus;
  lastEvent: PoolLiveEvent | null;
  /** Manual reconnect. */
  reconnect: () => void;
}

/**
 * Subscribe to backend WebSocket broadcast and filter events for one pool.
 *
 * Endpoint: `GET /api/v1/ws` (see `backend/src/ws.rs`). Events are JSON with
 * at least `{ type, pool_id }`. Prediction placements update stake totals and
 * counts on the pool detail page.
 */
export function usePoolWebSocket({
  poolId,
  token,
  onEvent,
  enabled = true,
  maxReconnectAttempts = DEFAULT_MAX_RECONNECT_ATTEMPTS,
}: UsePoolWebSocketOptions): UsePoolWebSocketResult {
  const [status, setStatus] = useState<WsStatus>("closed");
  const [lastEvent, setLastEvent] = useState<PoolLiveEvent | null>(null);
  const [nonce, setNonce] = useState(0);
  const onEventRef = useRef(onEvent);
  useEffect(() => {
    onEventRef.current = onEvent;
  }, [onEvent]);

  const reconnect = useCallback(() => setNonce((n) => n + 1), []);

  useEffect(() => {
    if (!enabled || !Number.isFinite(poolId)) {
      return;
    }

    const params = new URLSearchParams();
    params.set("pool_id", String(poolId));
    params.set("events", "prediction_placed");
    if (token) params.set("token", token);
    const qs = params.toString();
    const url = `${wsBaseUrl()}/api/v1/ws${qs ? `?${qs}` : ""}`;

    let closed = false;
    let retryTimer: ReturnType<typeof setTimeout> | undefined;
    let socket: WebSocket | null = null;
    let attempts = 0;
    // True when a reconnect is owed but was deferred because the tab is hidden.
    let pendingWhileHidden = false;

    const isHidden = () =>
      typeof document !== "undefined" && document.visibilityState === "hidden";

    const scheduleReconnect = () => {
      if (attempts >= maxReconnectAttempts) {
        // Give up; the UI can offer the manual `reconnect()` action.
        setStatus("error");
        return;
      }
      if (isHidden()) {
        pendingWhileHidden = true;
        return;
      }
      const delay = reconnectDelay(attempts);
      attempts += 1;
      retryTimer = setTimeout(connect, delay);
    };

    const handleVisibilityChange = () => {
      if (closed) return;
      if (isHidden()) {
        // Pause: drop any scheduled retry and resume once visible again.
        if (retryTimer) {
          clearTimeout(retryTimer);
          retryTimer = undefined;
          pendingWhileHidden = true;
        }
        return;
      }
      if (pendingWhileHidden) {
        pendingWhileHidden = false;
        connect();
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);

    const connect = () => {
      if (closed) return;
      retryTimer = undefined;
      setStatus("connecting");
      socket = new WebSocket(url);

      socket.onopen = () => {
        if (closed) return;
        attempts = 0;
        setStatus("open");
      };

      socket.onerror = () => {
        if (!closed) setStatus("error");
      };

      socket.onclose = () => {
        if (closed) return;
        setStatus("closed");
        scheduleReconnect();
      };

      socket.onmessage = (msg) => {
        try {
          const event = JSON.parse(String(msg.data)) as PoolLiveEvent;
          if (event.pool_id !== poolId) return;
          setLastEvent(event);
          onEventRef.current?.(event);
        } catch {
          // ignore malformed frames
        }
      };
    };

    connect();

    return () => {
      closed = true;
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      if (retryTimer) clearTimeout(retryTimer);
      if (socket) {
        socket.onclose = null;
        socket.close();
      }
    };
  }, [enabled, poolId, token, nonce, maxReconnectAttempts]);

  return { status, lastEvent, reconnect };
}
