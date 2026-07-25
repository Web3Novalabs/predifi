import type { Metadata } from "next";
import { Suspense } from "react";
import dynamic from "next/dynamic";
import Navbar from "../(marketing)/components/NavBar";

export const metadata: Metadata = {
  title: "About",
  description:
    "Learn about PrediFi — a decentralized prediction protocol built on the Stellar network with Soroban smart contracts. Our mission, story, and how it works.",
  openGraph: {
    title: "About PrediFi | Decentralized Prediction Protocol on Stellar",
    description:
      "Discover how PrediFi is reshaping prediction markets with trustless, transparent, and automated outcomes on the Stellar blockchain.",
    url: "https://predifi.app/about",
    siteName: "PrediFi",
    images: [
      {
        url: "https://predifi.app/logo.jpeg",
        width: 1200,
        height: 630,
        alt: "About PrediFi",
      },
    ],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "About PrediFi | Decentralized Prediction Protocol on Stellar",
    description:
      "Discover how PrediFi is reshaping prediction markets with trustless, transparent, and automated outcomes on the Stellar blockchain.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};

// Above the fold — eagerly loaded
import Hero from "./components/Hero";

// Below the fold — lazily loaded via next/dynamic
const Mission = dynamic(() => import("./components/Mission"), {
  loading: () => (
    <div className="h-[400px] w-full animate-pulse bg-white/5" aria-hidden="true" />
  ),
});

const HowItWorks = dynamic(() => import("./components/HowItWorks"), {
  loading: () => (
    <div className="h-[500px] w-full animate-pulse bg-white/5" aria-hidden="true" />
  ),
});

const Benefits = dynamic(() => import("./components/Benefits"), {
  loading: () => (
    <div className="h-[400px] w-full animate-pulse bg-white/5" aria-hidden="true" />
  ),
});

const Footer = dynamic(() => import("../(marketing)/components/Footer"), {
  loading: () => (
    <div className="h-[120px] w-full animate-pulse bg-white/5 rounded-t-[40px]" aria-hidden="true" />
  ),
});

export default function AboutPage() {
  return (
    <div className="text-sm min-h-screen bg-[#001112]">
      <Navbar />

      <main className="w-full overflow-x-hidden">
        <Hero />

        <Suspense
          fallback={
            <div className="h-[400px] w-full animate-pulse bg-white/5" aria-hidden="true" />
          }
        >
          <Mission />
        </Suspense>

        <Suspense
          fallback={
            <div className="h-[500px] w-full animate-pulse bg-white/5" aria-hidden="true" />
          }
        >
          <HowItWorks />
        </Suspense>

        <Suspense
          fallback={
            <div className="h-[400px] w-full animate-pulse bg-white/5" aria-hidden="true" />
          }
        >
          <Benefits />
        </Suspense>

        <Suspense
          fallback={
            <div className="h-[120px] w-full animate-pulse bg-white/5 rounded-t-[40px]" aria-hidden="true" />
          }
        >
          <Footer />
        </Suspense>
      </main>
    </div>
  );
}
