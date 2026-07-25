"use client";

/**
 * StakedChartRenderer
 *
 * Intentionally isolated so recharts is placed in its own JS chunk by the
 * bundler. Loaded exclusively via dynamic import in StakedChart.tsx so the
 * library never ships in the initial page bundle.
 *
 * Sizing strategy
 * ───────────────
 * The parent (StakedChart) measures its own container with useContainerSize
 * and passes explicit pixel dimensions here.  We still wrap the BarChart in
 * ResponsiveContainer so recharts' internal layout math is happy, but we
 * set width/height on the container to the measured values rather than "100%"
 * so re-renders are triggered correctly when the container resizes.
 *
 * Fallback: when dimensions are not yet known (first render / SSR), the
 * component renders nothing — the skeleton placeholder handles that state.
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

interface TooltipPayload {
  value: number;
}

interface CustomTooltipProps {
  active?: boolean;
  payload?: TooltipPayload[];
  label?: string;
}

const CustomTooltip = ({ active, payload, label }: CustomTooltipProps) => {
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

export interface StakedChartRendererProps {
  /** Measured container width in px from useContainerSize. */
  width: number;
  /** Measured container height in px from useContainerSize. */
  height: number;
}

/**
 * The raw recharts bar chart.
 *
 * Width and height are driven by the parent's ResizeObserver measurement so
 * the chart scales precisely when the layout container changes size — e.g.
 * on window resize, sidebar collapse, or responsive breakpoint transitions.
 */
export function StakedChartRenderer({ width, height }: StakedChartRendererProps) {
  // Don't render until we have real measurements
  if (width === 0 || height === 0) return null;

  return (
    <ResponsiveContainer width={width} height={height}>
      <BarChart
        data={data}
        margin={{ top: 4, right: 4, bottom: 0, left: 4 }}
      >
        <XAxis
          dataKey="name"
          axisLine={false}
          tickLine={false}
          tick={{ fill: "#525252", fontSize: 10 }}
          dy={10}
        />
        <Tooltip
          content={<CustomTooltip />}
          cursor={{ fill: "transparent" }}
        />
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
