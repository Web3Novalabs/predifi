import React from "react";
import { ArrowRight, Check } from "lucide-react";

const whyChoosePrediFi = [
  "Earn rewards for accurate predictions",
  "Create and manage your own prediction pools",
  "Trade predictions with other participants",
  "Fully decentralized and blockchain-verified",
  "No intermediaries or hidden fees",
  "Community-driven governance",
];

export default function BenefitsAndCTASection() {
  return (
    <>
      {/* WHY CHOOSE PREDIFI*/}
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
    </>
  );
}