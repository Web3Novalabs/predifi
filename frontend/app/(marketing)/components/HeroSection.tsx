import React from "react";
import Image from "next/image";

function HeroSection() {
  return (
    // Added min-h to ensure it covers screen on mobile, and overflow adjustments
    <section className="relative py-12 md:py-[105px] flex flex-col items-center text-center overflow-visible">
      {/* Background Pattern */}
      <Image
        src="/swirl-pattern.png"
        alt=""
        aria-hidden="true"
        fill
        className="absolute inset-0 w-full h-full object-cover pointer-events-none z-0"
      />

      {/* Main content */}
      <div className="relative z-10 flex flex-col items-center max-w-4xl px-5">
        <h1 className="max-w-[736px] font-medium text-[48px] leading-[110%] md:text-[80px] md:leading-[120%] -tracking-[0.05em] md:-tracking-[10%] bg-[linear-gradient(263.91deg,#CEFFF7_30.32%,#59B1A6_93.13%)] bg-clip-text text-transparent transition-all duration-300">
          Predict. Profit. Flip
        </h1>

        <p className="mt-4 md:mt-2 mb-8 md:mb-10 max-w-xl text-[#E0FFFB] text-base md:text-[18px]/[140%] tracking-[2%]">
          Create or join decentralized prediction pools and earn rewards for
          your insights.
        </p>

        <button className="mb-6 md:mb-[30px] rounded-full bg-[#37B7C3] px-8 py-3 md:px-[42px] md:py-[15px] text-lg md:text-xl font-semibold text-black transition hover:scale-[1.02] active:scale-[0.98] w-full md:w-auto max-w-[300px] md:max-w-fit">
          Explore Pool Market
        </button>

        <span className="inline-block rounded-full px-[13px] py-2 text-sm md:text-lg/[100%] text-white opacity-80 md:opacity-100">
          Powered by: Stellar
        </span>
      </div>

      {/* Stats Section
        Mobile: Positioned normally (static) with margin-top, vertical layout.
        Desktop: Positioned absolutely at bottom, horizontal layout.
      */}
      <div className="relative mt-12 w-full px-5 md:mt-0 md:absolute md:left-1/2 md:-translate-x-1/2 md:-bottom-[92px] z-20">
        <div className="flex flex-col md:flex-row items-center justify-center gap-y-20 md:gap-x-[70px] rounded-[24px] md:rounded-[14px] bg-[#001518] md:bg-[#00262A66] px-6 py-10 md:py-6 backdrop-blur-none md:backdrop-blur-[14px] shadow-2xl md:shadow-lg w-full max-w-[350px] md:max-w-fit mx-auto border border-[#ffffff0d] md:border-none">
          <Stat label="Prediction Accuracy" value="99%" />
          <Stat label="Amount Predicted" value="$44k+" />
          {/* Hidden on mobile if you want to match the screenshot exactly (which only showed 3 items), 
              but kept here for completeness. You can add 'hidden md:block' to hide specific ones. */}
          <Stat label="Total Coin Flips" value="5k+" />
          <Stat label="Active Prediction Pools" value="2k+" />
        </div>
      </div>
    </section>
  );
}

export default HeroSection;

function Stat({ value, label }: { value: string; label: string }) {
  return (
    <div className="w-full md:w-[219px] text-center space-y-1 md:space-y-[5px]">
      <h3 className="text-[32px] md:text-[28px] font-semibold bg-[linear-gradient(180deg,#F2FFFD_-18.66%,#009886_136.06%)] bg-clip-text text-transparent">
        {value}
      </h3>
      <p className="text-[#B3CECB] text-sm md:text-sm font-medium tracking-wide">
        {label}
      </p>
    </div>
  );
}
