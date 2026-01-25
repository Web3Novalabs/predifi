"use client";

import { Bar, BarChart, ResponsiveContainer, XAxis, Tooltip, Cell } from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/button";

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

const CustomTooltip = ({ active, payload, label }: any) => {
    if (active && payload && payload.length) {
        return (
            <div className="bg-zinc-900 border border-white/10 p-2 rounded-lg shadow-xl">
                <p className="text-zinc-400 text-xs mb-1">{label}</p>
                <p className="text-white font-bold font-mono">
                    {/* Formatter for currency */}
                    ${(payload[0].value / 1000).toFixed(0)}k
                </p>
            </div>
        );
    }
    return null;
};

export function StakedChart() {
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
                            {data.map((entry, index) => (
                                <Cell
                                    key={`cell-${index}`}
                                    fill={entry.active ? '#37B7C3' : '#262626'}
                                    className="transition-all duration-300 hover:opacity-80"
                                />
                            ))}
                        </Bar>
                    </BarChart>
                </ResponsiveContainer>
            </CardContent>
        </Card>
    );
}
