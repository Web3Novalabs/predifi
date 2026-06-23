import type { Metadata } from "next";
import { DM_Mono } from "next/font/google";
import "./globals.css";
import { SWRProvider } from "@/components/providers/SWRProvider";

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
  description: "Decentralized prediction protocol built on the Stellar. ",
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
    description:
      "PrediFi is a decentralized prediction protocol built on the Stellar network using Soroban smart contracts. ",
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
    description:
      "PrediFi is a decentralized prediction protocol built on the Stellar network using Soroban smart contracts.",
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
        <SWRProvider>{children}</SWRProvider>
      </body>
    </html>
  );
}
