"use client";

import { useEffect } from "react";
import Link from "next/link";
import { ArrowLeft, RefreshCw, AlertTriangle } from "lucide-react";

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    // Log the error to an error reporting service
    console.error("Application error:", error);
  }, [error]);

  const isNetworkError =
    error.message?.includes("fetch") ||
    error.message?.includes("network") ||
    error.message?.includes("Failed to fetch");

  return (
    <main
      className="relative flex min-h-screen items-center justify-center bg-[#0A0A0A] px-6 overflow-hidden"
      aria-labelledby="error-heading"
    >
      {/* Ambient background glows */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute left-1/2 top-1/4 -translate-x-1/2 -translate-y-1/2 w-[480px] h-[480px] rounded-full bg-red-500/10 blur-[120px]"
      />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute right-0 bottom-0 w-64 h-64 rounded-full bg-red-500/5 blur-[80px]"
      />

      {/* Grid overlay */}
      <svg
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 h-full w-full opacity-[0.03]"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
            <path d="M 40 0 L 0 0 0 40" fill="none" stroke="#37B7C3" strokeWidth="1" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#grid)" />
      </svg>

      {/* Content */}
      <div className="relative z-10 text-center animate-fade-in max-w-md">
        {/* Error label */}
        <p className="text-xs font-medium tracking-[0.3em] text-red-400 uppercase mb-6 flex items-center justify-center gap-2">
          <AlertTriangle className="w-4 h-4" />
          Error
        </p>

        {/* Large numeral */}
        <h1
          id="error-heading"
          className="text-[clamp(4rem,15vw,10rem)] font-bold leading-none tracking-tighter text-white/5 select-none mb-2"
          aria-hidden="true"
        >
          Oops
        </h1>

        {/* Divider */}
        <div className="mx-auto mb-6 h-px w-24 bg-gradient-to-r from-transparent via-red-400/60 to-transparent" />

        <h2 className="text-2xl md:text-3xl font-semibold text-white mb-3">
          {isNetworkError ? "Connection Error" : "Something Went Wrong"}
        </h2>
        <p className="text-sm text-zinc-400 max-w-sm mx-auto mb-6 leading-relaxed">
          {isNetworkError
            ? "We couldn't connect to the server. Please check your internet connection and try again."
            : "An unexpected error occurred. Please try again or return to the homepage."}
        </p>

        {/* Error message for development */}
        {process.env.NODE_ENV === "development" && error.message && (
          <div className="mb-6 p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-left">
            <p className="text-xs text-red-400 font-mono break-all">{error.message}</p>
            {error.digest && (
              <p className="text-xs text-zinc-500 mt-2 font-mono">
                Digest: {error.digest}
              </p>
            )}
          </div>
        )}

        {/* CTAs */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-3">
          <button
            onClick={reset}
            className="group inline-flex items-center gap-2 rounded-xl border border-[#37B7C3]/30 bg-[#37B7C3]/10 px-6 py-3 text-sm font-medium text-[#37B7C3] transition-all duration-200 hover:bg-[#37B7C3]/20 hover:border-[#37B7C3]/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3]"
          >
            <RefreshCw className="w-4 h-4" />
            Try Again
          </button>
          <Link
            href="/"
            prefetch
            className="group inline-flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-800/50 px-6 py-3 text-sm font-medium text-white transition-all duration-200 hover:bg-zinc-800 hover:gap-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500"
          >
            <ArrowLeft className="w-4 h-4 transition-transform duration-200 group-hover:-translate-x-0.5" />
            Go Home
          </Link>
        </div>
      </div>
    </main>
  );
}