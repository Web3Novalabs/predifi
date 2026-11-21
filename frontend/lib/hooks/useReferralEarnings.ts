"use client";

import useSWR from "swr";
import {
  fetchReferralEarnings,
  referralsUrl,
  type ReferralEarnings,
} from "@/lib/api/referrals";

export interface UseReferralEarningsResult {
  earnings: ReferralEarnings | undefined;
  isLoading: boolean;
  isError: boolean;
  error: Error | undefined;
  refresh: () => void;
}

/**
 * useReferralEarnings — SWR-cached access to a user's per-pool referral
 * earnings. The request is skipped when no wallet address is connected.
 */
export function useReferralEarnings(
  address: string | undefined,
): UseReferralEarningsResult {
  const key = address ? referralsUrl(address) : null;

  const { data, error, isLoading, mutate } = useSWR<ReferralEarnings>(
    key,
    fetchReferralEarnings,
  );

  return {
    earnings: data,
    isLoading,
    isError: Boolean(error),
    error: error as Error | undefined,
    refresh: () => {
      void mutate();
    },
  };
}
