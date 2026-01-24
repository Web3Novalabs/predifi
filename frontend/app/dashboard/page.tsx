import { Diamond, Box, Activity, ShieldCheck } from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { BalanceSection } from "@/components/dashboard/BalanceSection";
import { StakedChart } from "@/components/dashboard/StakedChart";
import { PredictionList } from "@/components/dashboard/PredictionList";
import { PoolsList } from "@/components/dashboard/PoolsList";

export default function DashboardPage() {
    return (
        <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8 space-y-8">
            {/* Header */}
            <div className="space-y-1">
                <h1 className="text-3xl font-bold text-white">My Dashboard</h1>
                <p className="text-zinc-400 text-sm">
                    Lorem ipsum dolor sit amet consectetur. Non eget non odio lobortis odio.
                </p>
            </div>

            {/* Metric Cards Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                <MetricCard
                    title="Total Earned"
                    value="1,255"
                    icon={<Diamond />}
                    change="65% increase"
                    changeType="positive"
                />
                <MetricCard
                    title="Active Pool"
                    value="75"
                    icon={<Box />}
                    change="+7 new add"
                    changeType="positive" // Using positive usage for green color
                />
                <MetricCard
                    title="Win Rate"
                    value="65%"
                    icon={<Activity />}
                    change="7.8% Growth"
                    changeType="positive"
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
                />
            </div>

            {/* Charts Section */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <div className="lg:col-span-1 h-[320px]">
                    <BalanceSection />
                </div>
                <div className="lg:col-span-2 h-[320px]">
                    <StakedChart />
                </div>
            </div>

            {/* Activity Section */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <div className="lg:col-span-1">
                    <PredictionList />
                </div>
                <div className="lg:col-span-2">
                    <PoolsList />
                </div>
            </div>
        </div>
    );
}
