import React from "react";
import Image from "next/image";

/**
 * Footer
 *
 * Static footer component rendered on marketing pages.
 * Wrapped with React.memo to prevent unnecessary re-renders since it has no props or state.
 */
const Footer = React.memo(function Footer() {
  return (
    <div className="rounded-t-[40px] px-10 lg:px-20 py-10 lg:py-[50px] bg-[#FFFFFF0D]">
      <div className="pb-5 border-b-[#CBECEF] lg:flex-row flex-col-reverse gap-y-8 items-center border-b-[0.5px] flex justify-between lg:items-baseline">
        <Image src="/logo.svg" width={100} height={32} alt="PrediFi logo" />
        <div className="flex gap-x-[34px]">
          <Image src="/socials/telegram.svg" width={24} height={24} className="w-6 h-6" alt="Telegram" />
          <Image src="/socials/reddit.svg" width={24} height={24} className="w-6 h-6" alt="Reddit" />
          <Image src="/socials/x.svg" width={24} height={24} className="w-6 h-6" alt="X (Twitter)" />
          <Image src="/socials/discord.svg" width={24} height={24} className="w-6 h-6" alt="Discord" />
        </div>
      </div>

      <div className="flex gap-y-3  justify-between mt-10 text-[#758382] text-xs lg:text-sm font-medium">
        <h5>@copyright2025</h5>
        <h5>Designed by Zyrick</h5>
      </div>
    </div>
  );
});

Footer.displayName = "Footer";

export default Footer;
