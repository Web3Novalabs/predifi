import { CSSProperties } from "react";
import { cn } from "@/lib/utils";

type SkeletonVariant = "pulse" | "shimmer";

interface SkeletonProps {
  className?: string;
  style?: CSSProperties;
  /**
   * `shimmer` sweeps a highlight across the placeholder; `pulse` fades it in
   * and out. Both respect `prefers-reduced-motion` via Tailwind's `motion-safe`
   * prefix — the placeholder still renders, it just holds still.
   */
  variant?: SkeletonVariant;
}

const variantClasses: Record<SkeletonVariant, string> = {
  pulse: "motion-safe:animate-pulse",
  shimmer:
    "relative overflow-hidden before:absolute before:inset-0 before:-translate-x-full " +
    "before:bg-gradient-to-r before:from-transparent before:via-white/10 before:to-transparent " +
    "motion-safe:before:animate-shimmer",
};

/**
 * Layout-preserving placeholder for content that is still loading.
 *
 * Skeletons are decorative — the loading state itself is announced by the
 * wrapping container (see `skeletons.tsx`), so individual bars are hidden from
 * assistive tech to avoid a stream of meaningless nodes.
 */
export function Skeleton({
  className,
  style,
  variant = "shimmer",
}: SkeletonProps) {
  return (
    <div
      aria-hidden="true"
      className={cn(
        "rounded-md bg-zinc-800/60",
        variantClasses[variant],
        className,
      )}
      style={style}
    />
  );
}

interface SkeletonTextProps extends SkeletonProps {
  /** Number of lines to render. */
  lines?: number;
  /** Width of the final line, which reads as a natural paragraph ending. */
  lastLineWidth?: string;
}

/** A multi-line text placeholder sized to match a paragraph of copy. */
export function SkeletonText({
  lines = 3,
  lastLineWidth = "60%",
  className,
  variant,
}: SkeletonTextProps) {
  return (
    <div className={cn("space-y-2", className)}>
      {Array.from({ length: lines }).map((_, i) => (
        <Skeleton
          key={i}
          variant={variant}
          className="h-4 w-full"
          style={i === lines - 1 ? { width: lastLineWidth } : undefined}
        />
      ))}
    </div>
  );
}

/** A circular placeholder for avatars and token icons. */
export function SkeletonCircle({
  className,
  style,
  variant,
}: SkeletonProps) {
  return (
    <Skeleton
      variant={variant}
      style={style}
      className={cn("h-10 w-10 rounded-full", className)}
    />
  );
}
