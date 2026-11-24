/**
 * Layout-matched skeleton screens (#1388).
 *
 * These replace spinners: each one mirrors the shape of the content it stands
 * in for, so the page does not reflow when data lands. Every skeleton block is
 * wrapped in `SkeletonScreen`, which owns the single accessible announcement —
 * the placeholder bars themselves are `aria-hidden`.
 */
import { ReactNode } from "react";
import { cn } from "@/lib/utils";
import { Skeleton, SkeletonCircle, SkeletonText } from "./skeleton";

interface SkeletonScreenProps {
  children: ReactNode;
  className?: string;
  /** Announced to screen readers while the content loads. */
  label?: string;
}

/**
 * Accessible wrapper for a group of skeletons.
 *
 * `role="status"` + `aria-busy` tells assistive tech that content is pending
 * without reading out each placeholder bar, and the visually hidden label says
 * *what* is loading.
 */
export function SkeletonScreen({
  children,
  className,
  label = "Loading content",
}: SkeletonScreenProps) {
  return (
    <div role="status" aria-busy="true" aria-live="polite" className={className}>
      <span className="sr-only">{label}</span>
      {children}
    </div>
  );
}

/** Placeholder matching a single pool card in the pools grid. */
export function PoolCardSkeleton({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        "rounded-lg border border-zinc-800 bg-zinc-900/40 p-4 space-y-4",
        className,
      )}
    >
      {/* Category chip + status pill */}
      <div className="flex items-center justify-between">
        <Skeleton className="h-5 w-20" />
        <Skeleton className="h-5 w-16 rounded-full" />
      </div>

      {/* Pool title */}
      <SkeletonText lines={2} lastLineWidth="70%" />

      {/* Outcome odds row */}
      <div className="grid grid-cols-2 gap-2">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
      </div>

      {/* Total stake + end time footer */}
      <div className="flex items-center justify-between pt-2">
        <Skeleton className="h-4 w-24" />
        <Skeleton className="h-4 w-20" />
      </div>
    </div>
  );
}

/** A grid of pool card placeholders, matching the pools listing layout. */
export function PoolListSkeleton({
  count = 6,
  className,
}: {
  count?: number;
  className?: string;
}) {
  return (
    <SkeletonScreen label="Loading prediction pools">
      <div
        className={cn(
          "grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3",
          className,
        )}
      >
        {Array.from({ length: count }).map((_, i) => (
          <PoolCardSkeleton key={i} />
        ))}
      </div>
    </SkeletonScreen>
  );
}

/** Placeholder for the pool detail page: header, odds panel, and activity. */
export function PoolDetailSkeleton({ className }: { className?: string }) {
  return (
    <SkeletonScreen label="Loading pool details">
      <div className={cn("space-y-6", className)}>
        <div className="flex items-start gap-4">
          <SkeletonCircle className="h-12 w-12" />
          <div className="flex-1 space-y-2">
            <Skeleton className="h-7 w-3/4" />
            <Skeleton className="h-4 w-1/3" />
          </div>
        </div>

        <div className="grid grid-cols-1 gap-4 lg:grid-cols-[2fr_1fr]">
          {/* Odds / chart panel */}
          <Skeleton className="h-64 w-full" />
          {/* Stake form panel */}
          <div className="space-y-3 rounded-lg border border-zinc-800 p-4">
            <Skeleton className="h-5 w-28" />
            <Skeleton className="h-11 w-full" />
            <Skeleton className="h-11 w-full" />
            <Skeleton className="h-11 w-full rounded-md" />
          </div>
        </div>

        <PredictionTableSkeleton rows={4} />
      </div>
    </SkeletonScreen>
  );
}

/** Placeholder rows for prediction/activity tables. */
export function PredictionTableSkeleton({
  rows = 5,
  className,
}: {
  rows?: number;
  className?: string;
}) {
  return (
    <div className={cn("space-y-2", className)}>
      {Array.from({ length: rows }).map((_, i) => (
        <div
          key={i}
          className="flex items-center gap-4 rounded-md border border-zinc-800/60 px-4 py-3"
        >
          <SkeletonCircle className="h-8 w-8" />
          <Skeleton className="h-4 flex-1" />
          <Skeleton className="h-4 w-16" />
          <Skeleton className="h-4 w-20" />
        </div>
      ))}
    </div>
  );
}

/** Placeholder for the dashboard stat tiles. */
export function StatsSkeleton({
  count = 4,
  className,
}: {
  count?: number;
  className?: string;
}) {
  return (
    <SkeletonScreen label="Loading statistics">
      <div
        className={cn("grid grid-cols-2 gap-4 lg:grid-cols-4", className)}
      >
        {Array.from({ length: count }).map((_, i) => (
          <div
            key={i}
            className="space-y-2 rounded-lg border border-zinc-800 p-4"
          >
            <Skeleton className="h-4 w-20" />
            <Skeleton className="h-7 w-28" />
          </div>
        ))}
      </div>
    </SkeletonScreen>
  );
}
