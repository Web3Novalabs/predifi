import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  async headers() {
    return [
      {
        // Build-time hashed assets — safe to cache forever
        source: "/_next/static/:path*",
        headers: [
          {
            key: "Cache-Control",
            value: "public, max-age=31536000, immutable",
          },
        ],
      },
      {
        // Public folder assets (images, icons, manifests, fonts)
        source: "/(:path*\.(?:ico|png|jpg|jpeg|svg|webp|woff|woff2|ttf|otf|json|txt|xml))",
        headers: [
          {
            key: "Cache-Control",
            value: "public, max-age=86400, must-revalidate",
          },
        ],
      },
    ];
  },
  // Enable CSS minification and optimization in production
  compress: true,
  experimental: {
    // Optimize CSS loading
    optimizeCss: true,
    // Rewrites named imports from these packages into per-module imports
    // (e.g. `import { ChevronDown } from "lucide-react"` becomes an import of
    // just that icon's module) so webpack's bundler sees exactly what's used
    // instead of having to tree-shake through each package's barrel file.
    // Both are among Next.js's own documented candidates for this flag, and
    // are the two largest UI dependencies in package.json (#1939).
    optimizePackageImports: ["lucide-react", "recharts"],
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
