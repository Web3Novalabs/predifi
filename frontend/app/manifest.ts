import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "Predifi | Web3 Prediction Markets",
    short_name: "Predifi",
    description:
      "PrediFi is a decentralized prediction market protocol built on the Stellar network with Soroban smart contracts.",
    start_url: "/",
    display: "standalone",
    background_color: "#0a0a0a",
    theme_color: "#37B7C3",
    icons: [
      {
        src: "/logo.svg",
        sizes: "any",
        type: "image/svg+xml",
        purpose: "any",
      },
    ],
  };
}
