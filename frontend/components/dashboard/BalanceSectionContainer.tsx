"use client";

import { BalanceSection } from "@/components/dashboard/BalanceSection";
import { useCurrentUserAddress } from "@/lib/hooks/useCurrentUserAddress";
import { useProfile } from "@/lib/hooks/useProfile";

/**
 * Feeds BalanceSection with real data instead of a demo value.
 *
 * - Rewards are the sum of the connected wallet's unclaimed, unexpired winning
 *   claims from the profile API.
 * - There is no wallet/token balance source in the app yet, so the balance is
 *   passed as `null` ("Unavailable") rather than a made-up number. Pass the real
 *   value here once a balance hook exists.
 * - While the profile is loading, rewards are `undefined` (skeleton).
 */
export function BalanceSectionContainer() {
  const address = useCurrentUserAddress();
  const { profile, isLoading, isError } = useProfile(address);

  let rewards: number | null | undefined;
  if (!address || isError) {
    rewards = null;
  } else if (isLoading || !profile) {
    rewards = undefined;
  } else {
    rewards = profile.claims
      .filter((c) => c.is_winner === true && !c.claimed && !c.claim_expired)
      .reduce((sum, c) => sum + c.amount, 0);
  }

  return <BalanceSection balance={null} rewards={rewards} />;
}
