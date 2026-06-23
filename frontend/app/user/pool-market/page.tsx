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
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi Pool Market" }],
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

export default function Page(){
    return(
        <h1>Pool Market Page</h1>
    )
}