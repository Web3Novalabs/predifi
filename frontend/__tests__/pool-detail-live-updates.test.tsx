import React from "react";
import { act, render } from "@testing-library/react";
import { PoolDetailView } from "@/components/pool/PoolDetailView";
import type { PoolDetail, PoolLiveEvent } from "@/lib/api/pools";

let mockSocketOptions:
  | { onEvent?: (event: PoolLiveEvent) => void }
  | undefined;
const mockMutate = jest.fn();
const mockSWR = jest.fn();

jest.mock("swr", () => ({
  __esModule: true,
  default: (...args: unknown[]) => mockSWR(...args),
}));

jest.mock("@/lib/hooks/usePoolWebSocket", () => ({
  usePoolWebSocket: (options: { onEvent?: (event: PoolLiveEvent) => void }) => {
    mockSocketOptions = options;
    return { status: "open" };
  },
}));

jest.mock("@/components/pool/AnimatedNumber", () => ({
  AnimatedNumber: ({ value, decimals = 0, suffix = "" }: {
    value: number;
    decimals?: number;
    suffix?: string;
  }) => <span>{value.toFixed(decimals)}{suffix}</span>,
}));

jest.mock("@/components/pool/CountdownTimer", () => ({
  CountdownTimer: () => null,
}));

jest.mock("@/components/pool/PoolLeaderboard", () => ({
  PoolLeaderboard: () => null,
}));

jest.mock("@/components/ui", () => ({
  CopyButton: () => null,
  ShareButton: () => null,
}));

jest.mock("@/components/ui/live-region", () => ({
  useAnnounce: () => jest.fn(),
}));

const pool: PoolDetail = {
  pool_id: 42,
  name: "Will it rain?",
  category: "Weather",
  tags: [],
  total_stake: 30_000_000,
  end_time: 2_000_000_000,
  created_at: "2025-01-01T00:00:00Z",
  state: "open",
  creator: "GABC123456789",
  token: "XLM",
  result: null,
  odds: [
    { outcome: 0, stake: 20_000_000, odds: 1.5 },
    { outcome: 1, stake: 10_000_000, odds: 3 },
  ],
  outcome_descriptions: ["Yes", "No"],
  prediction_count: 5,
};

describe("PoolDetailView live updates", () => {
  beforeEach(() => {
    mockSocketOptions = undefined;
    mockMutate.mockReset();
    mockSWR.mockReturnValue({
      data: pool,
      error: undefined,
      isLoading: false,
      mutate: mockMutate,
    });
  });

  it("updates the matching pool stake, count, and odds from a prediction event", () => {
    let updatedPool = pool;
    mockMutate.mockImplementation((update, options) => {
      updatedPool = update(pool);
      expect(options).toEqual({ revalidate: true });
    });

    render(
      <PoolDetailView
        poolId={42}
        initialPool={pool}
        shareUrl="https://predifi.app/user/pool-market/42"
        shareTitle="Will it rain?"
      />,
    );

    act(() => {
      mockSocketOptions?.onEvent?.({
        type: "prediction_placed",
        pool_id: 42,
        outcome: 0,
        amount: 5_000_000,
      });
    });

    expect(updatedPool.total_stake).toBe(35_000_000);
    expect(updatedPool.prediction_count).toBe(6);
    expect(updatedPool.odds[0]).toEqual({
      outcome: 0,
      stake: 25_000_000,
      odds: 1.4,
    });
  });

  it("ignores other event types and invalid stake amounts", () => {
    render(
      <PoolDetailView
        poolId={42}
        initialPool={pool}
        shareUrl="https://predifi.app/user/pool-market/42"
        shareTitle="Will it rain?"
      />,
    );

    act(() => {
      mockSocketOptions?.onEvent?.({
        type: "pool_resolved",
        pool_id: 42,
        outcome: 0,
        amount: 5_000_000,
      });
      mockSocketOptions?.onEvent?.({
        type: "prediction_placed",
        pool_id: 42,
        outcome: 0,
        amount: -1,
      });
    });

    expect(mockMutate).not.toHaveBeenCalled();
  });
});
