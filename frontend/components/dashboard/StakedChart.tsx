"use client";

import { useMemo } from "react";
import { Bar, BarChart, ResponsiveContainer, XAxis, Tooltip, Cell } from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

/**
 * Dynamically import the recharts renderer so the library is code-split into
 * its own chunk.  The loading fallback mirrors the bar-chart skeleton shown
 * during the outer lazy load so the transition is seamless.
 */
const StakedChartRenderer = dynamic(
  () =>
    import("@/components/dashboard/StakedChartRenderer").then(
      (mod) => mod.StakedChartRenderer
    ),
  {
    ssr: false,
    loading: () => (
      <div className="h-full w-full flex items-end gap-2 px-2 pb-2">
        {[55, 70, 55, 45, 90, 25, 75, 40, 70, 100, 70, 85].map((h, i) => (
          <Skeleton
            key={i}
            className="flex-1 rounded-t-sm"
            style={{ height: `${h}%` }}
          />
        ))}
      </div>
    ),
  }
);

interface StakedChartProps {
  isLoading?: boolean;
}

export function StakedChart({ isLoading = false }: StakedChartProps) {
    const skeletonHeights = useMemo(() => [55, 70, 55, 45, 90, 25, 75, 40, 70, 100, 70, 85], []);

    if (isLoading) {
        return (
            <Card className="bg-[#121212] border-none text-white h-full">
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                    <Skeleton className="h-5 w-28" />
                    <div className="flex items-center gap-2">
                        <Skeleton className="h-7 w-12 rounded-full" />
                        <Skeleton className="h-8 w-28 rounded-full" />
                    </div>
                </CardHeader>
                <CardContent className="h-[240px] mt-4 flex items-end gap-2 px-6">
                    {skeletonHeights.map((h, i) => (
                        <Skeleton
                            key={i}
                            className="flex-1 rounded-t-sm"
                            style={{ height: `${h}%` }}
                        />
                    ))}
                </CardContent>
            </Card>
        );
    }

    const chartCells = useMemo(() => 
        data.map((entry, index) => (
            <Cell
                key={`cell-${index}`}
                fill={entry.active ? '#37B7C3' : '#262626'}
                className="transition-all duration-300 hover:opacity-80"
            />
        )),
    []);

    return (
        <Card className="bg-[#121212] border-none text-white h-full">
            <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-lg font-medium">Total staked</CardTitle>
                <div className="flex items-center gap-2">
                    <div className="bg-zinc-800/50 px-3 py-1.5 rounded-full text-xs font-medium text-white/80">
                        23k
                    </div>
                    <Button className="bg-zinc-900 border-zinc-800 text-zinc-400 hover:text-white hover:bg-zinc-800 h-8 rounded-full text-xs">
                        This month
                        <ChevronDown className="ml-2 w-3 h-3" />
                    </Button>
                </div>
            </CardHeader>
            <CardContent className="h-[240px] w-full mt-4">
                <ResponsiveContainer width="100%" height="100%">
                    <BarChart data={data}>
                        <XAxis
                            dataKey="name"
                            axisLine={false}
                            tickLine={false}
                            tick={{ fill: '#525252', fontSize: 10 }}
                            dy={10}
                        />
                        <Tooltip content={<CustomTooltip />} cursor={{ fill: 'transparent' }} />
                        <Bar dataKey="value" radius={[4, 4, 4, 4]}>
                            {chartCells}
                        </Bar>
                    </BarChart>
                </ResponsiveContainer>
            </CardContent>
        </Card>
    );
  }

  return (
    <Card className="bg-[#121212] border-none text-white h-full">
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-lg font-medium">Total staked</CardTitle>
        <div className="flex items-center gap-2">
          <div className="bg-zinc-800/50 px-3 py-1.5 rounded-full text-xs font-medium text-white/80">
            23k
          </div>
          <Button className="bg-zinc-900 border-zinc-800 text-zinc-400 hover:text-white hover:bg-zinc-800 h-8 rounded-full text-xs">
            This month
            <ChevronDown className="ml-2 w-3 h-3" />
          </Button>
        </div>
      </CardHeader>
      {/* StakedChartRenderer is loaded lazily — recharts is not in this chunk */}
      <CardContent className="h-[240px] w-full mt-4">
        <StakedChartRenderer />
      </CardContent>
    </Card>
  );
}
