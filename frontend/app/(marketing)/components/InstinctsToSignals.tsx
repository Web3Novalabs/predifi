import {
  ArrowUpRight,
  ChartPie,
  LayoutPanelTop,
  SquareSplitHorizontal,
} from "lucide-react";

function InstinctsToSignals() {
  return (
    <div>
      <p className="text-[24px] md:text-[48px]/[120%] max-w-[1000px] mx-auto -tracking-[4%] text-center text-[#D9D9D9] font-medium">
        You don’t just bet. turns instincts into signals
      </p>

      <div className="flex lg:flex-row flex-col items-center justify-center gap-y-10 gap-x-[80px] items-center mt-[50px] mb-10">
        <div className="space-y-[10px] max-w-[340px]">
          <ChartPie size={40} />
          <h3 className="text-sm lg:text-lg font-medium">
            Real-Time Statistics
          </h3>
          <p className="text-xs lg:text-base/[140%] tracking-[2%] opacity-70 font-medium">
            Live updates on pool status, stakes, and participant data.
          </p>
        </div>
        <div className="space-y-[10px] max-w-[340px]">
          <SquareSplitHorizontal size={40} />
          <h3 className="text-sm lg:text-lg font-medium">Diverse Pool Types</h3>
          <p className="text-xs lg:text-base/[140%] tracking-[2%] opacity-70 font-medium">
            Win Bet, Opinion-Based, Over/Under, Parlay (coming soon)
          </p>
        </div>
        <div className="space-y-[10px] max-w-[340px]">
          <LayoutPanelTop size={40} />
          <h3 className="text-sm lg:text-lg font-medium">
            Decentralized Platform
          </h3>
          <p className="text-xs lg:text-base/[140%] tracking-[2%] opacity-70 font-medium">
            No central authority; community-driven
          </p>
        </div>
      </div>

      <button className="w-[300px] py-[10px] gap-x-3 items-center flex justify-center text-lg bg-[#37B7C31A] rounded-[10px] mx-auto">
        Learn more <ArrowUpRight />
      </button>
    </div>
  );
}

export default InstinctsToSignals;
