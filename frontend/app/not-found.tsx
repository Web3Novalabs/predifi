"use client";

import Link from 'next/link';

export default function NotFound() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-zinc-50 dark:bg-black px-4">
      <div className="text-center animate-in fade-in duration-1000">
        <h1 className="text-6xl md:text-9xl font-bold text-[#0E0E10] dark:text-zinc-100 mb-4 animate-pulse">
          404
        </h1>
        <h2 className="text-xl md:text-2xl font-semibold text-zinc-700 dark:text-zinc-300 mb-4">
          Page Not Found
        </h2>
        <p className="text-sm md:text-base text-zinc-600 dark:text-zinc-400 mb-8 max-w-md mx-auto">
          Oops! The page you&apos;re looking for doesn&apos;t exist. It might have been moved or deleted.
        </p>
        <Link
          href="/"
          className="inline-block px-6 py-3 bg-[#259BA5] dark:bg-zinc-100 text-white dark:text-black rounded-lg hover:bg-zinc-800 dark:hover:bg-zinc-200 transition-colors duration-200"
        >
          Go Home
        </Link>
      </div>
      <style jsx>{`
        @keyframes fade-in {
          from { opacity: 0; transform: translateY(20px); }
          to { opacity: 1; transform: translateY(0); }
        }
        .animate-in {
          animation: fade-in 1s ease-out;
        }
      `}</style>
    </main>
  );
}
