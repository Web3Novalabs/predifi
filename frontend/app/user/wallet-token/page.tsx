import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Wallet & Tokens",
  description:
    "Manage your PrediFi wallet and token balances — view your XLM holdings, stake tokens, track transaction history, and connect your Stellar wallet.",
  openGraph: {
    title: "Wallet & Tokens | PrediFi — Manage Your Stellar Assets",
    description:
      "Manage your PrediFi wallet and token balances — view your XLM holdings, stake tokens, track transaction history, and connect your Stellar wallet.",
    url: "https://predifi.app/user/wallet-token",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi Wallet & Tokens" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Wallet & Tokens | PrediFi — Manage Your Stellar Assets",
    description:
      "Manage your PrediFi wallet and token balances — view your XLM holdings, stake tokens, track transaction history, and connect your Stellar wallet.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function Page(){
    return(
        <h1>Wallet-Token Page</h1>
    )
}