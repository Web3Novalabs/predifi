import Navbar from "../(marketing)/components/Navbar";
import Footer from "../(marketing)/components/Footer";
import React from "react";
import { ArrowRight, Check } from "lucide-react";

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

const whyChoosePrediFi = [
  "Earn rewards for accurate predictions",
  "Create and manage your own prediction pools",
  "Trade predictions with other participants",
  "Fully decentralized and blockchain-verified",
  "No intermediaries or hidden fees",
  "Community-driven governance",
];

export default function AboutPage() {
  return (
    <div className="text-sm min-h-screen bg-[#001112]">
      <Navbar />

      <main className="w-screen overflow-x-hidden">
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

        {/* WHAT IS PREDIFI SECTION */}
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

        {/* HOW IT WORKS SECTION */}
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

        {/* WHY CHOOSE PREDIFI SECTION */}
        <section className="py-[60px] md:py-[100px] px-5">
          <div className="max-w-[1200px] mx-auto">
            <div className="flex flex-col md:flex-row gap-8 md:gap-12 items-center">
              {/* Left side - Image/Visual */}
              <div className="flex-1 flex justify-center">
                <div className="w-full max-w-[400px] h-[300px] md:h-[400px] rounded-[24px] bg-[#03353A4D] backdrop-blur-[15px] border border-[#37B7C322] flex items-center justify-center">
                  <div className="text-6xl">🎯</div>
                </div>
              </div>

              {/* Right side - Content */}
              <div className="flex-1">
                <h2 className="text-3xl md:text-4xl font-medium text-white mb-8">
                  Why Choose PrediFi?
                </h2>

                <div className="space-y-4">
                  {whyChoosePrediFi.map((benefit, index) => (
                    <div key={index} className="flex gap-4 items-start">
                      <Check className="w-6 h-6 text-[#37B7C3] flex-shrink-0 mt-0.5" />
                      <span className="text-base md:text-lg text-[#FFFFFFCC]">
                        {benefit}
                      </span>
                    </div>
                  ))}
                </div>

                <button className="mt-8 inline-flex items-center px-6 py-3 md:px-8 md:py-4 bg-[#37B7C3] text-black font-medium rounded-2xl hover:bg-[#2aa0ac] transition-colors">
                  Start Predicting Today
                  <ArrowRight size={20} className="ml-2" />
                </button>
              </div>
            </div>
          </div>
        </section>

        {/* CTA SECTION */}
        <section className="py-[60px] md:py-[100px] px-5">
          <div className="max-w-[1000px] mx-auto text-center">
            <div className="rounded-[24px] md:rounded-[33px] bg-gradient-to-r from-[#03353A66] to-[#03353A4D] backdrop-blur-[15px] p-8 md:p-12 border border-[#37B7C322]">
              <h2 className="text-3xl md:text-4xl font-medium text-white mb-6">
                Ready to Predict the Future?
              </h2>
              <p className="text-base md:text-lg text-[#FFFFFFCC] mb-8 leading-[140%]">
                Join thousands of predictors on PrediFi and start earning rewards for your insights.
              </p>

              <button className="inline-flex items-center px-8 py-4 md:px-10 md:py-5 bg-[#37B7C3] text-black font-semibold rounded-full hover:bg-[#2aa0ac] transition-colors text-lg">
                Explore Pools
                <ArrowRight size={22} className="ml-2" />
              </button>
            </div>
          </div>
        </section>

        <Footer />
      </main>
    </div>
  );
}
