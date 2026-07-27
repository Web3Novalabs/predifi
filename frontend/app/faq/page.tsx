import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "FAQ",
  description:
    "Frequently asked questions about PrediFi — how prediction pools work, staking, rewards, validation, and getting started on the Stellar network.",
  openGraph: {
    title: "FAQ | PrediFi — Your Questions Answered",
    description:
      "Frequently asked questions about PrediFi — how prediction pools work, staking, rewards, validation, and getting started on the Stellar network.",
    url: "https://predifi.app/faq",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi FAQ" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "FAQ | PrediFi — Your Questions Answered",
    description:
      "Frequently asked questions about PrediFi — how prediction pools work, staking, rewards, validation, and getting started on the Stellar network.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function Faq(){
    return(
        <h1>Faq Page</h1>
    )
}