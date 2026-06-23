import { ArrowUpRight } from "lucide-react";
import { Button, Card, CardContent, CardHeader, CardTitle, Skeleton } from "@/components/ui";
import { formatUsd } from "@/lib/stakeFilters";

interface BalanceSectionProps {
    isLoading?: boolean;
    /** Total balance in USD. Defaults to demo value. */
    balance?: number | string | null;
    /** Rewards amount in USD. Defaults to demo value. */
    rewards?: number | string | null;
}

export function BalanceSection({
    isLoading = false,
    balance = 15255.25,
    rewards = 1255.68,
}: BalanceSectionProps) {
    if (isLoading) {
        return (
            <Card className="bg-[#121212] border-none text-white h-full relative overflow-hidden">
                <CardHeader>
                    <Skeleton className="h-4 w-24" />
                </CardHeader>
                <CardContent className="space-y-6">
                    <div className="space-y-3">
                        <Skeleton className="h-12 w-48" />
                        <Skeleton className="h-9 w-40 rounded-full" />
                    </div>
                    <div className="flex gap-4">
                        <Skeleton className="h-12 w-36 rounded-xl" />
                        <Skeleton className="h-12 w-36 rounded-xl" />
                    </div>
                </CardContent>
            </Card>
        );
    }

    return (
        <Card className="bg-[#121212] border-none text-white h-full relative overflow-hidden">
            <CardHeader>
                <CardTitle className="text-zinc-400 font-medium text-sm">Total Balance</CardTitle>
            </CardHeader>
            <CardContent className="space-y-6 relative z-10">
                <div>
                    <h2 className="text-5xl font-bold font-mono tracking-tighter mb-4">{formatUsd(balance)}</h2>
                    <div className="inline-flex items-center px-4 py-2 rounded-full bg-zinc-900 border border-white/5">
                        <span className="text-zinc-400 mr-2">Rewards:</span>
                        <span className="font-bold text-white font-mono">{formatUsd(rewards)}</span>
                    </div>
                </div>

                <div className="flex gap-4">
                    <Button className="bg-primary hover:bg-primary/90 text-black min-w-[140px] rounded-xl h-12 text-base font-medium group">
                        Withdrawal
                        <ArrowUpRight className="ml-2 w-4 h-4 transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
                    </Button>
                    <Button className="bg-primary hover:bg-primary text-black border-transparent hover:border-transparent min-w-[140px] rounded-xl h-12 text-base font-medium group bg-[#37B7C3] hover:opacity-90">
                        Claim
                        <ArrowUpRight className="ml-2 w-4 h-4 transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
                    </Button>
                </div>
            </CardContent>
        </Card>
    );
}
