"use client";

import { useCallback, useMemo, type ReactNode } from "react";
import useSWR from "swr";
import Link from "next/link";
import { ArrowLeft, Radio, Users, Coins } from "lucide-react";
import {
  fetchPoolDetail,
  poolDetailUrl,
  type PoolDetail,
  type PoolLiveEvent,
} from "@/lib/api/pools";
import { usePoolWebSocket } from "@/lib/hooks/usePoolWebSocket";
import { AnimatedNumber } from "@/components/pool/AnimatedNumber";
import { CountdownTimer } from "@/components/pool/CountdownTimer";
import { PoolLeaderboard } from "@/components/pool/PoolLeaderboard";
import { CopyButton, ShareButton } from "@/components/ui";
import { formatStake } from "@/lib/stakeFilters";
import { cn } from "@/lib/utils";
import { useAnnounce } from "@/components/ui/live-region";

export interface PoolDetailViewProps {
  poolId: number;
  initialPool: PoolDetail;
  shareUrl: string;
  shareTitle: string;
  highlightedOutcome?: number;
  /** Optional JWT for authenticated WS (local may be permissive). */
  wsToken?: string | null;
}

function recomputeOdds(
  odds: PoolDetail["odds"],
  totalStake: number,
): PoolDetail["odds"] {
  if (totalStake <= 0) {
    return odds.map((o) => ({ ...o, odds: 0 }));
  }
  return odds.map((o) => ({
    ...o,
    odds: o.stake > 0 ? totalStake / o.stake : 0,
  }));
}

/**
 * Pool detail surface with live WebSocket updates for stakes, counts, odds,
 * and a closing countdown. Value changes animate for feedback.
 */
export function PoolDetailView({
  poolId,
  initialPool,
  shareUrl,
  shareTitle,
  highlightedOutcome,
  wsToken,
}: PoolDetailViewProps) {
  const key = poolDetailUrl(poolId);
  const { data, error, isLoading, mutate } = useSWR(key, fetchPoolDetail, {
    fallbackData: initialPool,
  });
  const announce = useAnnounce();
  const pool = data ?? initialPool;

  const onEvent = useCallback(
    (event: PoolLiveEvent) => {
      if (event.type !== "prediction_placed") return;
      const amount = Number(event.amount ?? 0);
      const outcome = Number(event.outcome ?? 0);
      if (!Number.isSafeInteger(amount) || amount <= 0 || !Number.isInteger(outcome) || outcome < 0) {
        return;
      }

      const outcomeDescription =
        pool?.outcome_descriptions?.[outcome] ?? `Outcome ${outcome + 1}`;
      announce(
        `New prediction of ${formatStake(String(amount), pool?.token ?? "")} on ${outcomeDescription}.`,
      );
      void mutate((current) => {
        const base = current ?? initialPool;
        const nextOdds = base.odds.map((o) =>
          o.outcome === outcome ? { ...o, stake: o.stake + amount } : { ...o },
        );
        if (!nextOdds.some((o) => o.outcome === outcome)) {
          nextOdds.push({ outcome, stake: amount, odds: 0 });
          nextOdds.sort((a, b) => a.outcome - b.outcome);
        }

        const total_stake = base.total_stake + amount;
        return {
          ...base,
          total_stake,
          odds: recomputeOdds(nextOdds, total_stake),
          prediction_count: base.prediction_count === undefined
            ? undefined
            : base.prediction_count + 1,
        };
      }, { revalidate: true });
    },
    [announce, initialPool, mutate, pool],
  );

  const { status } = usePoolWebSocket({
    poolId,
    token: wsToken,
    onEvent,
    enabled: Boolean(pool),
  });

  const maxStake = useMemo(
    () => Math.max(1, ...(pool?.odds.map((o) => o.stake) ?? [1])),
    [pool],
  );

  if (isLoading && !pool) {
    return (
      <main
        id="main-content"
        tabIndex={-1}
        aria-busy="true"
        aria-label="Loading pool details"
        className="mx-auto max-w-3xl space-y-4 p-6"
      >
        <p role="status">Loading pool details…</p>
        <div className="h-8 w-48 animate-pulse rounded bg-white/5" />
        <div className="h-24 animate-pulse rounded-2xl bg-white/5" />
        <div className="h-40 animate-pulse rounded-2xl bg-white/5" />
      </main>
    );
  }

  if ((error && !data) || !pool) {
    return (
      <main
        id="main-content"
        tabIndex={-1}
        className="mx-auto max-w-3xl p-6 text-zinc-400"
      >
        <p role="alert">Could not load pool #{poolId}.</p>
        <Link href="/user/pool-market" className="mt-3 inline-block text-[#37B7C3]">
          Back to market
        </Link>
      </main>
    );
  }

  return (
    <main
      id="main-content"
      tabIndex={-1}
      className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8"
    >
      <div className="mx-auto max-w-3xl space-y-6">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2">
            <Link
              href="/user/pool-market"
              className="inline-flex items-center gap-1 text-xs text-zinc-400 transition hover:text-white"
            >
              <ArrowLeft className="h-3.5 w-3.5" />
              Pool market
            </Link>
            <h1 className="text-2xl font-bold text-white sm:text-3xl">
              {pool.name || `Pool #${pool.pool_id}`}
            </h1>
            <div className="flex flex-wrap items-center gap-2 text-xs text-zinc-400">
              <span className="rounded bg-white/5 px-2 py-0.5 text-zinc-300">
                {pool.category}
              </span>
              <span className="capitalize">{pool.state}</span>
              <LiveBadge status={status} />
            </div>
          </div>
          <div className="flex shrink-0 items-start gap-2">
            <CopyButton
              text={shareUrl}
              size="md"
              className="rounded-md border border-zinc-700 p-2"
              aria-label="Copy pool link"
            />
            <ShareButton url={shareUrl} title={shareTitle} text={pool.name} />
            <CountdownTimer endTime={pool.end_time} />
          </div>
        </div>

        <dl className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <Metric
            icon={<Coins className="h-4 w-4 text-[#37B7C3]" />}
            label="Total stake"
            value={
              <AnimatedNumber
                value={pool.total_stake / 10_000_000}
                decimals={2}
                suffix={` ${pool.token}`}
                className="text-xl font-semibold text-white"
              />
            }
          />
          <Metric
            icon={<Users className="h-4 w-4 text-[#37B7C3]" />}
            label="Live predictions"
            value={
              pool.prediction_count === undefined ? (
                <span aria-label="Prediction count unavailable">—</span>
              ) : (
                <AnimatedNumber
                  value={pool.prediction_count}
                  className="text-xl font-semibold text-white"
                />
              )}
          />
          <Metric
            icon={<Radio className="h-4 w-4 text-[#37B7C3]" />}
            label="Outcomes"
            value={
              <span className="text-xl font-semibold text-white">
                {pool.odds.length || "—"}
              </span>
            }
          />
        </dl>

        <section
          className="space-y-3 rounded-2xl border border-white/10 bg-[#121212] p-5"
          aria-labelledby="live-odds-heading"
        >
          <div className="flex items-center justify-between">
            <h2 id="live-odds-heading" className="text-sm font-semibold text-white">Live odds</h2>
            <p className="text-[10px] text-zinc-400">
              Updates stream over WebSocket
            </p>
          </div>
          <ul className="space-y-3">
            {pool.odds.map((o) => {
              const width = Math.max(4, (o.stake / maxStake) * 100);
              return (
                <li key={o.outcome} className="space-y-1.5">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-zinc-300">
                      {pool.outcome_descriptions?.[o.outcome] || `Outcome ${o.outcome + 1}`}
                      {pool.result != null && Number(pool.result) === o.outcome && (
                        <span className="ml-2 text-xs font-medium text-emerald-400">Winner</span>
                      )}
                      {highlightedOutcome === o.outcome && (
                        <span className="ml-2 text-xs font-medium text-amber-400">Your Pick</span>
                      )}
                    </span>
                    <span className="flex items-center gap-3 text-zinc-400">
                      <span>
                        stake{" "}
                        <AnimatedNumber
                          value={o.stake / 10_000_000}
                          decimals={2}
                          suffix={` ${pool.token}`}
                          className="text-white"
                        />
                      </span>
                      <span>
                        odds{" "}
                        <AnimatedNumber
                          value={o.odds}
                          decimals={2}
                          suffix="x"
                          className="text-[#37B7C3]"
                        />
                      </span>
                    </span>
                  </div>
                  <div className="h-1.5 overflow-hidden rounded-full bg-zinc-800" aria-hidden="true">
                    <div
                      className="h-full rounded-full bg-[#37B7C3] transition-[width] duration-500 ease-out"
                      style={{ width: `${width}%` }}
                    />
                  </div>
                </li>
              );
            })}
            {pool.odds.length === 0 && (
              <li className="text-sm text-zinc-400">No stakes yet — be first.</li>
            )}
          </ul>
        </section>

        <p className="text-[11px] text-zinc-400">
          Creator {pool.creator.slice(0, 6)}…{pool.creator.slice(-4)} · token{" "}
          {pool.token.slice(0, 6)}…
        </p>

        <PoolLeaderboard poolId={pool.pool_id} />
      </div>
    </main>
  );
}

function Metric({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: ReactNode;
}) {
  return (
    <div className="rounded-2xl border border-white/10 bg-[#121212] p-4">
      <dt className="mb-2 flex items-center gap-2 text-[11px] uppercase tracking-wide text-zinc-400">
        <span aria-hidden="true">{icon}</span>
        {label}
      </dt>
      <dd>{value}</dd>
    </div>
  );
}

function LiveBadge({ status }: { status: string }) {
  const live = status === "open";
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[10px] uppercase tracking-wide",
        live
          ? "bg-[#37B7C3]/15 text-[#37B7C3]"
          : "bg-zinc-800 text-zinc-400",
      )}
    >
      <span
        className={cn(
          "h-1.5 w-1.5 rounded-full",
          live ? "animate-pulse bg-[#37B7C3]" : "bg-zinc-400",
        )}
      />
      {live ? "Live" : status}
    </span>
  );
}
