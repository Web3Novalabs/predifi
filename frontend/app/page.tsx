/**
 * Home (marketing) page
 *
 * Performance strategy — above vs. below the fold:
 *
 * Above the fold (eagerly imported):
 *   - NavBar      — always visible; must render immediately
 *   - HeroSection — first thing the user sees; no lazy loading
 *
 * Below the fold (lazily imported via next/dynamic):
 *   - PredictionProtocol, Features, InstinctsToSignals, FAQ, Footer
 *
 * `next/dynamic` is Next.js's built-in wrapper around React.lazy + Suspense
 * that works correctly in both Server and Client Component trees. Using
 * React.lazy directly in a Server Component is not supported by the App Router,
 * so next/dynamic is the idiomatic equivalent here.
 *
 * Each dynamic import is given a lightweight skeleton fallback so the page
 * doesn't shift when the component loads in.
 */

import dynamic from "next/dynamic";
import { Suspense } from "react";
import Image from "next/image";
import NavBar from "./(marketing)/components/NavBar";
import HeroSection from "./(marketing)/components/HeroSection";

// ---------------------------------------------------------------------------
// Below-the-fold components — loaded lazily after the initial paint
// ---------------------------------------------------------------------------

/**
 * PredictionProtocol sits just below the hero stats bar.
 * It is a "use client" component (uses useState/useRef for the tab switcher),
 * so ssr:false avoids a hydration mismatch while still deferring its JS bundle.
 */
const PredictionProtocol = dynamic(
  () => import("./(marketing)/components/PredictionProtocol"),
  {
    loading: () => (
      <div
        className="h-[500px] w-full animate-pulse bg-white/5 rounded-2xl"
        aria-hidden="true"
      />
    ),
  }
);

/**
 * Features — three feature cards with images; well below the fold.
 * Pure server-renderable component, so ssr:true (default) is fine.
 */
const Features = dynamic(
  () => import("./(marketing)/components/Features"),
  {
    loading: () => (
      <div
        className="h-[600px] w-full animate-pulse bg-white/5 rounded-2xl"
        aria-hidden="true"
      />
    ),
  }
);

/**
 * InstinctsToSignals — stats/feature grid; well below the fold.
 */
const InstinctsToSignals = dynamic(
  () => import("./(marketing)/components/InstinctsToSignals"),
  {
    loading: () => (
      <div
        className="h-[300px] w-full animate-pulse bg-white/5 rounded-2xl"
        aria-hidden="true"
      />
    ),
  }
);

/**
 * FAQ — accordion; "use client" component (uses useState for open/close).
 * Deferred with ssr:false to keep the initial bundle lean.
 */
const FAQ = dynamic(
  () => import("./(marketing)/components/FAQ"),
  {
    loading: () => (
      <div
        className="h-[400px] w-full animate-pulse bg-white/5 rounded-2xl"
        aria-hidden="true"
      />
    ),
  }
);

/**
 * Footer — bottom of page; no interactivity, but deferred to prioritise
 * above-the-fold content in the initial JS bundle.
 */
const Footer = dynamic(
  () => import("./(marketing)/components/Footer"),
  {
    loading: () => (
      <div
        className="h-[120px] w-full animate-pulse bg-white/5 rounded-t-[40px]"
        aria-hidden="true"
      />
    ),
  }
);

// ---------------------------------------------------------------------------

export default function Home() {
  return (
    <div className="text-sm min-h-screen bg-[#001112]">
      <main className="w-screen overflow-x-hidden">
        {/* Above the fold — eagerly loaded */}
        <NavBar />
        <HeroSection />

        {/* Below the fold — lazily loaded; each wrapped in Suspense so the
            rest of the page can stream in independently */}
        <div className="relative space-y-10 lg:space-y-[150px] pt-[80px] lg:pt-[180px]">
          {/* Decorative background gradient — loaded eagerly as it is above the fold */}
          <Image
            src="/gradient.webp"
            alt=""
            aria-hidden="true"
            fill
            className="absolute top-0 left-0 w-full pointer-events-none z-0 object-cover"
            priority
            loading="eager"
            fetchPriority="high"
          />
          <Suspense
            fallback={
              <div
                className="h-[500px] w-full animate-pulse bg-white/5 rounded-2xl"
                aria-hidden="true"
              />
            }
          >
            <PredictionProtocol />
          </Suspense>

          <Suspense
            fallback={
              <div
                className="h-[600px] w-full animate-pulse bg-white/5 rounded-2xl"
                aria-hidden="true"
              />
            }
          >
            <Features />
          </Suspense>

          <Suspense
            fallback={
              <div
                className="h-[300px] w-full animate-pulse bg-white/5 rounded-2xl"
                aria-hidden="true"
              />
            }
          >
            <InstinctsToSignals />
          </Suspense>
        </div>

        <Suspense
          fallback={
            <div
              className="h-[400px] w-full animate-pulse bg-white/5 rounded-2xl"
              aria-hidden="true"
            />
          }
        >
          <FAQ />
        </Suspense>

        <Suspense
          fallback={
            <div
              className="h-[120px] w-full animate-pulse bg-white/5 rounded-t-[40px]"
              aria-hidden="true"
            />
          }
        >
          <Footer />
        </Suspense>
      </main>
    </div>
  );
}
