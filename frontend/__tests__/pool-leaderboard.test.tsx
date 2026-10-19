import React from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { PoolLeaderboard } from "@/components/pool/PoolLeaderboard";
import * as leaderboardApi from "@/lib/api/leaderboard";
import type { LeaderboardEntry } from "@/lib/api/leaderboard";

jest.mock("@/lib/api/leaderboard");

const mockFetch = leaderboardApi.fetchPoolLeaderboard as jest.MockedFunction<
  typeof leaderboardApi.fetchPoolLeaderboard
>;

const sampleEntries: LeaderboardEntry[] = [
  {
    rank: 1,
    user_address: "GABCDE1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    total_volume: 50000,
    win_rate: 0.75,
    current_streak: 5,
    prediction_count: 40,
    wins: 30,
    settled_count: 40,
  },
  {
    rank: 2,
    user_address: "GXYZ9876543210ZYXWVUTSRQPONMLKJIHGFEDCBA",
    total_volume: 30000,
    win_rate: 0.6,
    current_streak: 2,
    prediction_count: 25,
    wins: 15,
    settled_count: 25,
  },
];

const mockResponse = {
  leaderboard: sampleEntries,
  rank_by: "volume",
  limit: 10,
  offset: 0,
};

describe("PoolLeaderboard (pool browsing flow)", () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("shows a loading indicator while the fetch is in-flight", () => {
    mockFetch.mockReturnValue(new Promise(() => {}));
    render(<PoolLeaderboard poolId={1} />);
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("renders leaderboard entries after a successful fetch", async () => {
    mockFetch.mockResolvedValue(mockResponse);
    render(<PoolLeaderboard poolId={1} />);

    await waitFor(() => {
      expect(screen.queryByText(/loading/i)).not.toBeInTheDocument();
    });

    // Both entries visible (addresses truncated to first 6 + last 4 chars)
    expect(screen.getByText("GABCDE…WXYZ")).toBeInTheDocument();
    expect(screen.getByText("GXYZ98…DCBA")).toBeInTheDocument();

    // Default rank is volume — volumes should be rendered
    expect(screen.getByText("50,000")).toBeInTheDocument();
    expect(screen.getByText("30,000")).toBeInTheDocument();
  });

  it("renders an error message when the fetch rejects", async () => {
    mockFetch.mockRejectedValue(new Error("Network error"));
    render(<PoolLeaderboard poolId={2} />);

    await waitFor(() => {
      expect(screen.getByText("Network error")).toBeInTheDocument();
    });
  });

  it("renders the empty-state message when no entries are returned", async () => {
    mockFetch.mockResolvedValue({ ...mockResponse, leaderboard: [] });
    render(<PoolLeaderboard poolId={3} />);

    await waitFor(() => {
      expect(screen.getByText(/no predictions yet/i)).toBeInTheDocument();
    });
  });

  it("switches to win-rate ranking and re-fetches when the Win Rate tab is clicked", async () => {
    mockFetch.mockResolvedValue(mockResponse);
    render(<PoolLeaderboard poolId={1} />);

    await waitFor(() => {
      expect(screen.queryByText(/loading/i)).not.toBeInTheDocument();
    });

    const winRateBtn = screen.getByRole("button", { name: /win rate/i });
    fireEvent.click(winRateBtn);

    await waitFor(() => {
      // win_rate of 0.75 → "75%"
      expect(screen.getByText("75%")).toBeInTheDocument();
    });

    // fetchPoolLeaderboard called twice: initial + after filter change
    expect(mockFetch).toHaveBeenCalledTimes(2);
    expect(mockFetch).toHaveBeenLastCalledWith(1, expect.objectContaining({ rankBy: "win_rate" }));
  });

  it("switches to streak ranking and shows streak values", async () => {
    mockFetch.mockResolvedValue(mockResponse);
    render(<PoolLeaderboard poolId={1} />);

    await waitFor(() => {
      expect(screen.queryByText(/loading/i)).not.toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /streak/i }));

    await waitFor(() => {
      expect(screen.getByText(/5🔥/)).toBeInTheDocument();
      expect(screen.getByText(/2🔥/)).toBeInTheDocument();
    });
  });

  it("truncates long wallet addresses in the table rows", async () => {
    mockFetch.mockResolvedValue(mockResponse);
    render(<PoolLeaderboard poolId={1} />);

    await waitFor(() => {
      expect(screen.queryByText(/loading/i)).not.toBeInTheDocument();
    });

    // Full addresses must NOT appear
    expect(screen.queryByText("GABCDE1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ")).not.toBeInTheDocument();
    expect(screen.queryByText("GXYZ9876543210ZYXWVUTSRQPONMLKJIHGFEDCBA")).not.toBeInTheDocument();
  });
});
