"use client";

import Link from "next/link";
import { ArrowLeft } from "lucide-react";

export default function NotFound() {
  return (
    <main
      className="relative flex min-h-screen items-center justify-center bg-[#0A0A0A] px-6 overflow-hidden"
      aria-labelledby="not-found-heading"
    >
      {/* Ambient background glows */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute left-1/2 top-1/4 -translate-x-1/2 -translate-y-1/2 w-[480px] h-[480px] rounded-full bg-[#37B7C3]/10 blur-[120px]"
      />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute right-0 bottom-0 w-64 h-64 rounded-full bg-[#37B7C3]/5 blur-[80px]"
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
      <div className="relative z-10 text-center animate-fade-in">
        {/* 404 label */}
        <p className="text-xs font-medium tracking-[0.3em] text-[#37B7C3] uppercase mb-6">
          Error 404
        </p>

        {/* Large numeral */}
        <h1
          id="not-found-heading"
          className="text-[clamp(6rem,20vw,14rem)] font-bold leading-none tracking-tighter text-white/5 select-none mb-2"
          aria-hidden="true"
        >
          404
        </h1>

        {/* Divider */}
        <div className="mx-auto mb-6 h-px w-24 bg-gradient-to-r from-transparent via-[#37B7C3]/60 to-transparent" />

        <h2 className="text-2xl md:text-3xl font-semibold text-white mb-3">
          Page Not Found
        </h2>
        <p className="text-sm text-zinc-400 max-w-sm mx-auto mb-10 leading-relaxed">
          The page you&apos;re looking for doesn&apos;t exist or has been moved.
          Head back and keep predicting.
        </p>

        {/* CTA */}
        <Link
          href="/"
          prefetch
          className="group inline-flex items-center gap-2 rounded-xl border border-[#37B7C3]/30 bg-[#37B7C3]/10 px-6 py-3 text-sm font-medium text-[#37B7C3] transition-all duration-200 hover:bg-[#37B7C3]/20 hover:border-[#37B7C3]/60 hover:gap-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3]"
        >
          <ArrowLeft className="w-4 h-4 transition-transform duration-200 group-hover:-translate-x-0.5" />
          Go Home
        </Link>
      </div>
    </main>
  );
}
