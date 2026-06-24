import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Settings",
  description:
    "Manage your PrediFi account settings — update your profile, configure security options, and set your notification preferences.",
  openGraph: {
    title: "Settings | PrediFi",
    description:
      "Manage your PrediFi account settings — update your profile, configure security options, and set your notification preferences.",
    url: "https://predifi.app/settings",
    siteName: "PrediFi",
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "PrediFi Settings" }],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Settings | PrediFi",
    description:
      "Manage your PrediFi account settings — update your profile, configure security options, and set your notification preferences.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

export default function SettingsLayout({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}
