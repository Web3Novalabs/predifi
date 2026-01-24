import { ArrowUpRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function BalanceSection() {
    return (
        <Card className="bg-[#121212] border-none text-white h-full relative overflow-hidden">
            <CardHeader>
                <CardTitle className="text-zinc-400 font-medium text-sm">Total Balance</CardTitle>
            </CardHeader>
            <CardContent className="space-y-6 relative z-10">
                <div>
                    <h2 className="text-5xl font-bold font-mono tracking-tighter mb-4">$15,255.25</h2>
                    <div className="inline-flex items-center px-4 py-2 rounded-full bg-zinc-900 border border-white/5">
                        <span className="text-zinc-400 mr-2">Rewards:</span>
                        <span className="font-bold text-white font-mono">$1,255.68</span>
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
