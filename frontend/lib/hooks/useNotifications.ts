"use client";

import useSWR from "swr";
import {
  fetchNotifications,
  markNotificationsRead,
  notificationsUrl,
  type NotificationsResponse,
} from "@/lib/api/notifications";

export interface UseNotificationsResult {
  notifications: NotificationsResponse["notifications"];
  unreadCount: number;
  isLoading: boolean;
  isError: boolean;
  markRead: (ids?: number[]) => Promise<void>;
  refresh: () => void;
}

/**
 * useNotifications — polls a user's notifications every 30s so the bell badge
 * stays fresh without a WebSocket round-trip.
 */
export function useNotifications(address: string | undefined): UseNotificationsResult {
  const key = address ? notificationsUrl(address) : null;

  const { data, error, isLoading, mutate } = useSWR<NotificationsResponse>(
    key,
    fetchNotifications,
    { refreshInterval: 30_000 },
  );

  async function markRead(ids?: number[]) {
    if (!address) return;
    await markNotificationsRead(address, ids);
    void mutate();
  }

  return {
    notifications: data?.notifications ?? [],
    unreadCount: data?.unread_count ?? 0,
    isLoading,
    isError: Boolean(error),
    markRead,
    refresh: () => {
      void mutate();
    },
  };
}
