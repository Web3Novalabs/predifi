"use client";
import React, { useState, useRef } from "react";

import Image from "next/image";

// define your content and images here
const steps = [
  {
    title: "Create or Join",
    description: "Launch your own market or join an existing one in seconds.",
    image: "/home-screen.svg",
  },
  {
    title: "Stake Your Insight",
    description:
      "Pick a side, lock your stake — outcomes are resolved trustlessly.",
    image: "/home-screen-2.svg", // Replace with distinct images if available
  },
  {
    title: "Flip and Earn",
    description: "If you're right, you earn. If not, you still gain insight.",
    image: "/home-screen-3.svg", // Replace with distinct images if available
  },
];

function PredictionProtocol() {
  const [activeTab, setActiveTab] = useState(0);

  // Touch handling for mobile swipe
  const touchStartX = useRef<number | null>(null);
  const touchEndX = useRef<number | null>(null);

  const handleTouchStart = (e: React.TouchEvent) => {
    touchStartX.current = e.targetTouches[0].clientX;
  };

  const handleTouchMove = (e: React.TouchEvent) => {
    touchEndX.current = e.targetTouches[0].clientX;
  };

  const handleTouchEnd = () => {
    if (!touchStartX.current || !touchEndX.current) return;
    const distance = touchStartX.current - touchEndX.current;
    const isLeftSwipe = distance > 50;
    const isRightSwipe = distance < -50;

    if (isLeftSwipe) {
      setActiveTab((prev) => (prev + 1) % steps.length);
    }
    if (isRightSwipe) {
      setActiveTab((prev) => (prev - 1 + steps.length) % steps.length);
    }

    // Reset
    touchStartX.current = null;
    touchEndX.current = null;
  };

  return (
    <div className="px-5 overflow-hidden">
      <h1 className="max-w-[558px] text-center mb-[40px] md:mb-[52px] text-white text-[28px] md:text-[48px] leading-[120%] -tracking-[9%] font-medium mx-auto">
        A decentralized Prediction Protocol
      </h1>

      <div className="flex flex-col md:flex-row items-center md:items-start md:justify-center gap-y-8 md:gap-x-[73px]">
        {/* === RIGHT SIDE (Image) - Ordered First on Mobile === */}
        <div
          className="order-1 md:order-2 flex-shrink-0 w-full md:w-auto flex justify-center"
          onTouchStart={handleTouchStart}
          onTouchMove={handleTouchMove}
          onTouchEnd={handleTouchEnd}
        >
          <div className="relative w-full max-w-[320px] md:max-w-none">
            <Image
              key={activeTab} // Key forces re-render for animation
              src={steps[activeTab].image}
              alt={steps[activeTab].title}
              className="animate-fade-in w-full h-auto md:h-[700px] object-contain drop-shadow-2xl"
              draggable={false}
              width={500}
              height={700}
              style={{ width: '100%', height: 'auto' }}
            />
          </div>
        </div>

        {/* === LEFT SIDE (Content) - Ordered Second on Mobile === */}
        <div className="order-2 md:order-1 flex flex-col items-center md:items-start gap-y-6 md:gap-y-10 max-w-[700px] w-full">
          {/* MOBILE: Pagination Dots */}
          <div className="flex md:hidden gap-3 mb-2">
            {steps.map((_, index) => (
              <button
                key={index}
                onClick={() => setActiveTab(index)}
                className={`w-2.5 h-2.5 rounded-full transition-all duration-300 ${activeTab === index ? "bg-[#37B7C3] w-6" : "bg-[#FFFFFF33]"
                  }`}
                aria-label={`Go to slide ${index + 1}`}
              />
            ))}
          </div>

          {/* MOBILE: Centered Text (Only shows active step) */}
          <div className="block md:hidden text-center animate-fade-in">
            <h4 className="text-[24px] leading-[100%] font-medium text-white mb-3">
              {steps[activeTab].title}
            </h4>
            <p className="text-lg text-gray-300 px-4">
              {steps[activeTab].description}
            </p>
          </div>

          {/* DESKTOP: List of Buttons (Hidden on mobile) */}
          <div className="hidden md:flex flex-col gap-y-10">
            {steps.map((step, index) => {
              const isActive = activeTab === index;
              return (
                <button
                  key={index}
                  onClick={() => setActiveTab(index)}
                  className={`
                    px-6 py-2 space-y-2 text-left relative transition-opacity duration-300 ease-in-out
                    ${isActive ? "opacity-100" : "opacity-50 hover:opacity-100"}
                  `}
                >
                  {/* Gradient Border Line */}
                  {isActive && (
                    <span className="absolute left-0 top-0 h-full w-[3.66px] bg-[linear-gradient(180deg,#828282_0%,#1C1C1C_100%)] rounded-full animate-fade-in" />
                  )}

                  <h4 className="text-[30px] leading-[100%] tracking-0 font-medium text-white">
                    {step.title}
                  </h4>
                  <p className="text-xl text-gray-300 max-w-[450px]">
                    {step.description}
                  </p>
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}

export default PredictionProtocol;
