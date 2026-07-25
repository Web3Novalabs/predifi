import type { Metadata } from "next";
import { DM_Mono } from "next/font/google";
import "./globals.css";
import { SWRProvider } from "@/components/providers/SWRProvider";
import { NetworkGuardProvider } from "@/components/providers/NetworkGuardProvider";
import { ToastProvider } from "@/components/ui";

const SITE_DESCRIPTION =
  "PrediFi is a decentralized prediction market protocol built on the Stellar network with Soroban smart contracts.";

const dmMono = DM_Mono({
  subsets: ["latin"],
  weight: ["300", "400", "500"],
  variable: "--font-dm-mono",
  display: "swap",
  preload: true,
});

export const metadata: Metadata = {
  title: {
    default: "Predifi | Web3 Prediction Markets",
    template: "%s | Predifi",
  },
  description: SITE_DESCRIPTION,
  keywords: [
    "decentralized prediction",
    "predifi",
    "payment",
    "protocol",
    "automated rewards",
    "trustless",
    "Web3 payment",
    "betting",
    "crowd funding",
    "stellar",
    "prediction",
    "crypto payment",
  ],
  openGraph: {
    title: "Predifi- Decentralized prediction protocol built on the Stellar",
    description: SITE_DESCRIPTION,
    url: "https://predifi.app",
    siteName: "nevo",
    images: [
      {
        url: "https://predifi.app/logo.jpeg",
        width: 1200,
        height: 630,
        alt: "predifi - Decentralized prediction protocol built on the Stellar",
      },
    ],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Predifi - Decentralized prediction protocol built on the Stellarr",
    description: SITE_DESCRIPTION,
    images: ["https://predifi.app/logo.jpeg"],
    creator: "@nevoapp",
  },

  icons: {
    icon: [
      { url: "/Group 1.svg" },
      {
        url: "/Group 1.svg",
        sizes: "192x192",
        type: "image/svg+xml",
      },
      {
        url: "/Group 1.svg",
        sizes: "512x512",
        type: "image/svg+xml",
      },
    ],
    apple: [
      {
        url: "/Group 1.svg",
        sizes: "180x180",
        type: "image/svg+xml",
      },
    ],
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <head>
        {/* Preload critical hero images to improve LCP */}
        <link rel="preload" as="image" href="/swirl-pattern.webp" />
        <link rel="preload" as="image" href="/gradient.webp" />

        {/* Inline minimal critical CSS for hero to paint immediately */}
        <style>{`.hero-critical{min-height:calc(100vh - 40px);display:flex;flex-direction:column;align-items:center;text-align:center}`}</style>
      </head>
      <body className={`antialiased text-sm ${dmMono.variable}`}>
        <SWRProvider>
          <NetworkGuardProvider>
            <ToastProvider>{children}</ToastProvider>
          </NetworkGuardProvider>
        </SWRProvider>
      </body>
    </html>
  );
}
