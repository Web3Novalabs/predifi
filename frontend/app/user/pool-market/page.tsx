import type { Metadata } from "next";


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

import { OddsCalculator } from "@/components/ui/odds-calculator";

export default function Page() {
  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8">
      <div className="mx-auto max-w-3xl space-y-6">
        <div className="space-y-1">
          <h1 className="text-3xl font-bold text-white">Pool Market</h1>
          <p className="text-zinc-400 text-sm">Browse pools and calculate your potential returns.</p>
        </div>
        <OddsCalculator />
      </div>
    </div>
  );
}
