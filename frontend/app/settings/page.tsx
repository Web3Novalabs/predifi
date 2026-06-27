"use client";

import { useEffect, useState } from "react";
import dynamic from "next/dynamic";
import { SettingsSidebar, type SettingsTab } from "@/components/settings/SettingsSidebar";

// ProfileForm — below-the-fold, lazily loaded
const ProfileForm = dynamic(
  () => import("@/components/settings/ProfileForm").then((mod) => mod.ProfileForm),
  {
    loading: () => (
      <div className="h-[300px] w-full animate-pulse bg-zinc-800/50 rounded-xl" aria-hidden="true" />
    ),
  },
);

// SecuritySettings — below-the-fold, lazily loaded
const SecuritySettings = dynamic(
  () => import("@/components/settings/SecuritySettings").then((mod) => mod.SecuritySettings),
  {
    loading: () => (
      <div className="h-[300px] w-full animate-pulse bg-zinc-800/50 rounded-xl" aria-hidden="true" />
    ),
  },
);

// NotificationPreferences — below-the-fold, lazily loaded
const NotificationPreferences = dynamic(
  () => import("@/components/settings/NotificationPreferences").then((mod) => mod.NotificationPreferences),
  {
    loading: () => (
      <div className="h-[300px] w-full animate-pulse bg-zinc-800/50 rounded-xl" aria-hidden="true" />
    ),
  },
);

const PANEL_MAP: Record<SettingsTab, React.ReactNode> = {
  profile: <ProfileForm />,
  security: <SecuritySettings />,
  notifications: <NotificationPreferences />,
};

export default function SettingsPage() {
  const [activeTab, setActiveTab] = useState<SettingsTab>("profile");
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    if (typeof window === "undefined") return "dark";
    const stored = window.localStorage.getItem("theme");
    return (stored ?? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")) as "light" | "dark";
  });

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    window.localStorage.setItem("theme", theme);
  }, [theme]);

  return (
    <div className="min-h-screen bg-background p-6 lg:p-8 text-foreground">
      <div className="mx-auto max-w-5xl space-y-6">
        {/* Header */}
        <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
          <div className="space-y-1">
            <h1 className="text-3xl font-bold">Settings</h1>
            <p className="text-sm text-foreground/70">
              Manage your profile, security, and notification preferences.
            </p>
          </div>
          <button
            type="button"
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
            className="rounded-full border border-border bg-card px-4 py-2 text-sm transition hover:bg-secondary"
          >
            Switch to {theme === "dark" ? "light" : "dark"} mode
          </button>
        </div>

        {/* Layout */}
        <div className="flex flex-col lg:flex-row gap-6 lg:gap-8">
          <SettingsSidebar activeTab={activeTab} onTabChange={setActiveTab} />
          <main className="flex-1 min-w-0">
            <div
              key={activeTab}
              className="animate-fade-in rounded-xl border border-zinc-800 bg-zinc-900 p-6"
            >
              {PANEL_MAP[activeTab]}
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
