import type { Metadata } from "next";

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

export default function Page(){
    return(
        <h1>Dashboard Page</h1>
    )
}