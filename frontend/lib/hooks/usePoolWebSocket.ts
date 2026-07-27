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
    if (token) params.set("token", token);
    const qs = params.toString();
    const url = `${wsBaseUrl()}/api/v1/ws${qs ? `?${qs}` : ""}`;

    let closed = false;
    let retryTimer: ReturnType<typeof setTimeout> | undefined;
    let socket: WebSocket | null = null;

    const connect = () => {
      if (closed) return;
      setStatus("connecting");
      socket = new WebSocket(url);

      socket.onopen = () => {
        if (!closed) setStatus("open");
      };

      socket.onerror = () => {
        if (!closed) setStatus("error");
      };

      socket.onclose = () => {
        if (closed) return;
        setStatus("closed");
        // Simple backoff reconnect
        retryTimer = setTimeout(connect, 2_500);
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
      if (retryTimer) clearTimeout(retryTimer);
      if (socket) {
        socket.onclose = null;
        socket.close();
      }
    };
  }, [enabled, poolId, token, nonce]);

  return { status, lastEvent, reconnect };
}
