"use client";

import { useState } from "react";
import { Button, Checkbox } from "@/components/ui";

interface NotificationPrefs {
  poolResults: boolean;
  newPools: boolean;
  rewards: boolean;
  marketing: boolean;
}

const PREFS_CONFIG: { key: keyof NotificationPrefs; label: string; description: string }[] = [
  {
    key: "poolResults",
    label: "Pool results",
    description: "Get notified when a pool you joined is resolved.",
  },
  {
    key: "newPools",
    label: "New pools",
    description: "Alerts for newly created prediction pools.",
  },
  {
    key: "rewards",
    label: "Rewards & payouts",
    description: "Notifications when rewards are distributed.",
  },
  {
    key: "marketing",
    label: "Product updates",
    description: "Occasional announcements and feature releases.",
  },
];

export function NotificationPreferences() {
  const [prefs, setPrefs] = useState<NotificationPrefs>({
    poolResults: true,
    newPools: true,
    rewards: true,
    marketing: false,
  });
  const [saved, setSaved] = useState(false);

  function handleToggle(key: keyof NotificationPrefs, checked: boolean) {
    setPrefs((prev) => ({ ...prev, [key]: checked }));
    setSaved(false);
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    // TODO: wire up to API
    setSaved(true);
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      <div className="space-y-1">
        <h2 className="text-lg font-semibold text-white">Notifications</h2>
        <p className="text-xs text-zinc-500">Choose which alerts you want to receive.</p>
      </div>

      <div className="space-y-4">
        {PREFS_CONFIG.map(({ key, label, description }) => (
          <div key={key} className="flex items-start justify-between gap-4 py-3 border-b border-zinc-800 last:border-0">
            <div>
              <p className="text-sm font-medium text-white">{label}</p>
              <p className="text-xs text-zinc-500 mt-0.5">{description}</p>
            </div>
            <Checkbox
              checked={prefs[key]}
              onCheckedChange={(checked) => handleToggle(key, checked === true)}
              aria-label={label}
            />
          </div>
        ))}
      </div>

      <div className="flex items-center gap-3">
        <Button type="submit" size="medium">Save preferences</Button>
        {saved && (
          <span className="text-sm text-green-400" role="status">Saved!</span>
        )}
      </div>
    </form>
  );
}
