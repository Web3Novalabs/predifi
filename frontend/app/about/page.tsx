import type { Metadata } from "next";
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
    images: [{ url: "https://predifi.app/logo.jpeg", width: 1200, height: 630, alt: "About PrediFi" }],
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
import Footer from "../(marketing)/components/Footer";
import Hero from "./components/Hero";
import Mission from "./components/Mission";
import HowItWorks from "./components/HowItWorks";
import Benefits from "./components/Benefits";

export default function AboutPage() {
  return (
    <div className="text-sm min-h-screen bg-[#001112]">
      <Navbar />

      <main className="w-screen overflow-x-hidden">
        <Hero />
        <Mission />
        <HowItWorks />
        <Benefits />
        <Footer />
      </main>
    </div>
  );
}