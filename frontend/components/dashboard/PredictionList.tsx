"use client";

import {
  useState,
  useCallback,
  useMemo,
  memo,
  useId,
  type KeyboardEvent,
} from "react";
import { cn } from "@/lib/utils";
import { formatUtcDateTime } from "@/lib/date";
import { formatStake } from "@/lib/stakeFilters";
import useSWR from "swr";
import { Button, Card, CardContent, CopyButton } from "@/components/ui";
import { ChefHat, ChevronRight } from "lucide-react";
import { useCurrentUserAddress } from "@/lib/hooks/useCurrentUserAddress";
import {
  fetchUserPredictions,
  isActivePrediction,
  userPredictionsUrl,
  type UserPrediction,
  type UserPredictionsResponse,
} from "@/lib/api/predictions";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function predictionStatus(prediction: UserPrediction, active: boolean): {
  label: string;
  className: string;
} {
  if (active) return { label: "Pending", className: "text-emerald-400" };
  if (prediction.is_winning_outcome === true)
    return { label: "Won", className: "text-emerald-400" };
  if (prediction.is_winning_outcome === false)
    return { label: "Lost", className: "text-red-400" };
  return { label: "Closed", className: "text-zinc-400" };
}

// ---------------------------------------------------------------------------
// PredictionCard — memoized list item
// ---------------------------------------------------------------------------

const PredictionCard = memo(function PredictionCard({
  prediction,
  active,
}: {
  prediction: UserPrediction;
  active: boolean;
}) {
  const status = predictionStatus(prediction, active);
  const idNo = String(prediction.prediction_id);
  return (
    <Card className="bg-[#1E1E1E] border-none text-white overflow-hidden">
      <CardContent className="p-0">
        {/* Header */}
        <div className="p-4 flex items-center justify-between border-b border-white/5">
          <div>
            <h4 className="font-bold text-base">{prediction.pool_name}</h4>
            <p className="text-zinc-400 text-xs mt-1">
              {formatUtcDateTime(prediction.pool_end_time)}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <span className={cn("text-xs font-bold", status.className)}>
              {status.label}
            </span>
            <ChevronRight className="w-4 h-4 text-zinc-400" aria-hidden="true" />
          </div>
        </div>

        {/* Stats */}
        <div className="p-4 space-y-3">
          <div className="flex justify-between items-center text-sm">
            <span className="text-zinc-400">Stake</span>
            <span className="font-bold font-mono text-lg">
              {formatStake(BigInt(prediction.user_amount), "XLM")}
            </span>
          </div>
          <div className="flex justify-between items-center text-sm">
            <span className="text-zinc-400">Pool total</span>
            <span className="text-white">
              {formatStake(BigInt(prediction.pool_total_stake), "XLM")}
            </span>
          </div>
          <div className="flex justify-between items-center text-sm">
            <span className="text-zinc-400">Outcome</span>
            <span className="text-white">#{prediction.user_outcome}</span>
          </div>
          <div className="flex justify-between items-center text-sm">
            <span className="text-zinc-400">ID No.</span>
            <div className="flex items-center gap-2 text-white">
              {idNo}
              <CopyButton
                text={idNo}
                size="xs"
                aria-label={`Copy prediction ID ${idNo}`}
                copyOptions={{ successDescription: `ID ${idNo} copied to clipboard` }}
              />
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="p-4 bg-zinc-900/50 flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-indigo-500/20 flex items-center justify-center text-indigo-400">
            <ChefHat className="w-5 h-5" aria-hidden="true" />
          </div>
          <span className="text-sm font-medium text-zinc-300">
            {prediction.pool_category}
          </span>
        </div>
      </CardContent>
    </Card>
  );
});

PredictionCard.displayName = "PredictionCard";

// ---------------------------------------------------------------------------
// PredictionList
// ---------------------------------------------------------------------------

export function PredictionList() {
  const [activeTab, setActiveTab] = useState<"active" | "past">("active");
  const id = useId();

  const address = useCurrentUserAddress();
  const { data, error, isLoading, mutate } = useSWR<UserPredictionsResponse>(
    address ? userPredictionsUrl(address) : null,
    fetchUserPredictions,
  );

  const { active, past } = useMemo(() => {
    const now = Date.now();
    const activeList: UserPrediction[] = [];
    const pastList: UserPrediction[] = [];
    for (const p of data?.predictions ?? []) {
      (isActivePrediction(p, now) ? activeList : pastList).push(p);
    }
    return { active: activeList, past: pastList };
  }, [data]);

  const handleTabActive = useCallback(() => setActiveTab("active"), []);
  const handleTabPast = useCallback(() => setActiveTab("past"), []);

  const handleTabKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (
      event.key !== "ArrowLeft" &&
      event.key !== "ArrowRight" &&
      event.key !== "Home" &&
      event.key !== "End"
    ) {
      return;
    }

    event.preventDefault();
    const tabs = Array.from(
      event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>(
        '[role="tab"]',
      ) ?? [],
    );
    if (tabs.length === 0) return;

    const currentIndex = tabs.indexOf(event.currentTarget);
    const nextIndex =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? tabs.length - 1
          : (currentIndex + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) %
            tabs.length;
    tabs[nextIndex].focus();
    tabs[nextIndex].click();
  };

  const renderPanel = (
    items: UserPrediction[],
    emptyMessage: string,
    isActive: boolean,
  ) => {
    if (!address) {
      return (
        <p className="text-center py-10 text-zinc-400">
          Connect your wallet to see your predictions.
        </p>
      );
    }
    if (isLoading) {
      return (
        <p role="status" className="text-center py-10 text-zinc-400">
          Loading predictions…
        </p>
      );
    }
    if (error) {
      return (
        <div role="alert" className="text-center py-10 space-y-3 text-zinc-400">
          <p>Couldn&apos;t load your predictions.</p>
          <Button
            type="button"
            variant="tertiary"
            size="small"
            onClick={() => void mutate()}
          >
            Try again
          </Button>
        </div>
      );
    }
    if (items.length === 0) {
      return <p className="text-center py-10 text-zinc-400">{emptyMessage}</p>;
    }
    return items.map((prediction) => (
      <PredictionCard
        key={prediction.prediction_id}
        prediction={prediction}
        active={isActive}
      />
    ));
  };

  return (
    <div className="space-y-6">
      {/* Tab bar */}
      <div
        role="tablist"
        aria-label="Prediction history"
        className="flex items-center gap-8 border-b border-zinc-800 pb-1"
      >
        <button
          id={`${id}-active-tab`}
          type="button"
          role="tab"
          aria-selected={activeTab === "active"}
          aria-controls={`${id}-active-panel`}
          tabIndex={activeTab === "active" ? 0 : -1}
          onClick={handleTabActive}
          onKeyDown={handleTabKeyDown}
          className={cn(
            "pb-3 text-sm font-medium transition-colors relative",
            activeTab === "active"
              ? "text-primary"
              : "text-muted-foreground hover:text-white",
          )}
        >
          Active Prediction
          {activeTab === "active" && (
            <span className="absolute bottom-0 left-0 w-full h-0.5 bg-primary" />
          )}
          <span className="ml-2 bg-[#37B7C3] text-[#121212] text-[10px] font-bold px-1.5 py-0.5 rounded-full relative -top-0.5">
            {active.length}
          </span>
        </button>

        <button
          id={`${id}-past-tab`}
          type="button"
          role="tab"
          aria-selected={activeTab === "past"}
          aria-controls={`${id}-past-panel`}
          tabIndex={activeTab === "past" ? 0 : -1}
          onClick={handleTabPast}
          onKeyDown={handleTabKeyDown}
          className={cn(
            "pb-3 text-sm font-medium transition-colors relative",
            activeTab === "past"
              ? "text-primary"
              : "text-muted-foreground hover:text-white",
          )}
        >
          Past Predictions
          {activeTab === "past" && (
            <span className="absolute bottom-0 left-0 w-full h-0.5 bg-primary" />
          )}
        </button>
      </div>

      {/* Scrollable list */}
      <div
        id={`${id}-active-panel`}
        role="tabpanel"
        aria-labelledby={`${id}-active-tab`}
        tabIndex={0}
        hidden={activeTab !== "active"}
        className="max-h-[600px] overflow-y-auto space-y-4 pr-2"
      >
        {renderPanel(active, "No active predictions yet", true)}
      </div>
      <div
        id={`${id}-past-panel`}
        role="tabpanel"
        aria-labelledby={`${id}-past-tab`}
        tabIndex={0}
        hidden={activeTab !== "past"}
        className="max-h-[600px] overflow-y-auto space-y-4 pr-2"
      >
        {renderPanel(past, "No past predictions found", false)}
      </div>
    </div>
  );
}
