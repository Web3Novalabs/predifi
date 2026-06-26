"use client";

import { Activity, Diamond, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { ActivePoolsMetricCard } from "@/components/dashboard/ActivePoolsMetricCard";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { formatStakeCompact } from "@/lib/stakeFilters";

export function DashboardMetrics() {
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const timer = window.setTimeout(() => setIsLoading(false), 700);
    return () => window.clearTimeout(timer);
  }, []);

  return (
    <div
      className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4"
      aria-busy={isLoading}
      aria-live="polite"
      role="status"
    >
      <MetricCard
        title="Total Earned"
        value={formatStakeCompact(1255)}
        icon={<Diamond />}
        change="65% increase"
        changeType="positive"
        isLoading={isLoading}
      />
      <ActivePoolsMetricCard isLoading={isLoading} />
      <MetricCard
        title="Win Rate"
        value="65%"
        icon={<Activity />}
        change="7.8% Growth"
        changeType="positive"
        isLoading={isLoading}
      />
      <MetricCard
        title="Reputation Score"
        value={
          <span className="flex items-end gap-1">
            <span className="text-[#84CC16]">3.5</span>
            <span className="text-lg text-zinc-500 font-normal mb-1">/5.0</span>
          </span>
        }
        icon={<ShieldCheck />}
        change="70% accuracy"
        changeType="neutral"
        isLoading={isLoading}
      />
    </div>
  );
}
