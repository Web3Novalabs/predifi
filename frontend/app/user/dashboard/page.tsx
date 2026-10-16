import { Suspense } from "react";
import type { Metadata } from "next";
import { PredictionHistoryClient } from "@/app/user/my-profile/PredictionHistoryClient";
import { Skeleton } from "@/components/ui";

export const metadata: Metadata = {
  title: "My Dashboard",
  description:
    "Your PrediFi dashboard — track your active predictions, monitor staked balances, claim rewards, and view performance metrics all in one place.",
  openGraph: {
    title: "My Dashboard | PrediFi",
    description:
      "Your PrediFi dashboard — track your active predictions, monitor staked balances, claim rewards, and view performance metrics all in one place.",
    url: "https://predifi.app/user/dashboard",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi Dashboard" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "My Dashboard | PrediFi",
    description:
      "Your PrediFi dashboard — track your active predictions, monitor staked balances, claim rewards, and view performance metrics all in one place.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

/**
 * User dashboard page — prediction history, win/loss stats, earnings,
 * active positions, claim status, and performance charts.
 *
 * Uses the same `PredictionHistoryClient` as `/user/my-profile` so that
 * both entry points show the full profile view (#1927).
 */
export default function Page() {
  return (
    <Suspense
      fallback={
        <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8 space-y-8">
          <div className="flex items-center gap-5">
            <Skeleton className="h-16 w-16 rounded-full" />
            <div className="space-y-2">
              <Skeleton className="h-6 w-32" />
              <Skeleton className="h-3 w-48" />
            </div>
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-5 gap-4">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="rounded-xl border border-zinc-800 bg-zinc-900 p-4 space-y-2">
                <Skeleton className="h-3 w-20" />
                <Skeleton className="h-5 w-16" />
              </div>
            ))}
          </div>
          <Skeleton className="h-[280px] w-full rounded-xl" />
        </div>
      }
    >
      <PredictionHistoryClient />
    </Suspense>
  );
}
