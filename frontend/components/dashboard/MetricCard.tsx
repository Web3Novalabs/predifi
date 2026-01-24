import { ArrowUpRight, ArrowDownRight, Minus } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card, CardContent } from "@/components/ui/card";

interface MetricCardProps {
    title: string;
    value: React.ReactNode;
    icon: React.ReactNode;
    change?: string;
    changeType?: "positive" | "negative" | "neutral";
    subtext?: string;
}

export function MetricCard({
    title,
    value,
    icon,
    change,
    changeType = "neutral",
    subtext,
}: MetricCardProps) {
    return (
        <Card className="bg-[#121212] border-none text-white relative overflow-hidden group">
            <CardContent className="p-6 flex items-start justify-between relative z-10">
                <div className="flex items-start gap-4">
                    <div className="bg-[#1E1E1E]/50 p-3 rounded-xl border border-white/5 backdrop-blur-sm">
                        <div className="text-primary [&>svg]:w-6 [&>svg]:h-6">{icon}</div>
                    </div>
                    <div className="space-y-1">
                        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                            {title}
                        </p>
                        <h3 className="text-3xl font-bold font-mono tracking-tight">{value}</h3>
                        {(change || subtext) && (
                            <div
                                className={cn(
                                    "flex items-center text-xs font-medium mt-1 w-fit px-2 py-1 rounded-full",
                                    changeType === "positive" && "text-emerald-400 bg-emerald-400/10",
                                    changeType === "negative" && "text-rose-400 bg-rose-400/10",
                                    changeType === "neutral" && "text-blue-400 bg-blue-400/10"
                                )}
                            >
                                {changeType === "positive" && <ArrowUpRight className="w-3 h-3 mr-1" />}
                                {changeType === "negative" && <ArrowDownRight className="w-3 h-3 mr-1" />}
                                {changeType === "neutral" && <Minus className="w-3 h-3 mr-1" />}
                                {change || subtext}
                            </div>
                        )}
                    </div>
                </div>
            </CardContent>
            {/* Background gradient effect */}
            <div className="absolute top-0 right-0 -mt-4 -mr-4 w-24 h-24 bg-primary/5 rounded-full blur-3xl group-hover:bg-primary/10 transition-colors duration-500" />
        </Card>
    );
}
