"use client";

import { useState } from "react";
import { Bell } from "lucide-react";
import { cn } from "@/lib/utils";
import { formatUtcDateTime } from "@/lib/date";
import { useNotifications } from "@/lib/hooks/useNotifications";
import type { Notification, NotificationType } from "@/lib/api/notifications";

const TYPE_LABEL: Record<NotificationType, string> = {
  pool_ending_soon: "Ending soon",
  pool_resolved: "Resolved",
  claim_expiring: "Claim expiring",
  new_pool_match: "New pool",
};

const TYPE_STYLES: Record<NotificationType, string> = {
  pool_ending_soon: "bg-yellow-500/20 text-yellow-400",
  pool_resolved: "bg-emerald-500/20 text-emerald-400",
  claim_expiring: "bg-red-500/20 text-red-400",
  new_pool_match: "bg-[#37B7C3]/20 text-[#7DE3EC]",
};

interface NotificationBellProps {
  /** Connected wallet address. Renders nothing when absent. */
  address: string | undefined;
}

export function NotificationBell({ address }: NotificationBellProps) {
  const [open, setOpen] = useState(false);
  const { notifications, unreadCount, isLoading, markRead } = useNotifications(address);

  if (!address) return null;

  async function handleOpen() {
    setOpen((prev) => !prev);
  }

  async function handleNotificationClick(notification: Notification) {
    if (!notification.read) {
      await markRead([notification.id]);
    }
  }

  return (
    <div className="relative">
      <button
        type="button"
        onClick={handleOpen}
        aria-label="Notifications"
        aria-expanded={open}
        className="relative flex h-9 w-9 items-center justify-center rounded-full border border-zinc-800 bg-zinc-900 text-zinc-300 hover:text-white hover:border-zinc-700 transition-colors"
      >
        <Bell className="h-4 w-4" aria-hidden="true" />
        {unreadCount > 0 && (
          <span className="absolute -top-1 -right-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-[#37B7C3] px-1 text-[10px] font-bold text-black">
            {unreadCount > 9 ? "9+" : unreadCount}
          </span>
        )}
      </button>

      {open && (
        <>
          <button
            type="button"
            aria-label="Close notifications"
            className="fixed inset-0 z-40 cursor-default"
            onClick={() => setOpen(false)}
          />
          <div className="absolute right-0 z-50 mt-2 w-80 max-h-96 overflow-y-auto rounded-xl border border-zinc-800 bg-zinc-900 shadow-2xl">
            <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800">
              <p className="text-sm font-semibold text-white">Notifications</p>
              {unreadCount > 0 && (
                <button
                  type="button"
                  onClick={() => markRead()}
                  className="text-xs font-medium text-[#37B7C3] hover:underline"
                >
                  Mark all read
                </button>
              )}
            </div>

            {isLoading ? (
              <p className="p-6 text-center text-sm text-zinc-500">Loading…</p>
            ) : notifications.length === 0 ? (
              <p className="p-6 text-center text-sm text-zinc-500">You&apos;re all caught up.</p>
            ) : (
              <ul>
                {notifications.map((notification) => (
                  <li key={notification.id}>
                    <button
                      type="button"
                      onClick={() => handleNotificationClick(notification)}
                      className={cn(
                        "w-full text-left px-4 py-3 border-b border-zinc-800/50 last:border-0 hover:bg-white/[0.03] transition-colors",
                        !notification.read && "bg-[#37B7C3]/[0.04]",
                      )}
                    >
                      <div className="flex items-center gap-2">
                        <span
                          className={cn(
                            "text-[10px] font-semibold px-1.5 py-0.5 rounded-full",
                            TYPE_STYLES[notification.notif_type],
                          )}
                        >
                          {TYPE_LABEL[notification.notif_type]}
                        </span>
                        {!notification.read && (
                          <span className="h-1.5 w-1.5 rounded-full bg-[#37B7C3]" />
                        )}
                      </div>
                      <p className="text-sm text-white mt-1">{notification.title}</p>
                      <p className="text-xs text-zinc-500 mt-0.5">{notification.message}</p>
                      <p className="text-[10px] text-zinc-600 mt-1">
                        {formatUtcDateTime(notification.created_at)}
                      </p>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </>
      )}
    </div>
  );
}
