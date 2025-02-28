"use client";
import Image from "next/image";
import img from "@/public/avatar.png";
import Clip from "@/svg/clip";
import Link from "next/link";
import Edit from "@/svg/edit";
import { addressSlice } from "@/lib/helper";
import { useAccount } from "@starknet-react/core";
import { useContext, useState } from "react";
import ChevronDown from "@/svg/chevron-down";
import { FilterContext } from "@/context/filter-context-provider";
import { Toaster } from "@/components/ui/sonner";
function ProfileLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const { setGetType, getType } = useContext(FilterContext);
  const { address, isConnected } = useAccount();
  const [isOpen, setIsOpen] = useState(false);
  const user = isConnected ? addressSlice(address ?? "") : "Wallet address";
  const handleCopy = async () => {
    const url = window.location.href;
    try {
      await navigator.clipboard.writeText(url);
      //toast.success("Copied!");
      Toaster.success("Copied!")
    } catch (error) {
      //toast.error("Failed to copy!, try aagin");
      Toaster.error("Failed to copy!, try again")
      console.log(error);
    }
  };

  return (
    <>
      <div className="border-b border-[#373737] py-3">
        <h2 className="font-normal text-2xl">Profile</h2>
      </div>
      <div className="flex justify-between items-center w-full py-6">
        <div className="flex justify-between items-center w-full">
          <div className="flex justify-between items-center gap-10">
            <div className="w-[111px] h-[111px] ">
              <Image
                className="rounded-full"
                src={img}
                alt="user-avatar"
                width={111}
                height={111}
              />
            </div>
            <button
              type="button"
              className="flex justify-between items-center border-[#373737] border rounded-full py-2 sm:px-4 gap-2 text-[9px] sm:text-base px-2"
            >
              <span>{user}</span>
              <div onClick={handleCopy}>
                <Clip />
              </div>
            </button>
          </div>
          <div className="flex justify-between items-center gap-10 text-sm sm:text-base">
            <div className="flex justify-between items-center flex-col">
              <span>Total bet</span>
              <span>17</span>
            </div>
            <div className="flex justify-between items-center flex-col">
              <span>Wins</span>
              <span>10</span>
            </div>
            <div className="flex justify-between items-center flex-col">
              <span>Loss</span>
              <span>7</span>
            </div>
          </div>
        </div>
      </div>
      <div className="flex justify-between items-center md:border-b border-[#373737] mb-12 mt-4">
        <div className="relative  md:hidden border-[#373737] border rounded-full h-[30px] w-[150px] px-2">
          <select
            name=""
            id=""
            className="border-none bg-inherit w-full absolute z-10 profile-select capitalize"
            onMouseDown={() => setIsOpen((prev) => !prev)}
          >
            <option value="active pool" className="bg-[#373737]">
              active pool
            </option>
            <option value="active pool" className="bg-[#373737]">
              my pools
            </option>
            <option value="active pool" className="bg-[#373737]">
              Claiming
            </option>
          </select>
          <span
            className={`${
              isOpen ? "-rotate-180" : "rotate-0"
            } transition-all duration-500 absolute right-3 top-2`}
          >
            <ChevronDown />
          </span>
        </div>
        <div className="hidden justify-between items-center gap-3 md:flex ">
          <div
            className={`bg-[#373737] w-[170px] h-[43px] grid place-content-center align-middle rounded-t-full cursor-pointer${
              getType === "active" ? "bg-[#FFFFFF75]" : "bg-[#373737]"
            }`}
            onClick={() => setGetType("active")}
          >
            Active pools
          </div>
          <div className="bg-[#373737] w-[170px] h-[43px] grid place-content-center align-middle rounded-t-full">
            wins
          </div>
          <div className="bg-[#373737] w-[170px] h-[43px] grid place-content-center align-middle rounded-t-full">
            vote in
          </div>
          <div
            className={`bg-[#373737] w-[170px] h-[43px] grid place-content-center align-middle rounded-t-full cursor-pointer ${
              getType === "claim" ? "bg-[#FFFFFF75]" : "bg-[#373737]"
            }`}
            onClick={() => setGetType("claim")}
          >
            Claiming
          </div>
        </div>
        <Link href="#" className="flex gap-3 items-center mb-3">
          Edit profile
          <Edit />
        </Link>
      </div>
      {children}

      
    
    </>
  );
}
export default ProfileLayout;
