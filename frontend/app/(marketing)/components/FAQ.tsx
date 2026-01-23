"use client";
import { ChevronDown } from "lucide-react";
import React, { useState } from "react";

// 1. Define the shape of your data
interface FAQItem {
  question: string;
  answer: string;
}

// 2. Define the props for the child component
interface AccordionItemProps {
  question: string;
  answer: string;
  isOpen: boolean;
  onClick: () => void;
}

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

function AccordionItem({
  question,
  answer,
  isOpen,
  onClick,
}: AccordionItemProps) {
  return (
    <div className="border border-[#FFFFFF1A] rounded-[12px] overflow-hidden transition-all duration-300">
      <button
        onClick={onClick}
        className="w-full flex items-center justify-between p-4 text-left focus:outline-none group"
      >
        <span className="text-sm md:text-[18px] font-medium text-[#FFFFFFCC] group-hover:text-white transition-colors">
          {question}
        </span>

        {/* Chevron Icon */}
        <ChevronDown
          className={`w-5 h-5 text-[#FFFFFF80] transition-transform duration-300 ${
            isOpen ? "rotate-180" : "rotate-0"
          }`}
        />
      </button>

      {/* Answer Section with smooth transition */}
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
}

function FAQ() {
  // Explicitly typing state as number or null
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  const handleToggle = (index: number) => {
    setOpenIndex(openIndex === index ? null : index);
  };

  return (
    <div className="py-[100px] px-5 md:px-[75px]">
      <div className="bg-[#FFFFFF0D] backdrop-blur-sm p-4 md:p-[37px] mx-auto border border-[#FFFFFF0D]">
        <h3 className="mb-[40px] md:mb-[60px] font-medium text-[24px] md:text-[30px]/[100%] -tracking-[6%] text-white">
          Frequently Asked Questions
        </h3>

        <div className="flex flex-col gap-4">
          {faqData.map((item, index) => (
            <AccordionItem
              key={index}
              question={item.question}
              answer={item.answer}
              isOpen={openIndex === index}
              onClick={() => handleToggle(index)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

export default FAQ;
