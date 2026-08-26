import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Community",
  description:
    "Join the PrediFi community — connect with predictors, validators, and crypto enthusiasts building the future of decentralized prediction markets on Stellar.",
  openGraph: {
    title: "Community | PrediFi — Join the Decentralized Prediction Network",
    description:
      "Join the PrediFi community — connect with predictors, validators, and crypto enthusiasts building the future of decentralized prediction markets on Stellar.",
    url: "https://predifi.app/community",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi Community" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Community | PrediFi — Join the Decentralized Prediction Network",
    description:
      "Join the PrediFi community — connect with predictors, validators, and crypto enthusiasts building the future of decentralized prediction markets on Stellar.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function Community(){
    return(
        <h1>Community Page</h1>
    )
}