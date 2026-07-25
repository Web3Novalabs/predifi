"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { formatUtcDateTime } from "@/lib/date";

type PredictionStatus = "Active" | "Won" | "Lost" | "Pending";

interface PredictionHistoryEntry {
  id: string;
  pool: string;
  outcome: string;
  stake: string;
  payout: string;
  status: PredictionStatus;
  date: string;
}

const MOCK_PREDICTIONS: PredictionHistoryEntry[] = [
  { id: "1", pool: "BTC above $125k by Dec 2025", outcome: "Yes", stake: "50 XLM", payout: "127.50 XLM", status: "Won",     date: "2025-06-01T14:00:00Z" },
  { id: "2", pool: "ETH flips BTC market cap",    outcome: "No",  stake: "30 XLM", payout: "—",           status: "Active",  date: "2025-06-10T09:30:00Z" },
  { id: "3", pool: "XLM price above $1.00 Q3",   outcome: "Yes", stake: "20 XLM", payout: "—",           status: "Pending", date: "2025-06-15T11:00:00Z" },
  { id: "4", pool: "Fed rate cut before Nov",     outcome: "Yes", stake: "40 XLM", payout: "—",           status: "Lost",    date: "2025-05-20T16:45:00Z" },
  { id: "5", pool: "Stellar mainnet upgrade v21", outcome: "Yes", stake: "25 XLM", payout: "62.50 XLM",   status: "Won",     date: "2025-05-05T08:00:00Z" },
];

const STATUS_STYLES: Record<PredictionStatus, string> = {
  Active:  "bg-blue-500/20 text-blue-400",
  Won:     "bg-emerald-500/20 text-emerald-400",
  Lost:    "bg-red-500/20 text-red-400",
  Pending: "bg-yellow-500/20 text-yellow-400",
};

type Tab = "All" | PredictionStatus;
const TABS: Tab[] = ["All", "Active", "Won", "Lost", "Pending"];

export function PredictionHistoryClient() {
  const [activeTab, setActiveTab] = useState<Tab>("All");

  const filtered = activeTab === "All"
    ? MOCK_PREDICTIONS
    : MOCK_PREDICTIONS.filter((p) => p.status === activeTab);

  const totalStaked = MOCK_PREDICTIONS.reduce((sum, p) => sum + parseFloat(p.stake), 0);
  const won = MOCK_PREDICTIONS.filter((p) => p.status === "Won");
  const winRate = Math.round((won.length / MOCK_PREDICTIONS.length) * 100);
  const totalEarned = won.reduce((sum, p) => sum + parseFloat(p.payout) || 0, 0);

  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8 space-y-8">
      {/* Profile header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center gap-5">
        <div className="w-16 h-16 rounded-full bg-gradient-to-br from-[#37B7C3]/40 to-indigo-500/40 border border-white/10 flex-shrink-0" />
        <div>
          <h1 className="text-2xl font-bold text-white">My Profile</h1>
          <p className="text-zinc-400 text-xs mt-0.5 font-mono">GDRX…7K4P</p>
        </div>
      </div>

      {/* Stats row */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        {[
          { label: "Total Predictions", value: MOCK_PREDICTIONS.length },
          { label: "Win Rate",          value: `${winRate}%` },
          { label: "Total Staked",      value: `${totalStaked} XLM` },
          { label: "Total Earned",      value: `${totalEarned} XLM` },
        ].map(({ label, value }) => (
          <div key={label} className="rounded-xl border border-zinc-800 bg-zinc-900 p-4 space-y-1">
            <p className="text-[10px] text-zinc-500 uppercase tracking-wider">{label}</p>
            <p className="text-lg font-bold text-white font-mono">{value}</p>
          </div>
        ))}
      </div>

      {/* Predictions history */}
      <div className="space-y-4">
        <h2 className="text-base font-semibold text-white">Prediction History</h2>

        {/* Tabs */}
        <div className="flex items-center gap-1 border-b border-zinc-800 overflow-x-auto">
          {TABS.map((tab) => (
            <button
              key={tab}
              type="button"
              onClick={() => setActiveTab(tab)}
              className={cn(
                "px-4 py-2.5 text-sm font-medium transition-colors relative whitespace-nowrap",
                activeTab === tab
                  ? "text-[#37B7C3]"
                  : "text-zinc-500 hover:text-zinc-300",
              )}
            >
              {tab}
              {activeTab === tab && (
                <span className="absolute bottom-0 left-0 w-full h-0.5 bg-[#37B7C3]" />
              )}
            </button>
          ))}
        </div>

        {/* Table */}
        {filtered.length === 0 ? (
          <p className="text-center py-10 text-zinc-500 text-sm">No predictions found.</p>
        ) : (
          <div className="rounded-xl border border-zinc-800 overflow-hidden">
            {/* Header */}
            <div className="hidden sm:grid grid-cols-6 px-4 py-2.5 bg-zinc-900 text-[10px] text-zinc-500 uppercase tracking-wider border-b border-zinc-800">
              <span className="col-span-2">Pool</span>
              <span>Outcome</span>
              <span>Stake</span>
              <span>Payout</span>
              <span>Status</span>
            </div>
            {/* Rows */}
            {filtered.map((p) => (
              <div
                key={p.id}
                className="grid grid-cols-2 sm:grid-cols-6 items-center gap-2 px-4 py-3 border-b border-zinc-800/50 last:border-0 hover:bg-white/[0.02] transition-colors"
              >
                <div className="col-span-2 sm:col-span-2 min-w-0">
                  <p className="text-sm text-white truncate">{p.pool}</p>
                  <p className="text-[10px] text-zinc-500 mt-0.5">{formatUtcDateTime(p.date)}</p>
                </div>
                <span className="text-sm text-zinc-300 hidden sm:block">{p.outcome}</span>
                <span className="text-sm font-mono text-zinc-300 hidden sm:block">{p.stake}</span>
                <span className="text-sm font-mono text-zinc-300 hidden sm:block">{p.payout}</span>
                <span className={cn("text-[10px] font-semibold px-2 py-1 rounded-full w-fit", STATUS_STYLES[p.status])}>
                  {p.status}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
