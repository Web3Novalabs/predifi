import React from "react";

// Interface for individual metric
interface MetricCardProps {
    title: string;
    value: number | string;
}

// Metric Card Component
const MetricCard: React.FC<MetricCardProps> = ({ title, value }) => {
    return (
        <div className="border-[#fff] w-full lg:w-[205px] h-[119px] border text-center grid place-content-center rounded-lg">
            <h2 className="text-sm font-semibold">{title}</h2>
            <h3 className="font-bold text-3xl">{value}</h3>
        </div>
    );
};

// Configuration for metrics
const SITE_METRICS: MetricCardProps[] = [
    {
        title: "Total Bets Open",
        value: 17,
    },
    {
        title: "Total Volume",
        value: "$45K",
    },
    {
        title: "Active Users",
        value: 250,
    },
];

// Main Site Metrics Component
const SiteMetrics: React.FC = () => {
    return (
        <section className="my-10 px-5 md:px-10 xl:px-[100px]">
            <div className="border p-[3em] lg:p-[100px] rounded-lg">
                <h2 className="text-3xl font-normal font-jersey text-center mb-10">
                    Site Metrics
                </h2>
                <div className="flex flex-col lg:flex-row justify-center gap-4">
                    {SITE_METRICS.map((metric, index) => (
                        <MetricCard
                            key={index}
                            title={metric.title}
                            value={metric.value}
                        />
                    ))}
                </div>
            </div>
        </section>
    );
};

export default SiteMetrics;
