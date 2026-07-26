"use client";

import React, { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import {
  fetchLeaderboard,
  type LeaderboardEntry,
  type WinningsLeaderboardEntry,
  type LeaderboardPeriod,
  type LeaderboardRankBy,
} from "@/lib/api/leaderboard";

const RANK_OPTIONS: { value: LeaderboardRankBy; label: string }[] = [
  { value: "volume", label: "Volume" },
  { value: "winnings", label: "Earnings" },
  { value: "win_rate", label: "Win Rate" },
  { value: "streak", label: "Streak" },
];

const PERIOD_OPTIONS: { value: LeaderboardPeriod; label: string }[] = [
  { value: "all", label: "All-time" },
  { value: "month", label: "Monthly" },
  { value: "week", label: "Weekly" },
];

function shortAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}

function isWinningsEntry(
  entry: LeaderboardEntry | WinningsLeaderboardEntry,
): entry is WinningsLeaderboardEntry {
  return "total_winnings" in entry;
}

export default function LeaderboardPage() {
  const [rankBy, setRankBy] = useState<LeaderboardRankBy>("volume");
  const [period, setPeriod] = useState<LeaderboardPeriod>("all");
  const [entries, setEntries] = useState<
    (LeaderboardEntry | WinningsLeaderboardEntry)[]
  >([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Time-window filtering only applies to volume/win_rate/streak.
  const periodDisabled = rankBy === "winnings";

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    fetchLeaderboard({ rankBy, period: periodDisabled ? undefined : period, limit: 50 })
      .then((res) => {
        if (cancelled) return;
        setEntries(res.leaderboard);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : "Failed to load leaderboard.");
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [rankBy, period, periodDisabled]);

  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8">
      <div className="mx-auto max-w-3xl space-y-6">
        <div className="space-y-1">
          <h1 className="text-3xl font-bold text-white">Leaderboard</h1>
          <p className="text-zinc-400 text-sm">
            Top predictors ranked by volume, earnings, win rate, and streak.
          </p>
        </div>

        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex flex-wrap gap-2">
            {RANK_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                type="button"
                onClick={() => setRankBy(opt.value)}
                className={cn(
                  "rounded-full border px-3 py-1.5 text-sm font-medium transition-colors",
                  rankBy === opt.value
                    ? "border-[#37B7C3] bg-[#37B7C3]/10 text-[#7DE3EC]"
                    : "border-zinc-800 text-zinc-400 hover:text-white",
                )}
              >
                {opt.label}
              </button>
            ))}
          </div>

          <div className="flex gap-2">
            {PERIOD_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                type="button"
                disabled={periodDisabled}
                onClick={() => setPeriod(opt.value)}
                className={cn(
                  "rounded-full border px-3 py-1.5 text-xs font-medium transition-colors",
                  periodDisabled && "cursor-not-allowed opacity-40",
                  !periodDisabled && period === opt.value
                    ? "border-zinc-500 bg-zinc-800 text-white"
                    : "border-zinc-800 text-zinc-500 hover:text-white",
                )}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>

        <div className="rounded-xl border border-zinc-800 bg-zinc-900">
          {isLoading ? (
            <div className="p-8 text-center text-sm text-zinc-500">
              Loading leaderboard…
            </div>
          ) : error ? (
            <div className="p-8 text-center text-sm text-red-400">{error}</div>
          ) : entries.length === 0 ? (
            <div className="p-8 text-center text-sm text-zinc-500">
              No rankings yet for this view.
            </div>
          ) : (
            <ul className="divide-y divide-zinc-800">
              {entries.map((entry) => (
                <li
                  key={entry.user_address}
                  className="flex items-center justify-between gap-4 px-5 py-3.5"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <span className="w-6 shrink-0 text-sm font-semibold text-zinc-500 tabular-nums">
                      #{entry.rank}
                    </span>
                    <span className="truncate text-sm font-medium text-white">
                      {shortAddress(entry.user_address)}
                    </span>
                  </div>
                  <div className="flex shrink-0 items-center gap-4 text-sm tabular-nums">
                    {isWinningsEntry(entry) ? (
                      <>
                        <span className="text-white font-semibold">
                          {entry.total_winnings.toLocaleString()}
                        </span>
                        <span className="text-zinc-500">
                          {(entry.win_rate * 100).toFixed(0)}% win rate
                        </span>
                      </>
                    ) : (
                      <>
                        <span className="text-white font-semibold">
                          {rankBy === "volume" &&
                            entry.total_volume.toLocaleString()}
                          {rankBy === "win_rate" &&
                            `${(entry.win_rate * 100).toFixed(0)}%`}
                          {rankBy === "streak" && `${entry.current_streak}🔥`}
                        </span>
                        <span className="text-zinc-500">
                          {entry.prediction_count} predictions
                        </span>
                      </>
                    )}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
