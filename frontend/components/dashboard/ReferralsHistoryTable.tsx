"use client";

import { useState, useCallback, memo } from "react";
import { Copy, CheckCircle2, Clock, XCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ReferralStatus = "Completed" | "Pending" | "Failed";

interface Referral {
  id: string;
  date: string;
  referredUser: string;
  reward: string;
  status: ReferralStatus;
}

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

const referralHistory: Referral[] = [
  {
    id: "REF-001",
    date: "2025-05-12T10:30:00Z",
    referredUser: "0xA3B2...C19F",
    reward: "25.00 XLM",
    status: "Completed",
  },
  {
    id: "REF-002",
    date: "2025-05-18T14:10:00Z",
    referredUser: "0xD7E4...88AA",
    reward: "25.00 XLM",
    status: "Pending",
  },
  {
    id: "REF-003",
    date: "2025-06-01T09:55:00Z",
    referredUser: "0xF001...3C44",
    reward: "25.00 XLM",
    status: "Completed",
  },
  {
    id: "REF-004",
    date: "2025-06-10T17:20:00Z",
    referredUser: "0x12AB...9F0E",
    reward: "25.00 XLM",
    status: "Failed",
  },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

const STATUS_CONFIG: Record<
  ReferralStatus,
  { label: string; icon: React.ElementType; className: string }
> = {
  Completed: {
    label: "Completed",
    icon: CheckCircle2,
    className: "text-emerald-400 bg-emerald-400/10",
  },
  Pending: {
    label: "Pending",
    icon: Clock,
    className: "text-yellow-400 bg-yellow-400/10",
  },
  Failed: {
    label: "Failed",
    icon: XCircle,
    className: "text-red-400 bg-red-400/10",
  },
};

// ---------------------------------------------------------------------------
// StatusBadge
// ---------------------------------------------------------------------------

const StatusBadge = memo(function StatusBadge({ status }: { status: ReferralStatus }) {
  const { label, icon: Icon, className } = STATUS_CONFIG[status];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium",
        className,
      )}
    >
      <Icon className="w-3 h-3" />
      {label}
    </span>
  );
});

StatusBadge.displayName = "StatusBadge";

// ---------------------------------------------------------------------------
// CopyButton — copies referral id
// ---------------------------------------------------------------------------

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard API unavailable
    }
  }, [text]);

  return (
    <button
      onClick={handleCopy}
      aria-label={copied ? "Copied" : "Copy ID"}
      className="ml-1.5 text-zinc-500 hover:text-white transition-colors"
    >
      <Copy className={cn("w-3 h-3", copied && "text-emerald-400")} />
    </button>
  );
}

// ---------------------------------------------------------------------------
// ReferralsHistoryTable
// ---------------------------------------------------------------------------

export function ReferralsHistoryTable() {
  return (
    <Card className="bg-[#121212] border-none text-white overflow-hidden">
      <CardHeader className="pb-3">
        <CardTitle className="text-base font-semibold text-white">
          Referrals History
        </CardTitle>
      </CardHeader>
      <CardContent className="p-0">
        {referralHistory.length === 0 ? (
          <p className="px-6 pb-6 text-sm text-zinc-500">No referrals yet.</p>
        ) : (
          <div className="overflow-x-auto dark-scroll">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-white/5 text-zinc-400 text-xs uppercase tracking-wide">
                  <th className="px-6 py-3 text-left font-medium">Date</th>
                  <th className="px-6 py-3 text-left font-medium">Referred User</th>
                  <th className="px-6 py-3 text-left font-medium">Reward</th>
                  <th className="px-6 py-3 text-left font-medium">Ref ID</th>
                  <th className="px-6 py-3 text-left font-medium">Status</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-white/5">
                {referralHistory.map((ref) => (
                  <tr
                    key={ref.id}
                    className="hover:bg-white/[0.03] transition-colors"
                  >
                    <td className="px-6 py-4 text-zinc-300 whitespace-nowrap">
                      {formatDate(ref.date)}
                    </td>
                    <td className="px-6 py-4 font-mono text-zinc-300 whitespace-nowrap">
                      {ref.referredUser}
                    </td>
                    <td className="px-6 py-4 font-mono text-white font-medium whitespace-nowrap">
                      {ref.reward}
                    </td>
                    <td className="px-6 py-4 font-mono text-zinc-400 whitespace-nowrap">
                      <span className="flex items-center">
                        {ref.id}
                        <CopyButton text={ref.id} />
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <StatusBadge status={ref.status} />
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
