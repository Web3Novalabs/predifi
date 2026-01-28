import React from "react";

export default function HeroAndIntroSection() {
  return (
    <>
      {/* HERO SECTION */}
      <section className="relative py-12 md:py-[105px] flex flex-col items-center text-center overflow-visible px-5">
        <img
          src="/swirl-pattern.png"
          alt=""
          aria-hidden="true"
          className="absolute inset-0 w-full h-full object-cover pointer-events-none z-0"
        />

        <div className="relative z-10 flex flex-col items-center max-w-4xl">
          <h1 className="max-w-[736px] font-medium text-[48px] leading-[110%] md:text-[80px] md:leading-[120%] -tracking-[0.05em] md:-tracking-[10%] bg-[linear-gradient(263.91deg,#CEFFF7_30.32%,#59B1A6_93.13%)] bg-clip-text text-transparent">
            About PrediFi
          </h1>

          <p className="mt-4 md:mt-6 mb-8 md:mb-10 max-w-2xl text-[#E0FFFB] text-base md:text-[18px]/[140%] tracking-[2%]">
            The decentralized prediction protocol that puts the power of predicting future outcomes directly in your hands.
          </p>
        </div>
      </section>

      {/* WHAT IS PREDIFI SECTION*/}
      <section className="py-[60px] md:py-[100px] px-5 max-w-[1200px] mx-auto">
        <div className="rounded-[24px] md:rounded-[33px] bg-[#03353A4D] backdrop-blur-[15px] p-6 md:py-10 md:px-[80px]">
          <h2 className="text-3xl md:text-4xl font-medium text-white mb-6 md:mb-8 text-center">
            What is PrediFi?
          </h2>
          <p className="text-base md:text-lg text-[#FFFFFFCC] text-center leading-[140%] tracking-[2%]">
            PrediFi is a decentralized prediction market built on StarkNet that combines the thrill of prediction with blockchain transparency. Users create or join prediction pools, make predictions about future outcomes across multiple domains, and earn rewards for accuracy—all verified on-chain with zero intermediaries.
          </p>
        </div>
      </section>
    </>
  );
}