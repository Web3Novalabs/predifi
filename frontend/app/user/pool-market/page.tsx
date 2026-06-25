import type { Metadata } from "next";
import Link from "next/link";
import { Plus } from "lucide-react";

export const metadata: Metadata = {
  title: "Pool Market",
  description:
    "Explore all prediction pools on PrediFi — browse open and upcoming markets, compare odds, and stake XLM on outcomes powered by Soroban smart contracts.",
  openGraph: {
    title: "Pool Market | PrediFi — Browse All Prediction Pools",
    description:
      "Explore all prediction pools on PrediFi — browse open and upcoming markets, compare odds, and stake XLM on outcomes powered by Soroban smart contracts.",
    url: "https://predifi.app/user/pool-market",
    siteName: "PrediFi",
    images: [
      {
        url: "https://predifi.app/logo.jpeg",
        width: 1200,
        height: 630,
        alt: "PrediFi Pool Market",
      },
    ],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Pool Market | PrediFi — Browse All Prediction Pools",
    description:
      "Explore all prediction pools on PrediFi — browse open and upcoming markets, compare odds, and stake XLM on outcomes powered by Soroban smart contracts.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function PoolMarketPage() {
  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8">
      <div className="mx-auto max-w-5xl space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between gap-4 flex-wrap">
          <div className="space-y-1">
            <h1 className="text-3xl font-bold text-white">Pool Market</h1>
            <p className="text-zinc-400 text-sm">
              Browse active prediction pools and stake on outcomes.
            </p>
          </div>

          {/* Create Pool CTA */}
          <Link
            href="/user/pool-market/create"
            className="inline-flex items-center gap-2 rounded-md bg-[#37B7C3] px-4 py-2 text-sm font-semibold text-black transition-all duration-200 hover:bg-[#2aa0ac] hover:scale-[1.02] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3] focus-visible:ring-offset-2 focus-visible:ring-offset-[#0A0A0A]"
            aria-label="Create a new prediction pool"
          >
            <Plus className="h-4 w-4" aria-hidden="true" />
            Create Pool
          </Link>
        </div>

        {/* Placeholder content — pools list to be wired up */}
        <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-8 text-center text-zinc-500 text-sm">
          Pool listings coming soon.
        </div>
      </div>
    </div>
  );
}
