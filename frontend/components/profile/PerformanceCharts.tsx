"use client";

import { useMemo } from "react";
import {
  Area,
  AreaChart,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui";
import { formatChartValue } from "@/lib/stakeFilters";
import type { PerformancePoint, ProfileStats } from "@/lib/api/profile";

const WIN_COLOR = "#34d399";
const LOSS_COLOR = "#f87171";
const PENDING_COLOR = "#71717a";
const EARNINGS_COLOR = "#37B7C3";
const STAKED_COLOR = "#6366f1";

interface EarningsTooltipPayload {
  value: number;
  dataKey: string;
}

function EarningsTooltip({
  active,
  payload,
  label,
}: {
  active?: boolean;
  payload?: EarningsTooltipPayload[];
  label?: string;
}) {
  if (!active || !payload || payload.length === 0) return null;
  return (
    <div className="bg-zinc-900 border border-white/10 p-3 rounded-lg shadow-xl space-y-1">
      <p className="text-zinc-400 text-xs">{label}</p>
      {payload.map((entry) => (
        <p key={entry.dataKey} className="text-white text-sm font-mono">
          {entry.dataKey === "earnings" ? "Earnings" : "Staked"}:{" "}
          {formatChartValue(entry.value)}
        </p>
      ))}
    </div>
  );
}

/** Accumulates the raw daily series into cumulative staked/earnings totals. */
function toCumulative(points: PerformancePoint[]) {
  let staked = 0;
  let earnings = 0;
  return points.map((p) => {
    staked += p.staked;
    earnings += p.earnings;
    return {
      day: new Date(p.day).toLocaleDateString(undefined, { month: "short", day: "numeric" }),
      staked,
      earnings,
    };
  });
}

interface PerformanceChartsProps {
  stats: ProfileStats;
  performance: PerformancePoint[];
}

export function PerformanceCharts({ stats, performance }: PerformanceChartsProps) {
  const cumulative = useMemo(() => toCumulative(performance), [performance]);

  const winLossData = useMemo(
    () => [
      { name: "Won", value: stats.wins, color: WIN_COLOR },
      { name: "Lost", value: stats.losses, color: LOSS_COLOR },
      { name: "Pending", value: stats.pending, color: PENDING_COLOR },
    ].filter((d) => d.value > 0),
    [stats],
  );

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <Card className="bg-[#121212] border-none text-white lg:col-span-2">
        <CardHeader>
          <CardTitle className="text-base font-medium">Performance Over Time</CardTitle>
        </CardHeader>
        <CardContent className="h-[280px]">
          {cumulative.length === 0 ? (
            <div className="flex h-full items-center justify-center text-zinc-600 text-sm">
              No activity yet.
            </div>
          ) : (
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={cumulative} margin={{ top: 8, right: 8, bottom: 0, left: 0 }}>
                <defs>
                  <linearGradient id="earningsGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor={EARNINGS_COLOR} stopOpacity={0.35} />
                    <stop offset="95%" stopColor={EARNINGS_COLOR} stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="stakedGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor={STAKED_COLOR} stopOpacity={0.25} />
                    <stop offset="95%" stopColor={STAKED_COLOR} stopOpacity={0} />
                  </linearGradient>
                </defs>
                <XAxis
                  dataKey="day"
                  axisLine={false}
                  tickLine={false}
                  tick={{ fill: "#525252", fontSize: 10 }}
                  dy={10}
                />
                <YAxis
                  axisLine={false}
                  tickLine={false}
                  tick={{ fill: "#525252", fontSize: 10 }}
                  width={48}
                  tickFormatter={(v: number) => formatChartValue(v)}
                />
                <Tooltip content={<EarningsTooltip />} />
                <Area
                  type="monotone"
                  dataKey="staked"
                  stroke={STAKED_COLOR}
                  fill="url(#stakedGradient)"
                  strokeWidth={2}
                />
                <Area
                  type="monotone"
                  dataKey="earnings"
                  stroke={EARNINGS_COLOR}
                  fill="url(#earningsGradient)"
                  strokeWidth={2}
                />
              </AreaChart>
            </ResponsiveContainer>
          )}
        </CardContent>
      </Card>

      <Card className="bg-[#121212] border-none text-white">
        <CardHeader>
          <CardTitle className="text-base font-medium">Win / Loss</CardTitle>
        </CardHeader>
        <CardContent className="h-[280px] flex flex-col items-center justify-center">
          {winLossData.length === 0 ? (
            <div className="text-zinc-600 text-sm">No settled predictions yet.</div>
          ) : (
            <>
              <ResponsiveContainer width="100%" height={180}>
                <PieChart>
                  <Pie
                    data={winLossData}
                    dataKey="value"
                    nameKey="name"
                    innerRadius={50}
                    outerRadius={80}
                    paddingAngle={2}
                  >
                    {winLossData.map((entry) => (
                      <Cell key={entry.name} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value: number, name: string) => [value, name]}
                    contentStyle={{
                      background: "#18181b",
                      border: "1px solid rgba(255,255,255,0.1)",
                      borderRadius: 8,
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
              <div className="flex items-center gap-4 mt-2 text-xs">
                {winLossData.map((entry) => (
                  <div key={entry.name} className="flex items-center gap-1.5">
                    <span
                      className="h-2 w-2 rounded-full"
                      style={{ backgroundColor: entry.color }}
                    />
                    <span className="text-zinc-400">
                      {entry.name} ({entry.value})
                    </span>
                  </div>
                ))}
              </div>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
