import { ElementType, ReactNode } from "react";
import { cn } from "@/lib/utils";

interface VisuallyHiddenProps {
  children: ReactNode;
  /** Render as a different element when the context needs it (e.g. `span` inside a button). */
  as?: ElementType;
  className?: string;
  /**
   * When true the content becomes visible on keyboard focus — the pattern used
   * by skip links, which must be discoverable by keyboard but not shown to
   * pointer users. (WCAG 2.1 SC 2.4.1)
   */
  focusable?: boolean;
}

/**
 * Content available to screen readers but not shown visually (#1389).
 *
 * Use for context that sighted users get from layout or iconography — table
 * column meanings, "opens in a new tab", the target of an icon-only button.
 */
export function VisuallyHidden({
  children,
  as: Component = "span",
  className,
  focusable = false,
}: VisuallyHiddenProps) {
  return (
    <Component
      className={cn("sr-only", focusable && "focus:not-sr-only", className)}
    >
      {children}
    </Component>
  );
}
