"use client";

import { memo } from "react";
import { Trophy, Star, TrendingUp, Users } from "lucide-react";
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

// ---------------------------------------------------------------------------
// LeaderboardEmptyState
// ---------------------------------------------------------------------------

export const LeaderboardEmptyState = memo(function LeaderboardEmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-16 px-6 text-center space-y-6">
      {/* Decorative icon stack */}
      <div className="relative">
        {/* Outer glow ring */}
        <div className="absolute inset-0 rounded-full bg-[#37B7C3]/10 blur-xl scale-150" />

        {/* Podium silhouette */}
        <div className="relative flex items-end gap-1.5 mb-1">
          {/* 2nd place */}
          <div className="w-8 h-10 rounded-t-md bg-zinc-700/50 border border-white/10 flex items-end justify-center pb-1.5">
            <span className="text-[8px] font-bold text-zinc-500">2</span>
          </div>
          {/* 1st place */}
          <div className="w-8 h-14 rounded-t-md bg-[#37B7C3]/20 border border-[#37B7C3]/30 flex items-end justify-center pb-1.5">
            <span className="text-[8px] font-bold text-[#37B7C3]">1</span>
          </div>
          {/* 3rd place */}
          <div className="w-8 h-7 rounded-t-md bg-zinc-700/50 border border-white/10 flex items-end justify-center pb-1.5">
            <span className="text-[8px] font-bold text-zinc-500">3</span>
          </div>
        </div>

        {/* Trophy icon centred over 1st place */}
        <div className="absolute -top-6 left-1/2 -translate-x-1/2">
          <div className="w-10 h-10 rounded-full bg-[#37B7C3]/15 border border-[#37B7C3]/30 flex items-center justify-center shadow-[0_0_16px_rgba(55,183,195,0.25)]">
            <Trophy className="w-5 h-5 text-[#37B7C3]" />
          </div>
        </div>
      </div>

      {/* Heading */}
      <div className="space-y-2 pt-2">
        <h3 className="text-base font-semibold text-white">No rankings yet</h3>
        <p className="text-sm text-zinc-500 max-w-xs leading-relaxed">
          The leaderboard will populate once predictions start resolving.
          Make your first prediction to secure a spot.
        </p>
      </div>

      {/* Hint tiles */}
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
      {/* Rank badge */}
      <span
        className={cn(
          "w-8 h-8 rounded-lg border text-xs font-bold flex items-center justify-center flex-shrink-0",
          rs.bg, rs.text, rs.border,
        )}
      >
        {entry.rank <= 3 ? rs.label : entry.rank}
      </span>

      {/* Avatar placeholder */}
      <div className="w-8 h-8 rounded-full bg-gradient-to-br from-[#37B7C3]/30 to-indigo-500/30 border border-white/10 flex-shrink-0" />

      {/* Identity */}
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

      {/* Winnings */}
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
// Leaderboard (main export)
// ---------------------------------------------------------------------------

export const Leaderboard = memo(function Leaderboard({
  entries = [],
  isLoading = false,
  currentUserAddress,
}: LeaderboardProps) {
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
          <div className="space-y-1 px-2">
            {entries.map((entry) => (
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
        )}
      </CardContent>
    </Card>
  );
});

Leaderboard.displayName = "Leaderboard";
