import type { Metadata } from "next";
import { DM_Sans, Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { Anton } from "next/font/google";


const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

const anton = Anton({
  weight: ["400"], // Anton only has one weight
  subsets: ["latin"],
  display: "swap",
  variable: "--font-anton",
});

const dmSans = DM_Sans({
  subsets: ["latin"],
  weight: ["400", "500", "700", "900"], // Choose the weights you need
  display: "swap",
  variable: "--font-dmsans",
});

export const metadata: Metadata = {
  title: "Predifi",
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
      <body
        className={`bg-no-repeat bg-fixed bg h-full bg-cover py-7 ${dmSans.variable} ${anton.variable} ${geistSans.variable} ${geistMono.variable} antialiased font-dmsans`}
        suppressHydrationWarning={true}
      >
        <main className="mt-28 ">{children}</main>
      </body>
    </html>
  );
}
