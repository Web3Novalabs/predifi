"use client";
import { ChevronDown } from "lucide-react";
import React, { useState, useCallback, memo } from "react";

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
  /**
   * Stable click handler scoped to this item.
   * useCallback here is valid — it is called at the top level of the component,
   * not inside a loop. `onToggle` is stable (parent useCallback), and `index`
   * is a primitive that never changes for a given list position.
   */
  const handleClick = useCallback(() => {
    onToggle(index);
  }, [onToggle, index]);

  return (
    <div className="border border-[#FFFFFF1A] rounded-[12px] overflow-hidden transition-all duration-300">
      <button
        onClick={handleClick}
        className="w-full flex items-center justify-between p-4 text-left focus:outline-none group"
      >
        <span className="text-sm md:text-[18px] font-medium text-[#FFFFFFCC] group-hover:text-white transition-colors">
          {question}
        </span>

        {/* Chevron rotates when the item is open */}
        <ChevronDown
          className={`w-5 h-5 text-[#FFFFFF80] transition-transform duration-300 ${
            isOpen ? "rotate-180" : "rotate-0"
          }`}
        />
      </button>

      {/* Answer — CSS grid trick for smooth height transition */}
      <div
        className={`grid transition-[grid-template-rows] duration-300 ease-out ${
          isOpen ? "grid-rows-[1fr] pb-6" : "grid-rows-[0fr]"
        }`}
      >
        <div className="overflow-hidden px-6">
          <p className="text-[#FFFFFF99] text-sm lg:text-base leading-relaxed">
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
      <div className="bg-[#FFFFFF0D] backdrop-blur-sm p-4 md:p-[37px] mx-auto border border-[#FFFFFF0D]">
        <h3 className="mb-[40px] md:mb-[60px] font-medium text-[24px] md:text-[30px]/[100%] -tracking-[6%] text-white">
          Frequently Asked Questions
        </h3>

        <div className="flex flex-col gap-4">
          {faqData.map((item, index) => (
            <AccordionItem
              key={item.question}
              index={index}
              question={item.question}
              answer={item.answer}
              isOpen={openIndex === index}
              onToggle={handleToggle}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

export default FAQ;
