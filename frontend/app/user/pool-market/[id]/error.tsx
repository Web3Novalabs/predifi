"use client";

import { useEffect } from "react";
import Link from "next/link";
import { ArrowLeft, RefreshCw, AlertTriangle } from "lucide-react";

/**
 * Route-level error boundary for the pool detail segment (#1764).
 *
 * Scoped to /user/pool-market/[id] so a render error inside a pool page
 * does not unmount the surrounding layout or navigation. The root
 * app/error.tsx still catches anything above this boundary.
 */
export default function PoolDetailError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("Pool detail error:", error);
  }, [error]);

  return (
    <div
      role="alert"
      className="flex flex-col items-center justify-center min-h-[400px] rounded-xl border border-red-500/20 bg-red-500/5 px-6 py-12 text-center"
    >
      <AlertTriangle className="mb-4 h-10 w-10 text-red-400" aria-hidden="true" />

      <h2 className="mb-2 text-xl font-semibold text-white">
        Failed to load pool
      </h2>
      <p className="mb-2 max-w-sm text-sm text-zinc-400 leading-relaxed">
        Something went wrong while loading this prediction pool. You can try
        again or browse other pools.
      </p>

      {process.env.NODE_ENV === "development" && error.message && (
        <p className="mb-6 max-w-sm break-all rounded-md border border-red-500/20 bg-red-500/10 px-3 py-2 font-mono text-xs text-red-400">
          {error.message}
        </p>
      )}

      <div className="mt-6 flex flex-col sm:flex-row items-center gap-3">
        <button
          onClick={reset}
          className="inline-flex items-center gap-2 rounded-xl border border-[#37B7C3]/30 bg-[#37B7C3]/10 px-5 py-2.5 text-sm font-medium text-[#37B7C3] transition-all hover:bg-[#37B7C3]/20 hover:border-[#37B7C3]/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3]"
        >
          <RefreshCw className="h-4 w-4" />
          Try again
        </button>
        <Link
          href="/user/pool-market"
          className="inline-flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-800/50 px-5 py-2.5 text-sm font-medium text-white transition-all hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to pools
        </Link>
      </div>
    </div>
  );
}
