"use client";

import { cn } from "@/lib/utils";
import { User, Shield, Bell, ChevronDown } from "lucide-react";
import { useState } from "react";

export type SettingsTab = "profile" | "security" | "notifications";

interface NavItem {
  id: SettingsTab;
  label: string;
  icon: React.ReactNode;
  description: string;
}

const NAV_ITEMS: NavItem[] = [
  {
    id: "profile",
    label: "Profile",
    icon: <User className="h-4 w-4" />,
    description: "Personal info & avatar",
  },
  {
    id: "security",
    label: "Security",
    icon: <Shield className="h-4 w-4" />,
    description: "Wallet & authentication",
  },
  {
    id: "notifications",
    label: "Notifications",
    icon: <Bell className="h-4 w-4" />,
    description: "Alerts & preferences",
  },
];

interface SettingsSidebarProps {
  activeTab: SettingsTab;
  onTabChange: (tab: SettingsTab) => void;
}

export function SettingsSidebar({ activeTab, onTabChange }: SettingsSidebarProps) {
  const [mobileOpen, setMobileOpen] = useState(false);
  const activeItem = NAV_ITEMS.find((item) => item.id === activeTab)!;

  return (
    <>
      {/* Mobile dropdown */}
      <div className="lg:hidden">
        <button
          type="button"
          onClick={() => setMobileOpen((prev) => !prev)}
          className="w-full flex items-center justify-between px-4 py-3 rounded-xl border border-zinc-800 bg-zinc-900 text-white hover:border-zinc-700 transition-colors"
          aria-expanded={mobileOpen}
          aria-controls="settings-mobile-menu"
        >
          <span className="flex items-center gap-2 text-sm font-medium">
            {activeItem.icon}
            {activeItem.label}
          </span>
          <ChevronDown
            className={cn(
              "h-4 w-4 text-zinc-400 transition-transform duration-200",
              mobileOpen && "rotate-180"
            )}
          />
        </button>
        {mobileOpen && (
          <div
            id="settings-mobile-menu"
            className="mt-1 rounded-xl border border-zinc-800 bg-zinc-900 overflow-hidden"
          >
            {NAV_ITEMS.filter((item) => item.id !== activeTab).map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => {
                  onTabChange(item.id);
                  setMobileOpen(false);
                }}
                className="w-full flex items-center gap-2 px-4 py-3 text-sm text-zinc-400 hover:text-white hover:bg-zinc-800 transition-colors"
              >
                {item.icon}
                {item.label}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Desktop sidebar */}
      <nav
        aria-label="Settings navigation"
        className="hidden lg:flex flex-col gap-1 w-56 shrink-0"
      >
        {NAV_ITEMS.map((item) => {
          const isActive = item.id === activeTab;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onTabChange(item.id)}
              className={cn(
                "flex items-start gap-3 px-4 py-3 rounded-xl text-left transition-all duration-150",
                isActive
                  ? "bg-zinc-800 text-white"
                  : "text-zinc-400 hover:text-white hover:bg-zinc-800/50"
              )}
              aria-current={isActive ? "page" : undefined}
            >
              <span
                className={cn(
                  "mt-0.5 transition-colors",
                  isActive ? "text-primary" : "text-zinc-500"
                )}
              >
                {item.icon}
              </span>
              <span>
                <span className="block text-sm font-medium">{item.label}</span>
                <span className="block text-xs text-zinc-500 mt-0.5">
                  {item.description}
                </span>
              </span>
            </button>
          );
        })}
      </nav>
    </>
  );
}
