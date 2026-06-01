"use client";

import { useState, useCallback, memo } from "react";
import { FixedSizeList, type ListChildComponentProps } from "react-window";
import { cn } from "@/lib/utils";
import { Card, CardContent } from "@/components/ui/card";
import { ChefHat, ChevronRight, Users, Copy } from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Prediction {
  id: string;
  title: string;
  date: string;
  potentialPayout: string;
  stake: string;
  odd: string;
  idNo: string;
  creator: string;
  participants: number;
  status: "Pending" | "Completed";
}

// ---------------------------------------------------------------------------
// Data (module-level constant — never recreated)
// ---------------------------------------------------------------------------

const activePredictions: Prediction[] = [
  {
    id: "1",
    title: "125,000 or above",
    date: "18-04-2025 21:43",
    potentialPayout: "179.52 strk",
    stake: "100 strk",
    odd: "2.54",
    idNo: "19133DK",
    creator: "Best Al this mon...",
    participants: 185,
    status: "Pending",
  },
];

/** Height of each PredictionCard row in pixels (including gap). */
const ITEM_HEIGHT = 220;

/** Maximum visible height of the virtualized list before scrolling kicks in. */
const LIST_MAX_HEIGHT = 600;

// ---------------------------------------------------------------------------
// PredictionCard — memoized list item
// ---------------------------------------------------------------------------

const PredictionCard = memo(function PredictionCard({
  prediction,
}: {
  prediction: Prediction;
}) {
  return (
    <Card className="bg-[#1E1E1E] border-none text-white overflow-hidden">
      <CardContent className="p-0">
        {/* Header */}
        <div className="p-4 flex items-center justify-between border-b border-white/5">
          <div>
            <h4 className="font-bold text-base">{prediction.title}</h4>
            <p className="text-zinc-500 text-xs mt-1">{prediction.date}</p>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-emerald-400 text-xs font-bold">
              {prediction.status}
            </span>
            <ChevronRight className="w-4 h-4 text-zinc-500" />
          </div>
        </div>

        {/* Stats */}
        <div className="p-4 space-y-3">
          <div className="flex justify-between items-center text-sm">
            <span className="text-zinc-400">Potential Payout:</span>
            <span className="font-bold font-mono text-lg">
              {prediction.potentialPayout}
            </span>
          </div>
          <div className="flex justify-between items-center text-sm">
            <span className="text-zinc-400">Stake</span>
            <span className="text-white">{prediction.stake}</span>
          </div>
          <div className="flex justify-between items-center text-sm">
            <span className="text-zinc-400">Odd</span>
            <span className="text-white">{prediction.odd}</span>
          </div>
          <div className="flex justify-between items-center text-sm">
            <span className="text-zinc-400">ID No.</span>
            <div className="flex items-center gap-2 text-white">
              {prediction.idNo}
              <Copy className="w-3 h-3 text-zinc-500 cursor-pointer hover:text-white" />
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="p-4 bg-zinc-900/50 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-indigo-500/20 flex items-center justify-center text-indigo-400">
              <ChefHat className="w-5 h-5" />
            </div>
            <span className="text-sm font-medium text-zinc-300">
              {prediction.creator}
            </span>
          </div>
          <div className="flex items-center gap-1 text-zinc-400 text-sm">
            <Users className="w-4 h-4" />
            <span>{prediction.participants}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
});

PredictionCard.displayName = "PredictionCard";

// ---------------------------------------------------------------------------
// VirtualRow — renderer passed to FixedSizeList
//
// react-window calls this for every visible row, providing `index` and
// `style`. The `style` (position/height) MUST be applied to the outermost
// element so the list can correctly position each row.
// ---------------------------------------------------------------------------

function VirtualRow({ index, style, data }: ListChildComponentProps<Prediction[]>) {
  return (
    // Outer div carries react-window's absolute-position style.
    // Inner div adds the gap between cards via padding-bottom.
    <div style={style}>
      <div className="pb-4">
        <PredictionCard prediction={data[index]} />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PredictionList
// ---------------------------------------------------------------------------

export function PredictionList() {
  const [activeTab, setActiveTab] = useState<"active" | "past">("active");

  const handleTabActive = useCallback(() => setActiveTab("active"), []);
  const handleTabPast = useCallback(() => setActiveTab("past"), []);

  /** Clamp list height so a small dataset doesn't leave empty space. */
  const listHeight = Math.min(
    activePredictions.length * ITEM_HEIGHT,
    LIST_MAX_HEIGHT
  );

  return (
    <div className="space-y-6">
      {/* Tab bar */}
      <div className="flex items-center gap-8 border-b border-zinc-800 pb-1">
        <button
          onClick={handleTabActive}
          className={cn(
            "pb-3 text-sm font-medium transition-colors relative",
            activeTab === "active"
              ? "text-primary"
              : "text-muted-foreground hover:text-white"
          )}
        >
          Active Prediction
          {activeTab === "active" && (
            <span className="absolute bottom-0 left-0 w-full h-0.5 bg-primary" />
          )}
          <span className="ml-2 bg-[#37B7C3] text-[#121212] text-[10px] font-bold px-1.5 py-0.5 rounded-full relative -top-0.5">
            6
          </span>
        </button>

        <button
          onClick={handleTabPast}
          className={cn(
            "pb-3 text-sm font-medium transition-colors relative",
            activeTab === "past"
              ? "text-primary"
              : "text-muted-foreground hover:text-white"
          )}
        >
          Past Predictions
          {activeTab === "past" && (
            <span className="absolute bottom-0 left-0 w-full h-0.5 bg-primary" />
          )}
        </button>
      </div>

      {/* Virtualized list */}
      {activeTab === "active" ? (
        <FixedSizeList
          height={listHeight}
          itemCount={activePredictions.length}
          itemSize={ITEM_HEIGHT}
          width="100%"
          itemData={activePredictions}
          // Remove the default inline overflow so Tailwind/CSS controls scrolling
          style={{ overflow: "auto" }}
        >
          {VirtualRow}
        </FixedSizeList>
      ) : (
        <div className="text-center py-10 text-zinc-500">
          No past predictions found
        </div>
      )}
    </div>
  );
}
