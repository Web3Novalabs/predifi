"use client";

import { Sun, Moon, Monitor } from "lucide-react";
import { useTheme, type Theme } from "@/lib/hooks/useTheme";
import { cn } from "@/lib/utils";

const CYCLE: Theme[] = ["dark", "light", "system"];

const ICONS: Record<Theme, React.ReactNode> = {
  dark: <Moon className="w-4 h-4" />,
  light: <Sun className="w-4 h-4" />,
  system: <Monitor className="w-4 h-4" />,
};

const LABELS: Record<Theme, string> = {
  dark: "Dark",
  light: "Light",
  system: "System",
};

interface ThemeToggleProps {
  className?: string;
}

export function ThemeToggle({ className }: ThemeToggleProps) {
  const { theme, setTheme } = useTheme();

  function handleClick() {
    const next = CYCLE[(CYCLE.indexOf(theme) + 1) % CYCLE.length];
    setTheme(next);
  }

  return (
    <button
      type="button"
      onClick={handleClick}
      aria-label={`Switch theme, current: ${LABELS[theme]}`}
      className={cn(
        "flex items-center gap-1.5 px-3 py-2 rounded-lg border border-white/10 bg-white/[0.04]",
        "text-xs text-zinc-400 hover:text-white hover:border-white/20 transition-colors",
        className,
      )}
    >
      {ICONS[theme]}
      <span>{LABELS[theme]}</span>
    </button>
  );
}
