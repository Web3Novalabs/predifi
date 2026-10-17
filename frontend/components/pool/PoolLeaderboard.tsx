"use client";

import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import {
  fetchPoolLeaderboard,
  type LeaderboardEntry,
} from "@/lib/api/leaderboard";

export interface PoolLeaderboardProps {
  poolId: number | string;
}

const RANK_OPTIONS: { value: "volume" | "win_rate" | "streak"; label: string }[] = [
  { value: "volume", label: "Volume" },
  { value: "win_rate", label: "Win Rate" },
  { value: "streak", label: "Streak" },
];

function shortAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}

/**
 * Top stakers/predictors for a single pool (#1930).
 *
 * Thin client-side wrapper around `GET /api/v1/pools/:id/leaderboard`
 * (`fetchPoolLeaderboard`), which already existed in `lib/api/leaderboard.ts`
 * but had no UI consumer until now.
 */
export function PoolLeaderboard({ poolId }: PoolLeaderboardProps) {
  const [rankBy, setRankBy] = useState<"volume" | "win_rate" | "streak">("volume");
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);

    fetchPoolLeaderboard(poolId, { rankBy, limit: 10 })
      .then((res) => {
        if (cancelled) return;
        setEntries(res.leaderboard);
        setError(null);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : "Failed to load pool leaderboard.");
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [poolId, rankBy]);

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-5 space-y-3">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold text-white uppercase tracking-wide">
          Top Predictors
        </h2>
        <div className="flex gap-2">
          {RANK_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => setRankBy(opt.value)}
              className={cn(
                "rounded-full border px-2.5 py-1 text-xs font-medium transition-colors",
                rankBy === opt.value
                  ? "border-[#37B7C3] bg-[#37B7C3]/10 text-[#7DE3EC]"
                  : "border-zinc-800 text-zinc-500 hover:text-white",
              )}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </div>

      {isLoading ? (
        <div className="py-6 text-center text-sm text-zinc-500">Loading…</div>
      ) : error ? (
        <div className="py-6 text-center text-sm text-red-400">{error}</div>
      ) : entries.length === 0 ? (
        <div className="py-6 text-center text-sm text-zinc-500">
          No predictions yet for this pool.
        </div>
      ) : (
        <ul className="divide-y divide-zinc-800">
          {entries.map((entry) => (
            <li
              key={entry.user_address}
              className="flex items-center justify-between gap-4 py-2.5"
            >
              <div className="flex items-center gap-3 min-w-0">
                <span className="w-5 shrink-0 text-xs font-semibold text-zinc-500 tabular-nums">
                  #{entry.rank}
                </span>
                <span className="truncate text-sm font-medium text-white">
                  {shortAddress(entry.user_address)}
                </span>
              </div>
              <div className="flex shrink-0 items-center gap-3 text-sm tabular-nums">
                <span className="text-white font-semibold">
                  {rankBy === "volume" && entry.total_volume.toLocaleString()}
                  {rankBy === "win_rate" && `${(entry.win_rate * 100).toFixed(0)}%`}
                  {rankBy === "streak" && `${entry.current_streak}🔥`}
                </span>
                <span className="text-zinc-500">
                  {entry.prediction_count} predictions
                </span>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
