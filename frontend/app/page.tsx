import type { Metadata } from "next";
import HomeContent from "@/components/home/home-content";
import HomeJsonLd from "@/components/seo/home-json-ld";

export const metadata: Metadata = {
  title: "Home",
  description:
    "Transform Predictions Into Profits! Create and participate in decentralized prediction markets across sports, finance, and pop culture.",
  openGraph: {
    title: "PrediFI - Transform Predictions Into Profits!",
    description:
      "Create and participate in decentralized prediction markets across sports, finance, and pop culture.",
    images: [
      {
        url: "/code.png",
        width: 1200,
        height: 630,
        alt: "PrediFI - Transform Predictions Into Profits!",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "PrediFI - Transform Predictions Into Profits!",
    description:
      "Create and participate in decentralized prediction markets across sports, finance, and pop culture.",
    images: ["/code.png"],
  },
  alternates: {
    canonical: "",
  },
};

export default function Home() {
  return (
    <>
      <HomeJsonLd />
      <HomeContent />
    </>
  );
}
