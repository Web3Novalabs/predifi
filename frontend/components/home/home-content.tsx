"use client";
import PredictionType from "@/components/predicton-type-detail";
import HowITWork from "@/components/how-it-work";
import PoolTypes from "@/components/pool-types";
import Link from "next/link";
import { routes } from "@/lib/route";

export default function HomeContent() {
  return (
    <div className="">
      <header className="flex flex-col items-center justify-center gap-5 p-6 header-bg h-[70vh]">
        <h1 className="font-jersey font-normal  text-white text-center text-5xl">
          Transform Predictions Into Profits!
        </h1>
        <p className="font-normal text-white text-center text-xl">
          Create and participate in decentralized prediction markets across
          sports, finance, and pop culture.
        </p>
        <div className="flex gap-x-2">
          <Link
            href={routes.createPool}
            className="bg-transparent rounded-full transition-all duration-200 hover:bg-[#37B7C3] hover:border-[#37B7C3] shadow-none border border-[#fff] text-[#fff] hover:text-[#071952] py-1 px-4"
          >
            Create a Pool
          </Link>
          <Link
            href={routes.dashboard}
            className="bg-transparent rounded-full transition-all duration-200 hover:bg-[#37B7C3] hover:border-[#37B7C3] shadow-none border border-[#fff] text-[#fff] hover:text-[#071952] px-4 py-1"
          >
            Explore Markets
          </Link>
        </div>
      </header>
      <section className="px-2 md:px-10 xl:px-16 my-[3em] lg:my-[6em]">
        <div className="text-center grid gap-1">
          <PredictionType />
          <HowITWork />
          <PoolTypes />
        </div>
      </section>
    </div>
  );
}
