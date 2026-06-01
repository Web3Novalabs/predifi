import React from "react";
import Image from "next/image";
import SocialIcon from "@/components/ui/SocialIcon";

function Footer() {
  return (
    <div className="rounded-t-[40px] px-10 lg:px-20 py-10 lg:py-[50px] bg-[#FFFFFF0D]">
      <div className="pb-5 border-b-[#CBECEF] lg:flex-row flex-col-reverse gap-y-8 items-center border-b-[0.5px] flex justify-between lg:items-baseline">
        <Image src="/logo.svg" width={100} height={32} alt="PrediFi logo" />
        <div className="flex gap-x-[34px]">
          {/* Social icons are served from a single SVG sprite (/sprite.svg)
              so the browser makes one request instead of four. */}
          <SocialIcon id="telegram" label="Telegram" className="w-6 h-6" />
          <SocialIcon id="reddit" label="Reddit" className="w-6 h-6" />
          <SocialIcon id="x" label="X (Twitter)" className="w-6 h-6" />
          <SocialIcon id="discord" label="Discord" className="w-6 h-6" />
        </div>
      </div>

      <div className="flex gap-y-3  justify-between mt-10 text-[#758382] text-xs lg:text-sm font-medium">
        <h5>@copyright2025</h5>
        <h5>Designed by Zyrick</h5>
      </div>
    </div>
  );
}

export default Footer;
