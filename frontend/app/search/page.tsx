import { Suspense } from "react";
import type { Metadata } from "next";
import { SearchablePools } from "@/components/search/SearchablePools";
import { Skeleton } from "@/components/ui";

export const metadata: Metadata = { title: "Search Pools | PrediFi" };

/**
 * Fallback shown while the SearchablePools client bundle is loading.
 * Mirrors the skeleton rendered by SearchablePools itself so there is no
 * layout shift when the component hydrates.
 */
function SearchPageSkeleton() {
  return (
    <div className="rounded-2xl bg-[#121212] border-none min-h-[400px] p-6 space-y-4">
      <Skeleton className="h-5 w-32" />
      <Skeleton className="h-10 w-full rounded-md" />
      <div className="space-y-3">
        {Array.from({ length: 4 }).map((_, i) => (
          <div
            key={i}
            className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/50"
          >
            <div className="space-y-2">
              <Skeleton className="h-4 w-40" />
              <Skeleton className="h-3 w-24" />
            </div>
            <Skeleton className="h-6 w-16 rounded-full" />
          </div>
        ))}
      </div>
    </div>
  );
}

export default function SearchPage() {
  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8">
      <div className="mx-auto max-w-6xl space-y-6">
        {/* Page header */}
        <div className="rounded-3xl border border-white/10 bg-zinc-950/70 p-6 shadow-[0_25px_80px_rgba(0,0,0,0.25)] backdrop-blur-xl">
          <div className="space-y-3">
            <p className="text-sm uppercase tracking-[0.3em] text-[#7DE3EC]/80">
              Market search
            </p>
            <h1 className="text-3xl font-semibold text-white sm:text-4xl">
              Discover pools instantly with filtered search.
            </h1>
            <p className="max-w-2xl text-sm leading-6 text-zinc-400 sm:text-base">
              Filter active pools by name, category, or token. Sort and status
              filters are reflected in the URL so searches are bookmarkable.
            </p>
          </div>
        </div>

        {/*
          Suspense boundary required because SearchablePools calls
          useSearchParams() — a client hook that needs a boundary in the
          App Router to avoid blocking the server render.
        */}
        <Suspense fallback={<SearchPageSkeleton />}>
          <SearchablePools />
        </Suspense>
      </div>
    </div>
  );
}
