import React from "react";

const missionPoints = [
  {
    icon: "🎯",
    title: "Decentralization at Core",
    description: "No central authority controlling outcomes. Every prediction lives on-chain.",
  },
  {
    icon: "🔒",
    title: "Trustless Verification",
    description: "Outcomes verified through smart contracts, not by intermediaries.",
  },
  {
    icon: "⚡",
    title: "Transparent & Fair",
    description: "Every transaction and result is verifiable on the blockchain.",
  },
  {
    icon: "🚀",
    title: "Built for Web3",
    description: "Native integration with StarkNet's scalability and security.",
  },
];

const predictionDomains = [
  {
    title: "Sports Predictions",
    description: "Predict outcomes of your favorite sports events and earn rewards.",
    icon: "🏆",
  },
  {
    title: "Financial Markets",
    description:
      "Make informed predictions about asset prices, market movements, and trends.",
    icon: "📈",
  },
  {
    title: "Global Events",
    description: "Predict real-world outcomes across politics, weather, and more.",
    icon: "🌍",
  },
];

export default function MissionAndDomainsSection() {
  return (
    <>
      {/* MISSION SECTION */}
      <section className="py-[60px] md:py-[100px] px-5">
        <div className="max-w-[1200px] mx-auto">
          <h2 className="text-3xl md:text-4xl font-medium text-white mb-[50px] md:mb-[80px] text-center">
            Our Mission
          </h2>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 md:gap-8">
            {missionPoints.map((point, index) => (
              <div
                key={index}
                className="rounded-[20px] bg-[#03353A4D] backdrop-blur-[15px] p-6 md:p-8 border border-[#FFFFFF0D] hover:border-[#37B7C333] transition-colors"
              >
                <div className="text-4xl mb-4">{point.icon}</div>
                <h3 className="text-xl md:text-2xl font-medium text-white mb-3">
                  {point.title}
                </h3>
                <p className="text-sm md:text-base text-[#FFFFFFCC] leading-[140%]">
                  {point.description}
                </p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* PREDICTION DOMAINS SECTION */}
      <section className="py-[60px] md:py-[100px] px-5">
        <div className="max-w-[1200px] mx-auto">
          <h2 className="text-3xl md:text-4xl font-medium text-white mb-[50px] md:mb-[80px] text-center">
            What Can You Predict?
          </h2>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 md:gap-8">
            {predictionDomains.map((domain, index) => (
              <div
                key={index}
                className="rounded-[20px] bg-gradient-to-br from-[#03353A4D] to-[#0A2F364D] backdrop-blur-[15px] p-6 md:p-8 border border-[#37B7C322]"
              >
                <div className="text-5xl mb-6">{domain.icon}</div>
                <h3 className="text-xl md:text-2xl font-medium text-white mb-3">
                  {domain.title}
                </h3>
                <p className="text-sm md:text-base text-[#FFFFFFCC] leading-[140%]">
                  {domain.description}
                </p>
              </div>
            ))}
          </div>
        </div>
      </section>
    </>
  );
}