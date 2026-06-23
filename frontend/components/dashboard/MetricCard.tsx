import { ArrowUpRight, ArrowDownRight, Minus } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card, CardContent, Skeleton } from "@/components/ui";

interface MetricCardProps {
    title: string;
    value: React.ReactNode;
    icon: React.ReactNode;
    change?: string;
    changeType?: "positive" | "negative" | "neutral";
    subtext?: string;
    isLoading?: boolean;
}

export function MetricCard({
    title,
    value,
    icon,
    change,
    changeType = "neutral",
    subtext,
    isLoading = false,
}: MetricCardProps) {
    if (isLoading) {
        return (
            <Card className="bg-[#121212] border-none text-white relative overflow-hidden">
                <CardContent className="p-6 flex items-start gap-4">
                    <Skeleton className="w-12 h-12 rounded-xl" />
                    <div className="space-y-2 flex-1">
                        <Skeleton className="h-3 w-24" />
                        <Skeleton className="h-8 w-20" />
                        <Skeleton className="h-5 w-16 rounded-full" />
                    </div>
                </CardContent>
            </Card>
        );
    }

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
            {/* Background SVG graphic */}
            <svg
                aria-hidden="true"
                className="pointer-events-none absolute inset-0 h-full w-full opacity-[0.07] transition-opacity duration-500 group-hover:opacity-[0.13]"
                xmlns="http://www.w3.org/2000/svg"
                preserveAspectRatio="xMidYMid slice"
            >
                <defs>
                    <radialGradient id="mg-glow" cx="85%" cy="15%" r="50%">
                        <stop offset="0%" stopColor="#37B7C3" stopOpacity="1" />
                        <stop offset="100%" stopColor="#37B7C3" stopOpacity="0" />
                    </radialGradient>
                </defs>
                {/* Glow orb */}
                <ellipse cx="90%" cy="10%" rx="40%" ry="35%" fill="url(#mg-glow)" />
                {/* Corner hex ring */}
                <polygon
                    points="88,4 100,11 100,25 88,32 76,25 76,11"
                    fill="none"
                    stroke="#37B7C3"
                    strokeWidth="0.75"
                    className="transition-all duration-500 group-hover:stroke-[1.2]"
                />
                <polygon
                    points="94,8 100,11.5 100,22.5 94,26 88,22.5 88,11.5"
                    fill="none"
                    stroke="#37B7C3"
                    strokeWidth="0.4"
                    opacity="0.5"
                />
                {/* Diagonal grid lines */}
                <line x1="60%" y1="0" x2="100%" y2="60%" stroke="#37B7C3" strokeWidth="0.4" />
                <line x1="75%" y1="0" x2="100%" y2="40%" stroke="#37B7C3" strokeWidth="0.3" />
                <line x1="100%" y1="0" x2="60%" y2="80%" stroke="#37B7C3" strokeWidth="0.3" />
                {/* Bottom-left accent dot */}
                <circle cx="8%" cy="88%" r="1.5" fill="#37B7C3" opacity="0.5" />
                <circle cx="8%" cy="88%" r="4" fill="none" stroke="#37B7C3" strokeWidth="0.5" opacity="0.3" />
            </svg>
        </Card>
    );
}
