import type { Metadata } from "next";
import Navbar from "../(marketing)/components/NavBar";

export const metadata: Metadata = {
  title: "Join the Waitlist",
  description:
    "Join the PrediFi waitlist and be among the first to access the next generation of decentralized prediction markets built on the Stellar blockchain.",
  openGraph: {
    title:
      "Join the Waitlist | PrediFi — Early Access to Web3 Prediction Markets",
    description:
      "Join the PrediFi waitlist and be among the first to access the next generation of decentralized prediction markets built on the Stellar blockchain.",
    url: "https://predifi.app/waitlist",
    siteName: "PrediFi",
    images: [
      {
        url: "https://predifi.app/logo.jpeg",
        width: 1200,
        height: 630,
        alt: "PrediFi Waitlist",
      },
    ],
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title:
      "Join the Waitlist | PrediFi — Early Access to Web3 Prediction Markets",
    description:
      "Join the PrediFi waitlist and be among the first to access the next generation of decentralized prediction markets built on the Stellar blockchain.",
    images: ["https://predifi.app/logo.jpeg"],
  },
};
import Footer from "../(marketing)/components/Footer";
import WaitlistForm from "./components/WaitlistForm";

export default function WaitlistPage() {
  return (
    <div className="text-sm min-h-screen bg-[#001112] flex flex-col">
      <Navbar />

      <main className="w-full overflow-x-hidden flex-1">
        <WaitlistForm />
      </main>

      <Footer />
    </div>
  );
}
