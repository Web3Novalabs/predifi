import Link from "next/link";
import { ArrowLeft, Sparkles } from "lucide-react";

export interface ComingSoonProps {
  /** Feature name shown as the heading, e.g. "Validator". */
  title: string;
  /** Short explanation of what the feature will do once it ships. */
  description: string;
}

/**
 * ComingSoon — placeholder rendered by routes whose features have not shipped
 * yet. Mirrors the app's existing empty-state styling (dark surface, teal
 * accent) and offers a way back to the dashboard so visitors are never left on
 * a dead page.
 */
export function ComingSoon({ title, description }: ComingSoonProps) {
  return (
    <main
      className="relative flex min-h-[70vh] items-center justify-center bg-[#0A0A0A] px-6"
      aria-labelledby="coming-soon-heading"
    >
      <div
        aria-hidden="true"
        className="pointer-events-none absolute left-1/2 top-1/3 -translate-x-1/2 -translate-y-1/2 h-80 w-80 rounded-full bg-[#37B7C3]/10 blur-[110px]"
      />

      <div className="relative z-10 flex flex-col items-center text-center">
        <div className="mb-6 flex h-14 w-14 items-center justify-center rounded-full border border-[#37B7C3]/30 bg-[#37B7C3]/10 shadow-[0_0_24px_rgba(55,183,195,0.2)]">
          <Sparkles className="h-6 w-6 text-[#37B7C3]" />
        </div>

        <p className="mb-2 text-xs font-medium uppercase tracking-[0.3em] text-[#37B7C3]">
          Coming soon
        </p>

        <h1
          id="coming-soon-heading"
          className="mb-3 text-2xl font-semibold text-white md:text-3xl"
        >
          {title}
        </h1>

        <p className="mb-8 max-w-sm text-sm leading-relaxed text-zinc-400">
          {description}
        </p>

        <Link
          href="/user/dashboard"
          prefetch
          className="group inline-flex items-center gap-2 rounded-xl border border-[#37B7C3]/30 bg-[#37B7C3]/10 px-6 py-3 text-sm font-medium text-[#37B7C3] transition-all duration-200 hover:border-[#37B7C3]/60 hover:bg-[#37B7C3]/20 hover:gap-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3]"
        >
          <ArrowLeft className="h-4 w-4 transition-transform duration-200 group-hover:-translate-x-0.5" />
          Back to dashboard
        </Link>
      </div>
    </main>
  );
}
