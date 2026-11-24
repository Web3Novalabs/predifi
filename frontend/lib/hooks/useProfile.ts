"use client";

import useSWR from "swr";
import { fetchProfile, profileUrl, type UserProfile } from "@/lib/api/profile";

export interface UseProfileResult {
  profile: UserProfile | undefined;
  isLoading: boolean;
  isError: boolean;
  error: Error | undefined;
  refresh: () => void;
}

/**
 * useProfile — SWR-cached access to a user's full profile: aggregate stats,
 * per-pool claim status, and the daily activity series used for charts.
 */
export function useProfile(address: string | undefined): UseProfileResult {
  const key = address ? profileUrl(address) : null;

  const { data, error, isLoading, mutate } = useSWR<UserProfile>(key, fetchProfile);

  return {
    profile: data,
    isLoading,
    isError: Boolean(error),
    error: error as Error | undefined,
    refresh: () => {
      void mutate();
    },
  };
}
