"use client";
import { ChevronDown } from "lucide-react";
import React, { useState, useCallback, memo, useMemo } from "react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface FAQItem {
  question: string;
  answer: string;
}

interface AccordionItemProps {
  index: number;
  question: string;
  answer: string;
  isOpen: boolean;
  /**
   * Stable parent-level toggle handler. Receives the item index so the child
   * does not need its own closure over a changing value.
   * Parent wraps this with useCallback so React.memo can bail out correctly.
   */
  onToggle: (index: number) => void;
}

// ---------------------------------------------------------------------------
// Data (module-level constant — never recreated)
// ---------------------------------------------------------------------------

const faqData: FAQItem[] = [
  {
    question: "What is Flipnet?",
    answer:
      "Flipnet is a decentralized prediction market protocol that allows users to create markets, stake on outcomes, and earn rewards based on accurate predictions.",
  },
  {
    question: "How does the flipnet work?",
    answer:
      "Users can join existing markets or create new ones. Smart contracts handle the stakes and resolutions, ensuring a trustless and transparent process for all participants.",
  },
  {
    question: "Is this gambling?",
    answer:
      "Flipnet is a skill-based prediction market. While it involves staking assets on uncertain outcomes, it rewards research, insight, and market analysis rather than pure chance.",
  },
  {
    question: "How does the predifi pool works?",
    answer:
      "The liquidity pool ensures there is always a counterparty for trades. Liquidity providers earn fees from the trading volume generated within the pool.",
  },
];

// ---------------------------------------------------------------------------
// AccordionItem — memoized child component
//
// Wrapped with React.memo so it only re-renders when its own props change.
//
// Without memo, every FAQ state update (openIndex change) would re-render
// all four items even though only one actually changed.
//
// The parent passes `onToggle` (a stable useCallback reference) and `index`
// as a plain number. The child creates its own stable `handleClick` with
// useCallback so the button's onClick reference is also stable between renders.
// ---------------------------------------------------------------------------

const AccordionItem = memo(function AccordionItem({
  index,
  question,
  answer,
  isOpen,
  onToggle,
}: AccordionItemProps) {
  const handleClick = useCallback(() => {
    onToggle(index);
  }, [onToggle, index]);

  return (
    <div
      className={[
        "rounded-[12px] overflow-hidden",
        "border transition-[border-color,background-color,box-shadow] duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]",
        isOpen
          ? "border-[#37B7C3]/60 bg-[#37B7C3]/[0.06] shadow-[0_0_0_1px_rgba(55,183,195,0.15)]"
          : "border-[#FFFFFF1A] bg-transparent hover:border-[#FFFFFF30] hover:bg-white/[0.03]",
      ].join(" ")}
    >
      <button
        onClick={handleClick}
        aria-expanded={isOpen}
        className="w-full flex items-center justify-between p-4 md:p-5 text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-[#37B7C3]/50 focus-visible:ring-offset-2 focus-visible:ring-offset-transparent group"
      >
        <span
          className={[
            "text-sm md:text-[18px] font-medium transition-colors duration-200",
            isOpen ? "text-white" : "text-[#FFFFFFCC] group-hover:text-white",
          ].join(" ")}
        >
          {question}
        </span>

        {/* Chevron — smooth rotate + color shift */}
        <span
          className={[
            "ml-4 flex-shrink-0 rounded-full p-1",
            "transition-[transform,background-color,color] duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]",
            isOpen
              ? "rotate-180 bg-[#37B7C3]/20 text-[#37B7C3]"
              : "rotate-0 bg-transparent text-[#FFFFFF50]",
          ].join(" ")}
        >
          <ChevronDown className="w-4 h-4" />
        </span>
      </button>

      {/* Answer — CSS grid-rows trick for smooth height + opacity fade */}
      <div
        className={[
          "grid transition-[grid-template-rows] duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]",
          isOpen ? "grid-rows-[1fr]" : "grid-rows-[0fr]",
        ].join(" ")}
      >
        <div className="overflow-hidden">
          <p
            className={[
              "px-5 md:px-6 pb-5 md:pb-6 text-[#FFFFFF99] text-sm lg:text-base leading-relaxed",
              "transition-[opacity,transform] duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]",
              isOpen ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-1",
            ].join(" ")}
          >
            {answer}
          </p>
        </div>
      </div>
    </div>
  );
});

AccordionItem.displayName = "AccordionItem";

// ---------------------------------------------------------------------------
// FAQ — parent list component
//
// Memoization strategy:
//   - AccordionItem is wrapped with React.memo (above).
//   - handleToggle is wrapped with useCallback so its reference is stable
//     across re-renders. Without useCallback, a new function would be created
//     on every render, defeating React.memo's shallow-equality check and
//     causing all items to re-render on every click.
//   - The child receives `onToggle` + `index` (a primitive) instead of a
//     pre-bound `() => handleToggle(index)` arrow function. This avoids
//     creating new inline closures inside .map() which would also defeat memo.
// ---------------------------------------------------------------------------

function FAQ() {
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  /**
   * Stable toggle handler — memoized so AccordionItem's React.memo check
   * is not defeated by a new function reference on every render.
   * Uses the functional updater form so it has no dependency on `openIndex`.
   */
  const handleToggle = useCallback((index: number) => {
    setOpenIndex((prev) => (prev === index ? null : index));
  }, []); // setOpenIndex is stable — no deps needed

  return (
    <div className="py-[100px] px-5 md:px-[75px]">
      <div className="bg-[#FFFFFF0D] backdrop-blur-sm p-4 md:p-[37px] mx-auto max-w-[1000px] border border-[#FFFFFF0D]">
        <h3 className="mb-[40px] md:mb-[60px] font-medium text-[24px] md:text-[30px]/[100%] -tracking-[6%] text-white">
          Frequently Asked Questions
        </h3>

        <div className="flex flex-col gap-4">
          {useMemo(
            () =>
              faqData.map((item, index) => (
                <AccordionItem
                  key={item.question}
                  index={index}
                  question={item.question}
                  answer={item.answer}
                  isOpen={openIndex === index}
                  onToggle={handleToggle}
                />
              )),
            [openIndex, handleToggle],
          )}
        </div>
      </div>
    </div>
  );
}

export default FAQ;
