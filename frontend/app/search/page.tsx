import type { Metadata } from "next";
import { SearchablePools } from "@/components/search/SearchablePools";

export const metadata: Metadata = { title: "Search Pools | PrediFi" };

export default function SearchPage() {
  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8">
      <div className="mx-auto max-w-6xl space-y-6">
        <div className="rounded-3xl border border-white/10 bg-zinc-950/70 p-6 shadow-[0_25px_80px_rgba(0,0,0,0.25)] backdrop-blur-xl">
          <div className="space-y-3">
            <p className="text-sm uppercase tracking-[0.3em] text-[#7DE3EC]/80">
              Market search
            </p>
            <h1 className="text-3xl font-semibold text-white sm:text-4xl">
              Discover pools instantly with filtered search.
            </h1>
            <p className="max-w-2xl text-sm leading-6 text-zinc-400 sm:text-base">
              Use the search bar to filter active pools by name, category, or token. Results update in real time after you stop typing.
            </p>
          </div>
        </div>

        <SearchablePools />
      </div>
    </div>
  );
}
