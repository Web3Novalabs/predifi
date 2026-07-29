"use client";

/**
 * ThemeContext — single app-wide theme instance.
 *
 * Problem (issue #1406)
 * ─────────────────────
 * When useTheme() is called in multiple components (ThemeToggle, any component
 * that reads the current theme), each creates its own:
 *   • useState instance — state is duplicated, not shared
 *   • window.matchMedia listener — multiple listeners fire on system changes
 *   • localStorage read — redundant I/O on the same key per render
 *
 * Fix: one ThemeProvider at the root mounts exactly one useTheme() call.
 * All consumers read from context — zero duplicate listeners, coordinated
 * state, single source of truth.
 *
 * Usage
 * ─────
 *   // In app/layout.tsx:
 *   <ThemeProvider>…</ThemeProvider>
 *
 *   // In any client component:
 *   const { theme, setTheme } = useThemeContext();
 */

import {
  createContext,
  useContext,
  useMemo,
  type ReactNode,
} from "react";
import { useTheme, type Theme } from "@/lib/hooks/useTheme";

// ── Types ─────────────────────────────────────────────────────────────────────

interface ThemeContextValue {
  theme: Theme;
  setTheme: (next: Theme) => void;
}

// ── Context ───────────────────────────────────────────────────────────────────

export const ThemeContext = createContext<ThemeContextValue | undefined>(
  undefined,
);
ThemeContext.displayName = "ThemeContext";

// ── Provider ──────────────────────────────────────────────────────────────────

export function ThemeProvider({ children }: { children: ReactNode }) {
  const { theme, setTheme } = useTheme();

  // Keep reference stable so consumers don't re-render when unrelated
  // parent state changes.
  const value = useMemo<ThemeContextValue>(
    () => ({ theme, setTheme }),
    [theme, setTheme],
  );

  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}

// ── Hook ──────────────────────────────────────────────────────────────────────

/**
 * useThemeContext — read the app-wide theme from context.
 *
 * Throws a descriptive error outside <ThemeProvider> so misconfiguration
 * surfaces at development time.
 */
export function useThemeContext(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (ctx === undefined) {
    throw new Error(
      "useThemeContext must be used within a <ThemeProvider>. " +
        "Add <ThemeProvider> to your root layout.",
    );
  }
  return ctx;
}

export type { ThemeContextValue };
