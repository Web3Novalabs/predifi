"use client";

import { memo } from "react";
import { Users, AlertCircle } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  CopyButton,
  Skeleton,
} from "@/components/ui";
import { cn } from "@/lib/utils";
import { truncateAddress } from "@/components/settings/SecuritySettings";
import { useWalletAddress } from "@/lib/context/WalletContext";
import { useReferralEarnings } from "@/lib/hooks/useReferralEarnings";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Format a whole-token referral amount as `1,234.00 XLM`. */
function formatXlm(amount: number): string {
  return `${amount.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })} XLM`;
}

// ---------------------------------------------------------------------------
// ReferralsHistoryTable
// ---------------------------------------------------------------------------

export function ReferralsHistoryTable() {
  const address = useWalletAddress();
  const { earnings, isLoading, isError, error, refresh } =
    useReferralEarnings(address);

  const rows = earnings?.pools ?? [];
  const totalEarned = earnings?.total_earned ?? 0;

  return (
    <Card className="bg-[#121212] border-none text-white overflow-hidden">
      <CardHeader className="pb-3">
        <CardTitle className="text-base font-semibold text-white">
          Referrals History
        </CardTitle>
        {address && (
          <CardDescription className="text-xs text-zinc-500">
            <span className="font-mono text-zinc-400">
              {truncateAddress(address)}
            </span>
            {rows.length > 0 && (
              <>
                {" · "}
                <span className="text-zinc-400">
                  {formatXlm(totalEarned)} earned
                </span>
              </>
            )}
          </CardDescription>
        )}
      </CardHeader>
      <CardContent className="p-0">
        {!address ? (
          <ReferralsMessage
            title="Connect your wallet"
            description="Connect a wallet to see the referrals you've earned."
          />
        ) : isLoading ? (
          <ReferralsLoading />
        ) : isError ? (
          <ReferralsError
            message={error?.message}
            onRetry={refresh}
          />
        ) : rows.length === 0 ? (
          <ReferralsMessage
            title="No referrals yet"
            description="Share your referral link to start earning rewards on the predictions you bring in."
          />
        ) : (
          <div className="overflow-x-auto dark-scroll">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-white/5 text-zinc-400 text-xs uppercase tracking-wide">
                  <th className="px-6 py-3 text-left font-medium">Pool</th>
                  <th className="px-6 py-3 text-left font-medium">Referrals</th>
                  <th className="px-6 py-3 text-left font-medium">
                    Total Earned
                  </th>
                  <th className="px-6 py-3 text-left font-medium">Pool ID</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-white/5">
                {rows.map((row) => (
                  <tr
                    key={row.pool_id}
                    className="hover:bg-white/[0.03] transition-colors"
                  >
                    <td className="px-6 py-4 text-zinc-200">{row.pool_name}</td>
                    <td className="px-6 py-4 font-mono text-zinc-300 whitespace-nowrap">
                      {row.referral_count.toLocaleString("en-US")}
                    </td>
                    <td className="px-6 py-4 font-mono text-white font-medium whitespace-nowrap">
                      {formatXlm(row.total_earned)}
                    </td>
                    <td className="px-6 py-4 font-mono text-zinc-400 whitespace-nowrap">
                      <span className="flex items-center gap-1">
                        #{row.pool_id}
                        <CopyButton
                          text={String(row.pool_id)}
                          size="xs"
                          aria-label={`Copy pool ID ${row.pool_id}`}
                          copyOptions={{
                            successDescription: `Pool ID ${row.pool_id} copied to clipboard`,
                          }}
                          className="ml-0.5"
                        />
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// States
// ---------------------------------------------------------------------------

const ReferralsLoading = memo(function ReferralsLoading() {
  return (
    <div
      className="space-y-3 px-6 py-5"
      role="status"
      aria-label="Loading referrals"
    >
      <span className="sr-only">Loading referrals…</span>
      {Array.from({ length: 3 }).map((_, i) => (
        <Skeleton key={i} className="h-5 w-full" />
      ))}
    </div>
  );
});

ReferralsLoading.displayName = "ReferralsLoading";

const ReferralsMessage = memo(function ReferralsMessage({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 px-6 py-12 text-center">
      <div className="mb-1 flex h-11 w-11 items-center justify-center rounded-full border border-white/5 bg-white/[0.03]">
        <Users className="h-5 w-5 text-[#37B7C3]/70" />
      </div>
      <h3 className="text-sm font-semibold text-white">{title}</h3>
      <p className="max-w-xs text-sm leading-relaxed text-zinc-500">
        {description}
      </p>
    </div>
  );
});

ReferralsMessage.displayName = "ReferralsMessage";

const ReferralsError = memo(function ReferralsError({
  message,
  onRetry,
}: {
  message?: string;
  onRetry: () => void;
}) {
  return (
    <div
      className="flex flex-col items-center justify-center gap-3 px-6 py-12 text-center"
      role="alert"
    >
      <div className="flex h-11 w-11 items-center justify-center rounded-full border border-red-500/20 bg-red-500/10">
        <AlertCircle className="h-5 w-5 text-red-400" />
      </div>
      <div className="space-y-1">
        <h3 className="text-sm font-semibold text-white">
          Couldn&apos;t load referrals
        </h3>
        <p className="max-w-xs text-sm text-zinc-500">
          {message ?? "Something went wrong. Please try again."}
        </p>
      </div>
      <button
        type="button"
        onClick={onRetry}
        className={cn(
          "rounded-lg border border-[#37B7C3]/30 bg-[#37B7C3]/10 px-4 py-2 text-sm font-medium text-[#37B7C3]",
          "transition-colors hover:border-[#37B7C3]/60 hover:bg-[#37B7C3]/20",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3]",
        )}
      >
        Try again
      </button>
    </div>
  );
});

ReferralsError.displayName = "ReferralsError";
