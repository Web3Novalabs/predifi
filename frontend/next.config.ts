import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  /* config options here */
  env: {
    NEXT_PUBLIC_ALCHEMY_API_KEY:
      "https://starknet-mainnet.g.alchemy.com/starknet/version/rpc/v0_7/dvHmwiGiA_uE22lKpZKLk4FoGlC_Xzy4",
  },
  images: {
    domains: ["starknet.id", "images.unsplash.com", "firebasestorage.googleapis.com"],
  },
};

export default nextConfig;

// /** @type {import('next').NextConfig} */
// const nextConfig = {
//   output: 'standalone',
//   env: {
//     CAIRO_BUILD_PATH: '../contracts/target/dev/'
//   },
// };

// module.exports = nextConfig;

