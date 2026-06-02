import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Enable CSS minification and optimization in production
  compress: true,
  // Optimize CSS loading
  experimental: {
    optimizeCss: true,
  },
  images: {
    // Allow Next.js to optimise SVG files served from the /public directory.
    // The Content-Security-Policy header set below mitigates the XSS risk that
    // comes with serving SVGs as images (they cannot execute scripts when loaded
    // via <img> / next/image).
    dangerouslyAllowSVG: true,
    contentDispositionType: "attachment",
    contentSecurityPolicy: "default-src 'self'; script-src 'none'; sandbox;",
  },
};

export default nextConfig;
