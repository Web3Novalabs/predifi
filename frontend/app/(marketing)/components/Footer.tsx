import React from "react";

function Footer() {
  return (
    <div className="rounded-t-[40px] px-10 lg:px-20 py-10 lg:py-[50px] bg-[#FFFFFF0D]">
      <div className="pb-5 border-b-[#CBECEF] lg:flex-row flex-col-reverse gap-y-8 items-center border-b-[0.5px] flex justify-between lg:items-baseline">
        <img src="/logo.svg" alt="" />
        <div className="flex gap-x-[34px]">
          <img src="/socials/telegram.svg" className="w-6 h-6" alt="" />
          <img src="/socials/reddit.svg" className="w-6 h-6" alt="" />
          <img src="/socials/x.svg" className="w-6 h-6" alt="" />
          <img src="/socials/discord.svg" className="w-6 h-6" alt="" />
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
