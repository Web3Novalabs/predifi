"use client";
import React, { useState, useRef, useCallback, memo, useMemo } from "react";

// ---------------------------------------------------------------------------
// Data (module-level constant — never recreated)
// ---------------------------------------------------------------------------

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
    image: "/home-screen-2.svg",
  },
  {
    title: "Flip and Earn",
    description: "If you're right, you earn. If not, you still gain insight.",
    image: "/home-screen-3.svg",
  },
];

// ---------------------------------------------------------------------------
// StepButton — memoized desktop step button
//
// Extracted from the inline .map() so React.memo can prevent re-renders of
// inactive buttons when only the active index changes.
// The parent passes `onSelect` (a stable useCallback reference) and `index`
// as a primitive; the child creates its own stable handleClick.
// ---------------------------------------------------------------------------

interface StepButtonProps {
  index: number;
  title: string;
  description: string;
  isActive: boolean;
  /** Stable parent-level handler — parent must wrap with useCallback */
  onSelect: (index: number) => void;
}

const StepButton = memo(function StepButton({
  index,
  title,
  description,
  isActive,
  onSelect,
}: StepButtonProps) {
  /**
   * Stable click handler scoped to this step.
   * Called at the top level of the component (not inside a loop), so it
   * satisfies the Rules of Hooks. `onSelect` is stable (parent useCallback)
   * and `index` is a primitive that never changes for a given list position.
   */
  const handleClick = useCallback(() => {
    onSelect(index);
  }, [onSelect, index]);

  return (
    <button
      onClick={handleClick}
      className={`
        px-6 py-2 space-y-2 text-left relative transition-opacity duration-300 ease-in-out
        ${isActive ? "opacity-100" : "opacity-50 hover:opacity-100"}
      `}
    >
      {/* Gradient border line — only rendered for the active step */}
      {isActive && (
        <span className="absolute left-0 top-0 h-full w-[3.66px] bg-[linear-gradient(180deg,#828282_0%,#1C1C1C_100%)] rounded-full animate-fade-in" />
      )}
      <h4 className="text-[30px] leading-[100%] tracking-0 font-medium text-white">
        {title}
      </h4>
      <p className="text-xl text-gray-300 max-w-[450px]">{description}</p>
    </button>
  );
});

StepButton.displayName = "StepButton";

// ---------------------------------------------------------------------------
// DotButton — memoized mobile pagination dot
// ---------------------------------------------------------------------------

interface DotButtonProps {
  index: number;
  isActive: boolean;
  /** Stable parent-level handler — parent must wrap with useCallback */
  onSelect: (index: number) => void;
}

const DotButton = memo(function DotButton({
  index,
  isActive,
  onSelect,
}: DotButtonProps) {
  const handleClick = useCallback(() => {
    onSelect(index);
  }, [onSelect, index]);

  return (
    <button
      onClick={handleClick}
      className={`w-2.5 h-2.5 rounded-full transition-all duration-300 ${
        isActive ? "bg-[#37B7C3] w-6" : "bg-[#FFFFFF33]"
      }`}
      aria-label={`Go to slide ${index + 1}`}
    />
  );
});

DotButton.displayName = "DotButton";

// ---------------------------------------------------------------------------
// PredictionProtocol — parent component
//
// Memoization strategy:
//   - StepButton and DotButton are wrapped with React.memo so only the
//     button whose `isActive` prop changed re-renders on tab switch.
//   - handleSelect is wrapped with useCallback (no deps — uses functional
//     updater form) so its reference is stable across renders.
//   - Touch handlers are wrapped with useCallback so they don't cause
//     unnecessary re-renders of the image container div.
// ---------------------------------------------------------------------------

function PredictionProtocol() {
  const [activeTab, setActiveTab] = useState(0);

  const touchStartX = useRef<number | null>(null);
  const touchEndX = useRef<number | null>(null);

  /**
   * Stable tab selector — passed to both StepButton and DotButton.
   * No dependency on activeTab because it uses the direct value, not state.
   */
  const handleSelect = useCallback((index: number) => {
    setActiveTab(index);
  }, []);

  /** Records the X position where a touch gesture started. */
  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    touchStartX.current = e.targetTouches[0].clientX;
  }, []);

  /** Tracks the current X position as the finger moves. */
  const handleTouchMove = useCallback((e: React.TouchEvent) => {
    touchEndX.current = e.targetTouches[0].clientX;
  }, []);

  /**
   * Resolves the swipe direction and advances/retreats the active tab.
   * Uses the functional updater form so it has no dependency on activeTab.
   */
  const handleTouchEnd = useCallback(() => {
    if (!touchStartX.current || !touchEndX.current) return;
    const distance = touchStartX.current - touchEndX.current;

    if (distance > 50) {
      // Left swipe — advance
      setActiveTab((prev) => (prev + 1) % steps.length);
    } else if (distance < -50) {
      // Right swipe — retreat
      setActiveTab((prev) => (prev - 1 + steps.length) % steps.length);
    }

    touchStartX.current = null;
    touchEndX.current = null;
  }, []);

  /**
   * Memoized mobile pagination dots. Only re-calculates if activeTab changes.
   */
  const dotButtons = useMemo(() => (
    steps.map((_, index) => (
      <DotButton
        key={index}
        index={index}
        isActive={activeTab === index}
        onSelect={handleSelect}
      />
    ))
  ), [activeTab, handleSelect]);

  /**
   * Memoized desktop step buttons. Only re-calculates if activeTab changes.
   */
  const stepButtons = useMemo(() => (
    steps.map((step, index) => (
      <StepButton
        key={index}
        index={index}
        title={step.title}
        description={step.description}
        isActive={activeTab === index}
        onSelect={handleSelect}
      />
    ))
  ), [activeTab, handleSelect]);

  return (
    <div className="px-5 overflow-hidden">
      <h1 className="max-w-[558px] text-center mb-[40px] md:mb-[52px] text-white text-[28px] md:text-[48px] leading-[120%] -tracking-[9%] font-medium mx-auto">
        A decentralized Prediction Protocol
      </h1>

      <div className="flex flex-col md:flex-row items-center md:items-start md:justify-center gap-y-8 md:gap-x-[73px]">
        {/* === RIGHT SIDE (Image) — ordered first on mobile === */}
        <div
          className="order-1 md:order-2 flex-shrink-0 w-full md:w-auto flex justify-center"
          onTouchStart={handleTouchStart}
          onTouchMove={handleTouchMove}
          onTouchEnd={handleTouchEnd}
        >
          <div className="relative w-full max-w-[320px] md:max-w-none">
            <img
              key={activeTab} // key forces re-mount for the fade-in animation
              src={steps[activeTab].image}
              alt={steps[activeTab].title}
              width={400}
              height={700}
              className="animate-fade-in w-full h-auto md:h-[700px] object-contain drop-shadow-2xl"
              draggable={false}
            />
          </div>
        </div>

        {/* === LEFT SIDE (Content) — ordered second on mobile === */}
        <div className="order-2 md:order-1 flex flex-col items-center md:items-start gap-y-6 md:gap-y-10 max-w-[700px] w-full">
          {/* MOBILE: Pagination dots */}
          <div className="flex md:hidden gap-3 mb-2">
            {dotButtons}
          </div>

          {/* MOBILE: Active step text */}
          <div className="block md:hidden text-center animate-fade-in">
            <h4 className="text-[24px] leading-[100%] font-medium text-white mb-3">
              {steps[activeTab].title}
            </h4>
            <p className="text-lg text-gray-300 px-4">
              {steps[activeTab].description}
            </p>
          </div>

          {/* DESKTOP: Step buttons list */}
          <div className="hidden md:flex flex-col gap-y-10">
            {stepButtons}
          </div>
        </div>
      </div>
    </div>
  );
}

export default PredictionProtocol;
