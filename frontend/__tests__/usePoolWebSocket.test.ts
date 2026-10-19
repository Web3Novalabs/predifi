import { act, renderHook } from "@testing-library/react";
import { usePoolWebSocket } from "@/lib/hooks/usePoolWebSocket";

jest.mock("@/lib/api/pools", () => ({
  wsBaseUrl: () => "ws://api.test",
}));

class MockWebSocket {
  static latest: MockWebSocket;
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  readonly url: string;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.latest = this;
  }

  close() {}
}

describe("usePoolWebSocket", () => {
  const originalWebSocket = global.WebSocket;

  beforeEach(() => {
    Object.defineProperty(global, "WebSocket", {
      configurable: true,
      value: MockWebSocket,
    });
  });

  afterEach(() => {
    Object.defineProperty(global, "WebSocket", {
      configurable: true,
      value: originalWebSocket,
    });
  });

  it("subscribes to one pool and forwards only matching events", () => {
    const onEvent = jest.fn();
    const { unmount } = renderHook(() =>
      usePoolWebSocket({ poolId: 42, onEvent }),
    );

    expect(MockWebSocket.latest.url).toBe(
      "ws://api.test/api/v1/ws?pool_id=42&events=prediction_placed",
    );

    act(() => {
      MockWebSocket.latest.onmessage?.({
        data: JSON.stringify({
          type: "prediction_placed",
          pool_id: 43,
          outcome: 0,
          amount: 100,
        }),
      });
      MockWebSocket.latest.onmessage?.({
        data: JSON.stringify({
          type: "prediction_placed",
          pool_id: 42,
          outcome: 0,
          amount: 100,
        }),
      });
    });

    expect(onEvent).toHaveBeenCalledTimes(1);
    expect(onEvent).toHaveBeenCalledWith({
      type: "prediction_placed",
      pool_id: 42,
      outcome: 0,
      amount: 100,
    });
    unmount();
  });
});
