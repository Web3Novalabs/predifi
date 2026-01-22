import React from "react";
import { ArrowRight } from "lucide-react";

const featuresData = [
  {
    title: "Game-like engagement",
    description:
      "Lorem ipsum dolor sit amet consectetur. Quis consectetur elementum dui aliquet nunc nulla. Purus amet porttitor vel senectus morbi pharetra a orci vestibulum.",
    image: "/feature-illus-1.svg",
    id: 1,
  },
  {
    title: "Zero trust, full transparency",
    description:
      "Lorem ipsum dolor sit amet consectetur. Quis consectetur elementum dui aliquet nunc nulla. Purus amet porttitor vel senectus morbi pharetra a orci vestibulum.",
    image: "/feature-illus-2.svg",
    id: 2,
  },
  {
    title: "Real on-chain outcomes",
    description:
      "Lorem ipsum dolor sit amet consectetur. Quis consectetur elementum dui aliquet nunc nulla. Purus amet porttitor vel senectus morbi pharetra a orci vestibulum.",
    image: "/feature-illus-3.svg",
    id: 3,
  },
];

function Features() {
  return (
    <div className="py-[60px] md:py-[100px] px-5">
      <p className="text-[24px] md:text-[48px] leading-[120%] mb-[50px] md:mb-[80px] max-w-[1000px] mx-auto -tracking-[0.04em] text-center text-[#D9D9D9] font-medium">
        Where your predictions shape the outcome, not just your luck.
      </p>

      <div className="flex flex-col gap-y-12 md:space-y-15 max-w-[1200px] mx-auto">
        {featuresData.map((feature, index) => {
          // Determine if this row should be reversed (Text Left, Image Right)
          // index 1 is the second item (0, 1, 2)
          const isReversed = index % 2 !== 0;

          return (
            <div
              key={feature.id}
              className={`
                flex items-center gap-8 md:gap-x-[70px]
                p-6 md:py-10 md:px-[80px]
                rounded-[24px] md:rounded-[33px]
                bg-[#03353A4D] backdrop-blur-[15px] flex-col-reverse
                w-full md:w-fit mx-auto
                /* Desktop: Row (normal or reversed). Mobile: Always Column */
                ${isReversed ? "md:flex-row-reverse" : "md:flex-row"}
              `}
            >
              {/* IMAGE */}
              <div className="flex-shrink-0">
                <img
                  src={feature.image}
                  className="w-full max-w-[180px] md:max-w-[400px] h-auto object-contain"
                  alt={feature.title}
                />
              </div>

              {/* CONTENT */}
              <div className="text-center md:text-left">
                <h3 className="mb-4 md:mb-[22px] text-xl md:text-2xl uppercase font-medium text-white">
                  {feature.title}
                </h3>
                <p className="mb-6 md:mb-10 max-w-[488px] text-base md:text-lg/[140%] tracking-[4%] text-[#FFFFFFCC] mx-auto md:mx-0">
                  {feature.description}
                </p>

                <button className="inline-flex items-center px-4 py-2.5 md:p-[10.35px] bg-[#37B7C31A] rounded-[16px] text-[#0AB4C4] hover:bg-[#37B7C333] transition-colors font-medium">
                  Learn more
                  <ArrowRight size={18} className="ml-2" />
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default Features;
