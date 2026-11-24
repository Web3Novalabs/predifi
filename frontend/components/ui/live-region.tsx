"use client";

/**
 * Screen-reader announcements for dynamic content (#1389).
 *
 * Prediction placement, claim confirmations, and odds updates all change the
 * page without moving focus, so screen-reader users get no notification. This
 * provider owns two ARIA live regions — one polite, one assertive — and lets
 * any component push a message into them.
 *
 * ```tsx
 * const announce = useAnnounce();
 * announce("Prediction placed on Yes for 50 XLM");
 * announce("Transaction failed", "assertive");
 * ```
 */
import {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useMemo,
  useState,
} from "react";

type Politeness = "polite" | "assertive";

type AnnounceFn = (message: string, politeness?: Politeness) => void;

const AnnouncerContext = createContext<AnnounceFn | null>(null);

export function LiveRegionProvider({ children }: { children: ReactNode }) {
  const [polite, setPolite] = useState("");
  const [assertive, setAssertive] = useState("");

  const announce = useCallback<AnnounceFn>((message, politeness = "polite") => {
    const setter = politeness === "assertive" ? setAssertive : setPolite;
    // Clear first so repeating the same message still triggers an announcement.
    setter("");
    window.setTimeout(() => setter(message), 50);
  }, []);

  const value = useMemo(() => announce, [announce]);

  return (
    <AnnouncerContext.Provider value={value}>
      {children}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
      >
        {polite}
      </div>
      <div
        role="alert"
        aria-live="assertive"
        aria-atomic="true"
        className="sr-only"
      >
        {assertive}
      </div>
    </AnnouncerContext.Provider>
  );
}

/**
 * Announce a message to screen readers.
 *
 * Falls back to a no-op outside a `LiveRegionProvider` so components stay
 * usable in isolation (tests, Storybook) without crashing.
 */
export function useAnnounce(): AnnounceFn {
  const context = useContext(AnnouncerContext);
  const noop = useCallback<AnnounceFn>(() => {}, []);
  return context ?? noop;
}
