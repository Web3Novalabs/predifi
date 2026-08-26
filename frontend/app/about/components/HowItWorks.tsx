import React from "react";

export default function HowItWorksSection() {
  return (
    <section className="py-[60px] md:py-[100px] px-5">
      <div className="max-w-[1200px] mx-auto">
        <h2 className="text-3xl md:text-4xl font-medium text-white mb-[50px] md:mb-[80px] text-center">
          How It Works
        </h2>

        <div className="space-y-6 md:space-y-8">
          {[
            {
              step: 1,
              title: "Join or Create a Pool",
              description:
                "Browse existing prediction pools or create your own with custom parameters.",
            },
            {
              step: 2,
              title: "Make Your Prediction",
              description:
                "Place your prediction with stake, choosing from available outcome options.",
            },
            {
              step: 3,
              title: "Wait for Resolution",
              description:
                "Smart contracts monitor real-time data feeds and verify outcomes automatically.",
            },
            {
              step: 4,
              title: "Claim Your Rewards",
              description:
                "If your prediction is correct, rewards are automatically distributed on-chain.",
            },
          ].map((item) => (
            <div
              key={item.step}
              className="flex gap-6 md:gap-8 items-start rounded-[20px] bg-[#03353A4D] backdrop-blur-[15px] p-6 md:p-8 border border-[#FFFFFF0D]"
            >
              <div className="flex-shrink-0 w-12 h-12 md:w-16 md:h-16 rounded-full bg-[#37B7C3] flex items-center justify-center text-xl md:text-2xl font-bold text-black">
                {item.step}
              </div>
              <div className="flex-1">
                <h3 className="text-lg md:text-xl font-medium text-white mb-2">
                  {item.title}
                </h3>
                <p className="text-sm md:text-base text-[#FFFFFFCC] leading-[140%]">
                  {item.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}