import React from "react";
import Image from "next/image";
import Link from "next/link";

/**
 * Footer
 *
 * Static footer component rendered on marketing pages.
 * Wrapped with React.memo to prevent unnecessary re-renders since it has no props or state.
 */
const Footer = React.memo(function Footer() {
  return (
    <div className="rounded-t-[40px] px-5 md:px-10 lg:px-20 py-10 lg:py-[50px] bg-[#FFFFFF0D]">
      <div className="pb-5 border-b-[#CBECEF] flex flex-col-reverse gap-y-8 items-center border-b-[0.5px] md:flex-row md:justify-between md:items-baseline">
        <Image src="/logo.svg" width={100} height={32} alt="PrediFi logo" />
        <div className="flex gap-x-[34px]">
          <Link
            href="https://t.me/PrediFi"
            target="_blank"
            rel="noopener noreferrer"
            aria-label="PrediFi on Telegram"
            className="w-6 h-6"
          >
            <Image
              src="/socials/telegram.svg"
              width={24}
              height={24}
              alt=""
            />
          </Link>
          <Link
            href="https://www.reddit.com/r/PrediFi"
            target="_blank"
            rel="noopener noreferrer"
            aria-label="PrediFi on Reddit"
            className="w-6 h-6"
          >
            <Image
              src="/socials/reddit.svg"
              width={24}
              height={24}
              alt=""
            />
          </Link>
          <Link
            href="https://x.com/PrediFi"
            target="_blank"
            rel="noopener noreferrer"
            aria-label="PrediFi on X"
            className="w-6 h-6"
          >
            <Image
              src="/socials/x.svg"
              width={24}
              height={24}
              alt=""
            />
          </Link>
          <Link
            href="https://discord.gg/PrediFi"
            target="_blank"
            rel="noopener noreferrer"
            aria-label="PrediFi on Discord"
            className="w-6 h-6"
          >
            <Image
              src="/socials/discord.svg"
              width={24}
              height={24}
              alt=""
            />
          </Link>
        </div>
      </div>

      <div className="flex flex-col gap-y-3 justify-between mt-10 text-center text-[#758382] text-xs lg:text-sm font-medium md:flex-row md:text-left">
        <h5>@copyright2025</h5>
        <h5>Designed by Zyrick</h5>
      </div>
    </div>
  );
});

Footer.displayName = "Footer";

export default Footer;
