import { act, renderHook } from "@testing-library/react";
import {
  DEFAULT_MAX_RECONNECT_ATTEMPTS,
  reconnectDelay,
  usePoolWebSocket,
} from "@/lib/hooks/usePoolWebSocket";

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onmessage: ((msg: { data: string }) => void) | null = null;
  close = jest.fn();
  constructor(public url: string) {
    MockWebSocket.instances.push(this);
  }
}

const OriginalWebSocket = global.WebSocket;

function setVisibility(state: "visible" | "hidden") {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    get: () => state,
  });
  document.dispatchEvent(new Event("visibilitychange"));
}

const latest = () => MockWebSocket.instances[MockWebSocket.instances.length - 1];

describe("reconnectDelay", () => {
  it("grows exponentially and is capped at 30 s", () => {
    const max = () => 0.999999;
    const min = () => 0;
    expect(reconnectDelay(0, max)).toBe(1000);
    expect(reconnectDelay(1, max)).toBe(2000);
    expect(reconnectDelay(2, max)).toBe(4000);
    expect(reconnectDelay(10, max)).toBe(30000);
    expect(reconnectDelay(0, min)).toBe(500);
    expect(reconnectDelay(10, min)).toBe(15000);
  });
});

describe("usePoolWebSocket reconnect", () => {
  beforeEach(() => {
    jest.useFakeTimers();
    MockWebSocket.instances = [];
    global.WebSocket = MockWebSocket as unknown as typeof WebSocket;
    jest.spyOn(Math, "random").mockReturnValue(0.999999);
    setVisibility("visible");
  });

  afterEach(() => {
    jest.restoreAllMocks();
    jest.useRealTimers();
    global.WebSocket = OriginalWebSocket;
  });

  it("backs off exponentially between reconnects", () => {
    renderHook(() => usePoolWebSocket({ poolId: 1 }));
    expect(MockWebSocket.instances).toHaveLength(1);

    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(999));
    expect(MockWebSocket.instances).toHaveLength(1);
    act(() => jest.advanceTimersByTime(1));
    expect(MockWebSocket.instances).toHaveLength(2);

    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(1999));
    expect(MockWebSocket.instances).toHaveLength(2);
    act(() => jest.advanceTimersByTime(1));
    expect(MockWebSocket.instances).toHaveLength(3);
  });

  it("resets the backoff after a successful open", () => {
    renderHook(() => usePoolWebSocket({ poolId: 1 }));
    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(1000));
    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(2000));
    expect(MockWebSocket.instances).toHaveLength(3);

    act(() => latest().onopen?.());
    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(1000));
    expect(MockWebSocket.instances).toHaveLength(4);
  });

  it("stops after the retry cap and leaves status 'error'", () => {
    const { result } = renderHook(() =>
      usePoolWebSocket({ poolId: 1, maxReconnectAttempts: 2 }),
    );
    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(1000));
    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(2000));
    expect(MockWebSocket.instances).toHaveLength(3);

    act(() => latest().onclose?.());
    expect(result.current.status).toBe("error");
    act(() => jest.advanceTimersByTime(60_000));
    expect(MockWebSocket.instances).toHaveLength(3);
  });

  it("reconnect() starts a fresh connection with a reset budget", () => {
    const { result } = renderHook(() =>
      usePoolWebSocket({ poolId: 1, maxReconnectAttempts: 1 }),
    );
    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(1000));
    act(() => latest().onclose?.());
    expect(result.current.status).toBe("error");

    act(() => result.current.reconnect());
    expect(MockWebSocket.instances).toHaveLength(3);
    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(1000));
    expect(MockWebSocket.instances).toHaveLength(4);
  });

  it("pauses while the tab is hidden and resumes when visible", () => {
    renderHook(() => usePoolWebSocket({ poolId: 1 }));
    act(() => latest().onclose?.());

    act(() => setVisibility("hidden"));
    act(() => jest.advanceTimersByTime(60_000));
    expect(MockWebSocket.instances).toHaveLength(1);

    act(() => setVisibility("visible"));
    expect(MockWebSocket.instances).toHaveLength(2);
  });

  it("does not schedule a retry for a close that happens while hidden", () => {
    renderHook(() => usePoolWebSocket({ poolId: 1 }));
    act(() => setVisibility("hidden"));
    act(() => latest().onclose?.());
    act(() => jest.advanceTimersByTime(60_000));
    expect(MockWebSocket.instances).toHaveLength(1);

    act(() => setVisibility("visible"));
    expect(MockWebSocket.instances).toHaveLength(2);
  });

  it("exposes a sensible default retry cap", () => {
    expect(DEFAULT_MAX_RECONNECT_ATTEMPTS).toBeGreaterThan(0);
  });
});
