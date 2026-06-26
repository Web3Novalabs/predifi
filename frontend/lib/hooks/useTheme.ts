"use client";

import { useState, useCallback } from "react";

export type Theme = "dark" | "light" | "system";

const STORAGE_KEY = "predifi:theme";

function applyTheme(theme: Theme) {
  if (typeof document === "undefined") return;
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const resolved = theme === "system" ? (prefersDark ? "dark" : "light") : theme;
  document.documentElement.setAttribute("data-theme", resolved);
}

function readStored(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "dark" || stored === "light" || stored === "system") return stored;
  } catch {
    // ignore
  }
  return "dark";
}

export function useTheme() {
  const [theme, setThemeState] = useState<Theme>(() => {
    if (typeof window === "undefined") return "dark";
    const stored = readStored();
    applyTheme(stored);
    return stored;
  });

  const setTheme = useCallback((next: Theme) => {
    setThemeState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // ignore
    }
    applyTheme(next);
  }, []);

  return { theme, setTheme };
}
