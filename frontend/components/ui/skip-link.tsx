import { cn } from "@/lib/utils";

interface SkipLinkProps {
  /** Id of the landmark to jump to, without the `#`. */
  targetId?: string;
  children?: string;
  className?: string;
}

/**
 * "Skip to main content" bypass link (#1389, WCAG 2.1 SC 2.4.1 Bypass Blocks).
 *
 * Rendered first in the DOM so it is the very first tab stop. It stays hidden
 * until focused, then appears pinned to the top-left. Place the matching
 * `id` on the page's `<main>` element.
 */
export function SkipLink({
  targetId = "main-content",
  children = "Skip to main content",
  className,
}: SkipLinkProps) {
  return (
    <a
      href={`#${targetId}`}
      className={cn(
        "sr-only focus:not-sr-only",
        "focus:fixed focus:left-4 focus:top-4 focus:z-[100]",
        "focus:rounded-md focus:bg-zinc-900 focus:px-4 focus:py-2",
        "focus:text-sm focus:text-white focus:shadow-lg",
        "focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2",
        className,
      )}
    >
      {children}
    </a>
  );
}
