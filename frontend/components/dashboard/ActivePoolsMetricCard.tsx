"use client";

import { Box } from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { usePools } from "@/lib/hooks/usePools";

interface ActivePoolsMetricCardProps {
  isLoading?: boolean;
}

export function ActivePoolsMetricCard({ isLoading: forceLoading = false }: ActivePoolsMetricCardProps) {
  const { total, isLoading, isError } = usePools({
    status: "active",
    limit: 1,
  });

  return (
    <MetricCard
      title="Active Pools"
      value={isError ? "—" : total.toLocaleString()}
      icon={<Box />}
      change={isError ? "Count unavailable" : `${total} live now`}
      changeType={isError ? "neutral" : "positive"}
      tooltip="Prediction pools that are currently open for participation."
      isLoading={forceLoading || isLoading}
    />
  );
}
