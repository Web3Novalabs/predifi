import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Validator",
  description:
    "Act as a validator on PrediFi — review prediction outcomes, vote on disputed results, and earn validator rewards for keeping markets honest and trustless.",
  openGraph: {
    title: "Validator | PrediFi — Earn Rewards by Validating Outcomes",
    description:
      "Act as a validator on PrediFi — review prediction outcomes, vote on disputed results, and earn validator rewards for keeping markets honest and trustless.",
    url: "https://predifi.app/user/validator",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi Validator" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Validator | PrediFi — Earn Rewards by Validating Outcomes",
    description:
      "Act as a validator on PrediFi — review prediction outcomes, vote on disputed results, and earn validator rewards for keeping markets honest and trustless.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function Page(){
    return(
        <h1>Validator Page</h1>
    )
}