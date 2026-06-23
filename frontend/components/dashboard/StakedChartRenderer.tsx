"use client";

/**
 * StakedChartRenderer
 *
 * This module is intentionally isolated so that the recharts library is placed
 * in its own JavaScript chunk by the bundler.  It is loaded exclusively via a
 * dynamic import inside StakedChart.tsx, which means recharts is never part of
 * the initial page bundle and is only fetched when the chart is about to be
 * rendered.
 */

import {
  Bar,
  BarChart,
  Cell,
  ResponsiveContainer,
  Tooltip,
  XAxis,
} from "recharts";
import { formatChartValue } from "@/lib/stakeFilters";

const data = [
  { name: "JAN", value: 35000 },
  { name: "FEB", value: 45000 },
  { name: "MAR", value: 35000 },
  { name: "APR", value: 30000 },
  { name: "MAY", value: 65000, active: true },
  { name: "JUN", value: 15000 },
  { name: "JUL", value: 50000 },
  { name: "AUG", value: 25000 },
  { name: "SEP", value: 45000 },
  { name: "OCT", value: 75000 },
  { name: "NOV", value: 45000 },
  { name: "DEC", value: 60000 },
];

interface TooltipProps {
  active?: boolean;
  payload?: { value: number }[];
  label?: string;
}

const CustomTooltip = ({ active, payload, label }: TooltipProps) => {
  if (active && payload && payload.length) {
    return (
      <div className="bg-zinc-900 border border-white/10 p-2 rounded-lg shadow-xl">
        <p className="text-zinc-400 text-xs mb-1">{label}</p>
        <p className="text-white font-bold font-mono">
          {formatChartValue(payload[0].value)}
        </p>
      </div>
    );
  }
  return null;
};

/**
 * The raw recharts bar chart.  Rendered inside a fixed-height container
 * supplied by the parent StakedChart card.
 */
export function StakedChartRenderer() {
  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={data}>
        <XAxis
          dataKey="name"
          axisLine={false}
          tickLine={false}
          tick={{ fill: "#525252", fontSize: 10 }}
          dy={10}
        />
        <Tooltip content={<CustomTooltip />} cursor={{ fill: "transparent" }} />
        <Bar dataKey="value" radius={[4, 4, 4, 4]}>
          {data.map((entry, index) => (
            <Cell
              key={`cell-${index}`}
              fill={entry.active ? "#37B7C3" : "#262626"}
              className="transition-all duration-300 hover:opacity-80"
            />
          ))}
        </Bar>
      </BarChart>
    </ResponsiveContainer>
  );
}
