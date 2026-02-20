"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Card, CardContent } from "@/components/ui/card";
import { ChefHat, ChevronRight, Users, Copy } from "lucide-react";

interface Prediction {
    id: string;
    title: string;
    date: string;
    potentialPayout: string;
    stake: string;
    odd: string;
    idNo: string;
    creator: string;
    participants: number;
    status: "Pending" | "Completed";
}

const activePredictions: Prediction[] = [
    {
        id: "1",
        title: "125,000 or above",
        date: "18-04-2025 21:43",
        potentialPayout: "179.52 strk",
        stake: "100 strk",
        odd: "2.54",
        idNo: "19133DK",
        creator: "Best Al this mon...",
        participants: 185,
        status: "Pending"
    }
];

export function PredictionList() {
    const [activeTab, setActiveTab] = useState<"active" | "past">("active");

    return (
        <div className="space-y-6">
            <div className="flex items-center gap-8 border-b border-zinc-800 pb-1">
                <button
                    onClick={() => setActiveTab("active")}
                    className={cn(
                        "pb-3 text-sm font-medium transition-colors relative",
                        activeTab === "active" ? "text-primary" : "text-muted-foreground hover:text-white"
                    )}
                >
                    Active Prediction
                    {activeTab === "active" && (
                        <span className="absolute bottom-0 left-0 w-full h-0.5 bg-primary" />
                    )}
                    <span className="ml-2 bg-[#37B7C3] text-[#121212] text-[10px] font-bold px-1.5 py-0.5 rounded-full relative -top-0.5">
                        6
                    </span>
                </button>
                <button
                    onClick={() => setActiveTab("past")}
                    className={cn(
                        "pb-3 text-sm font-medium transition-colors relative",
                        activeTab === "past" ? "text-primary" : "text-muted-foreground hover:text-white"
                    )}
                >
                    Past Predictions
                    {activeTab === "past" && (
                        <span className="absolute bottom-0 left-0 w-full h-0.5 bg-primary" />
                    )}
                </button>
            </div>

            <div className="space-y-4">
                {activeTab === "active" ? (
                    activePredictions.map((prediction) => (
                        <Card key={prediction.id} className="bg-[#1E1E1E] border-none text-white overflow-hidden">
                            <CardContent className="p-0">
                                {/* Header */}
                                <div className="p-4 flex items-center justify-between border-b border-white/5">
                                    <div>
                                        <h4 className="font-bold text-base">{prediction.title}</h4>
                                        <p className="text-zinc-500 text-xs mt-1">{prediction.date}</p>
                                    </div>
                                    <div className="flex items-center gap-2">
                                        <span className="text-emerald-400 text-xs font-bold">{prediction.status}</span>
                                        <ChevronRight className="w-4 h-4 text-zinc-500" />
                                    </div>
                                </div>

                                {/* Stats */}
                                <div className="p-4 space-y-3">
                                    <div className="flex justify-between items-center text-sm">
                                        <span className="text-zinc-400">Potential Payout:</span>
                                        <span className="font-bold font-mono text-lg">{prediction.potentialPayout}</span>
                                    </div>
                                    <div className="flex justify-between items-center text-sm">
                                        <span className="text-zinc-400">Stake</span>
                                        <span className="text-white">{prediction.stake}</span>
                                    </div>
                                    <div className="flex justify-between items-center text-sm">
                                        <span className="text-zinc-400">Odd</span>
                                        <span className="text-white">{prediction.odd}</span>
                                    </div>
                                    <div className="flex justify-between items-center text-sm">
                                        <span className="text-zinc-400">ID No.</span>
                                        <div className="flex items-center gap-2 text-white">
                                            {prediction.idNo}
                                            <Copy className="w-3 h-3 text-zinc-500 cursor-pointer hover:text-white" />
                                        </div>
                                    </div>
                                </div>

                                {/* Footer */}
                                <div className="p-4 bg-zinc-900/50 flex items-center justify-between">
                                    <div className="flex items-center gap-2">
                                        <div className="w-8 h-8 rounded-lg bg-indigo-500/20 flex items-center justify-center text-indigo-400">
                                            <ChefHat className="w-5 h-5" />
                                        </div>
                                        <span className="text-sm font-medium text-zinc-300">{prediction.creator}</span>
                                    </div>
                                    <div className="flex items-center gap-1 text-zinc-400 text-sm">
                                        <Users className="w-4 h-4" />
                                        <span>{prediction.participants}</span>
                                    </div>
                                </div>
                            </CardContent>
                        </Card>
                    ))
                ) : (
                    <div className="text-center py-10 text-zinc-500">
                        No past predictions found
                    </div>
                )}
            </div>
        </div>
    );
}
