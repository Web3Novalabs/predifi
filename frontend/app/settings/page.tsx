"use client";

import { useState } from "react";
import { SettingsSidebar, type SettingsTab } from "@/components/settings/SettingsSidebar";
import { ProfileForm } from "@/components/settings/ProfileForm";
import { SecuritySettings } from "@/components/settings/SecuritySettings";
import { NotificationPreferences } from "@/components/settings/NotificationPreferences";

const PANEL_MAP: Record<SettingsTab, React.ReactNode> = {
  profile: <ProfileForm />,
  security: <SecuritySettings />,
  notifications: <NotificationPreferences />,
};

export default function SettingsPage() {
  const [activeTab, setActiveTab] = useState<SettingsTab>("profile");

  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8">
      <div className="mx-auto max-w-5xl space-y-6">
        {/* Header */}
        <div className="space-y-1">
          <h1 className="text-3xl font-bold text-white">Settings</h1>
          <p className="text-zinc-400 text-sm">
            Manage your profile, security, and notification preferences.
          </p>
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
