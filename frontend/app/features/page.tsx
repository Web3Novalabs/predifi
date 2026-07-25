import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Features",
  description:
    "Discover PrediFi's powerful features — decentralized prediction pools, automated payouts via Soroban smart contracts, real-time odds, and validator incentives.",
  openGraph: {
    title: "Features | PrediFi — Powerful Web3 Prediction Tools",
    description:
      "Discover PrediFi's powerful features — decentralized prediction pools, automated payouts via Soroban smart contracts, real-time odds, and validator incentives.",
    url: "https://predifi.app/features",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi Features" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Features | PrediFi — Powerful Web3 Prediction Tools",
    description:
      "Discover PrediFi's powerful features — decentralized prediction pools, automated payouts via Soroban smart contracts, real-time odds, and validator incentives.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function Features(){
    return(
        <h1>Features Page</h1>
    )
}