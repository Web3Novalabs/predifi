import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Live Market",
  description:
    "Browse and join live prediction markets on PrediFi — real-time pools with live odds, active participants, and instant stake placement on the Stellar network.",
  openGraph: {
    title: "Live Market | PrediFi — Real-Time Prediction Pools",
    description:
      "Browse and join live prediction markets on PrediFi — real-time pools with live odds, active participants, and instant stake placement on the Stellar network.",
    url: "https://predifi.app/user/live-market",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi Live Market" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Live Market | PrediFi — Real-Time Prediction Pools",
    description:
      "Browse and join live prediction markets on PrediFi — real-time pools with live odds, active participants, and instant stake placement on the Stellar network.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function Page(){
    return(
        <h1>Live market Page</h1>
    )
}