"use client";

import { memo, useState, useMemo, useCallback } from "react";
import { Trophy, Star, TrendingUp, Users, ChevronUp, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle, Skeleton } from "@/components/ui";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface LeaderboardEntry {
  rank: number;
  address: string;
  displayName?: string;
  totalWinnings: string;
  winRate: number;
  predictions: number;
}

export interface LeaderboardProps {
  entries?: LeaderboardEntry[];
  isLoading?: boolean;
  currentUserAddress?: string;
}

type SortKey = "rank" | "winRate" | "predictions" | "totalWinnings";
type SortDir = "asc" | "desc";

// ---------------------------------------------------------------------------
// Rank medal colours
// ---------------------------------------------------------------------------

const RANK_STYLES: Record<number, { bg: string; text: string; border: string; label: string }> = {
  1: { bg: "bg-yellow-400/10", text: "text-yellow-400", border: "border-yellow-400/30", label: "1st" },
  2: { bg: "bg-zinc-400/10",   text: "text-zinc-300",   border: "border-zinc-400/30",   label: "2nd" },
  3: { bg: "bg-orange-400/10", text: "text-orange-400", border: "border-orange-400/30", label: "3rd" },
};

function rankStyle(rank: number) {
  return RANK_STYLES[rank] ?? { bg: "bg-white/5", text: "text-zinc-500", border: "border-white/10", label: `${rank}th` };
}

function parseWinnings(s: string): number {
  return parseFloat(s.replace(/[^0-9.]/g, "")) || 0;
}

// ---------------------------------------------------------------------------
// LeaderboardEmptyState
// ---------------------------------------------------------------------------

export const LeaderboardEmptyState = memo(function LeaderboardEmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-16 px-6 text-center space-y-6">
      <div className="relative">
        <div className="absolute inset-0 rounded-full bg-[#37B7C3]/10 blur-xl scale-150" />
        <div className="relative flex items-end gap-1.5 mb-1">
          <div className="w-8 h-10 rounded-t-md bg-zinc-700/50 border border-white/10 flex items-end justify-center pb-1.5">
            <span className="text-[8px] font-bold text-zinc-500">2</span>
          </div>
          <div className="w-8 h-14 rounded-t-md bg-[#37B7C3]/20 border border-[#37B7C3]/30 flex items-end justify-center pb-1.5">
            <span className="text-[8px] font-bold text-[#37B7C3]">1</span>
          </div>
          <div className="w-8 h-7 rounded-t-md bg-zinc-700/50 border border-white/10 flex items-end justify-center pb-1.5">
            <span className="text-[8px] font-bold text-zinc-500">3</span>
          </div>
        </div>
        <div className="absolute -top-6 left-1/2 -translate-x-1/2">
          <div className="w-10 h-10 rounded-full bg-[#37B7C3]/15 border border-[#37B7C3]/30 flex items-center justify-center shadow-[0_0_16px_rgba(55,183,195,0.25)]">
            <Trophy className="w-5 h-5 text-[#37B7C3]" />
          </div>
        </div>
      </div>
      <div className="space-y-2 pt-2">
        <h3 className="text-base font-semibold text-white">No rankings yet</h3>
        <p className="text-sm text-zinc-500 max-w-xs leading-relaxed">
          The leaderboard will populate once predictions start resolving.
          Make your first prediction to secure a spot.
        </p>
      </div>
      <div className="grid grid-cols-3 gap-3 w-full max-w-xs pt-2">
        {[
          { icon: Star,        label: "Predict" },
          { icon: TrendingUp,  label: "Win"     },
          { icon: Trophy,      label: "Rank up" },
        ].map(({ icon: Icon, label }) => (
          <div
            key={label}
            className="flex flex-col items-center gap-1.5 rounded-xl border border-white/5 bg-white/[0.03] py-3 px-2"
          >
            <Icon className="w-4 h-4 text-[#37B7C3]/70" />
            <span className="text-[10px] text-zinc-500">{label}</span>
          </div>
        ))}
      </div>
    </div>
  );
});

LeaderboardEmptyState.displayName = "LeaderboardEmptyState";

// ---------------------------------------------------------------------------
// LeaderboardRow
// ---------------------------------------------------------------------------

const LeaderboardRow = memo(function LeaderboardRow({
  entry,
  isCurrentUser,
}: {
  entry: LeaderboardEntry;
  isCurrentUser: boolean;
}) {
  const rs = rankStyle(entry.rank);
  const shortAddr = entry.displayName ?? `${entry.address.slice(0, 6)}…${entry.address.slice(-4)}`;

  return (
    <div
      className={cn(
        "flex items-center gap-3 px-4 py-3 rounded-xl border transition-colors",
        isCurrentUser
          ? "border-[#37B7C3]/30 bg-[#37B7C3]/[0.06]"
          : "border-transparent hover:bg-white/[0.03]",
      )}
    >
      <span
        className={cn(
          "w-8 h-8 rounded-lg border text-xs font-bold flex items-center justify-center flex-shrink-0",
          rs.bg, rs.text, rs.border,
        )}
      >
        {entry.rank <= 3 ? rs.label : entry.rank}
      </span>
      <div className="w-8 h-8 rounded-full bg-gradient-to-br from-[#37B7C3]/30 to-indigo-500/30 border border-white/10 flex-shrink-0" />
      <div className="flex-1 min-w-0">
        <p className={cn("text-sm font-medium truncate", isCurrentUser ? "text-[#37B7C3]" : "text-white")}>
          {shortAddr}
          {isCurrentUser && (
            <span className="ml-1.5 text-[10px] bg-[#37B7C3]/20 text-[#37B7C3] px-1.5 py-0.5 rounded-full font-normal">
              you
            </span>
          )}
        </p>
        <p className="text-[10px] text-zinc-500">
          {entry.predictions} predictions · {entry.winRate}% win rate
        </p>
      </div>
      <span className="text-sm font-mono font-medium text-white flex-shrink-0">
        {entry.totalWinnings}
      </span>
    </div>
  );
});

LeaderboardRow.displayName = "LeaderboardRow";

// ---------------------------------------------------------------------------
// Loading skeleton
// ---------------------------------------------------------------------------

function LeaderboardSkeleton() {
  return (
    <div className="space-y-2 px-4 py-2">
      {Array.from({ length: 5 }).map((_, i) => (
        <div key={i} className="flex items-center gap-3 py-3">
          <Skeleton className="w-8 h-8 rounded-lg" />
          <Skeleton className="w-8 h-8 rounded-full" />
          <div className="flex-1 space-y-1.5">
            <Skeleton className="h-3.5 w-32 rounded" />
            <Skeleton className="h-2.5 w-24 rounded" />
          </div>
          <Skeleton className="h-3.5 w-16 rounded" />
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// SortHeader
// ---------------------------------------------------------------------------

const SORT_COLS: { key: SortKey; label: string }[] = [
  { key: "rank",          label: "Rank"     },
  { key: "winRate",       label: "Win Rate" },
  { key: "predictions",   label: "Preds"    },
  { key: "totalWinnings", label: "Winnings" },
];

function SortHeader({
  sortKey,
  dir,
  active,
  onSort,
}: {
  sortKey: SortKey;
  dir: SortDir;
  active: boolean;
  onSort: (k: SortKey) => void;
}) {
  const Icon = active ? (dir === "asc" ? ChevronUp : ChevronDown) : ChevronUp;
  return (
    <button
      type="button"
      onClick={() => onSort(sortKey)}
      className={cn(
        "flex items-center gap-1 hover:text-zinc-300 transition-colors",
        active ? "text-[#37B7C3]" : "text-zinc-500",
      )}
    >
      {SORT_COLS.find((c) => c.key === sortKey)?.label}
      <Icon className={cn("w-3 h-3", !active && "opacity-30")} />
    </button>
  );
}

// ---------------------------------------------------------------------------
// Leaderboard (main export)
// ---------------------------------------------------------------------------

export const Leaderboard = memo(function Leaderboard({
  entries = [],
  isLoading = false,
  currentUserAddress,
}: LeaderboardProps) {
  const [sortKey, setSortKey] = useState<SortKey>("rank");
  const [sortDir, setSortDir] = useState<SortDir>("asc");

  const handleSort = useCallback((key: SortKey) => {
    setSortKey((prev) => {
      if (prev === key) {
        setSortDir((d) => (d === "asc" ? "desc" : "asc"));
        return key;
      }
      setSortDir("asc");
      return key;
    });
  }, []);

  const sorted = useMemo(() => {
    if (entries.length === 0) return entries;
    return [...entries].sort((a, b) => {
      let diff = 0;
      if (sortKey === "rank") diff = a.rank - b.rank;
      else if (sortKey === "winRate") diff = a.winRate - b.winRate;
      else if (sortKey === "predictions") diff = a.predictions - b.predictions;
      else if (sortKey === "totalWinnings") diff = parseWinnings(a.totalWinnings) - parseWinnings(b.totalWinnings);
      return sortDir === "asc" ? diff : -diff;
    });
  }, [entries, sortKey, sortDir]);

  return (
    <Card className="bg-[#121212] border-none text-white overflow-hidden">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base font-semibold text-white flex items-center gap-2">
            <Trophy className="w-4 h-4 text-[#37B7C3]" />
            Leaderboard
          </CardTitle>
          {!isLoading && entries.length > 0 && (
            <span className="flex items-center gap-1 text-[10px] text-zinc-500">
              <Users className="w-3 h-3" />
              {entries.length} players
            </span>
          )}
        </div>
      </CardHeader>

      <CardContent className="p-0 pb-2">
        {isLoading ? (
          <LeaderboardSkeleton />
        ) : entries.length === 0 ? (
          <LeaderboardEmptyState />
        ) : (
          <>
            {/* Sort header bar */}
            <div className="flex items-center gap-4 px-4 py-2 border-b border-white/5 text-[10px] uppercase tracking-wider">
              {SORT_COLS.map((col) => (
                <SortHeader
                  key={col.key}
                  sortKey={col.key}
                  dir={sortDir}
                  active={sortKey === col.key}
                  onSort={handleSort}
                />
              ))}
            </div>
            <div className="space-y-1 px-2 pt-1">
              {sorted.map((entry) => (
                <LeaderboardRow
                  key={entry.address}
                  entry={entry}
                  isCurrentUser={
                    !!currentUserAddress &&
                    entry.address.toLowerCase() === currentUserAddress.toLowerCase()
                  }
                />
              ))}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
});

Leaderboard.displayName = "Leaderboard";
