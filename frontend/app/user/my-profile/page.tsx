import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "My Profile",
  description:
    "View and manage your PrediFi profile — update your wallet address, track your referral code, and review your prediction history and earned rewards.",
  openGraph: {
    title: "My Profile | PrediFi",
    description:
      "View and manage your PrediFi profile — update your wallet address, track your referral code, and review your prediction history and earned rewards.",
    url: "https://predifi.app/user/my-profile",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi My Profile" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "My Profile | PrediFi",
    description:
      "View and manage your PrediFi profile — update your wallet address, track your referral code, and review your prediction history and earned rewards.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function Page(){
    return(
        <h1>My Profile Page</h1>
    )
}