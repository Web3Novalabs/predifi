import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Benefits",
  description:
    "Explore the benefits of using PrediFi — earn rewards, access transparent markets, and participate in trustless prediction pools on the Stellar blockchain.",
  openGraph: {
    title: "Benefits | PrediFi — Earn & Win on Decentralized Predictions",
    description:
      "Explore the benefits of using PrediFi — earn rewards, access transparent markets, and participate in trustless prediction pools on the Stellar blockchain.",
    url: "https://predifi.app/benefits",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi Benefits" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Benefits | PrediFi — Earn & Win on Decentralized Predictions",
    description:
      "Explore the benefits of using PrediFi — earn rewards, access transparent markets, and participate in trustless prediction pools on the Stellar blockchain.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function Benefits(){
    return(
        <h1>Benefits Page</h1>
    )
}