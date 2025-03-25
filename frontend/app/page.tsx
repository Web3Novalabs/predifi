"use client"
import PredictionType from "@/components/predicton-type-detail";
import HowITWork from "@/components/how-it-work";
import PoolTypes from "@/components/pool-types";
import Link from "next/link";
import { routes } from "@/lib/route";
//import bgImg from "@/public/code.png"

export default function Home() {
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
          <Link href={routes.createPool} className="bg-transparent rounded-full transition-all duration-200 hover:bg-[#37B7C3] hover:border-[#37B7C3] shadow-none border border-[#fff] text-[#fff] hover:text-[#071952] py-1 px-4">
            Create a Pool
          </Link>
          <Link href={routes.dashboard} className="bg-transparent rounded-full transition-all duration-200 hover:bg-[#37B7C3] hover:border-[#37B7C3] shadow-none border border-[#fff] text-[#fff] hover:text-[#071952] px-4 py-1">
            Explore Markets
          </Link>
        </div>
      </header>
      <section className="px-2 md:px-10 xl:px-16 my-[3em] lg:my-[6em]">
        <div className="text-center grid gap-1">
          <h2 className="">Features overview</h2>
          <h1 className="font-jersey text-3xl">Why Choose PredFi?</h1>
        </div>
        <div className="mt-10">
          <PredictionType />
        </div>
      </section>
      <section className="px-2 md:px-10 xl:px-[100px]  py-[3em] lg:py-[6em]">
        <div className="text-center grid gap-1">
          <h2 className="">Step-by-step</h2>
          <h1 className="font-jersey text-3xl">How It Works</h1>
        </div>
        <div className="mt-5 lg:mt-10">
          <HowITWork />
        </div>
      </section>
      <section className="mt-16 px-5 md:px-10 xl:px-[100px]">
        <div className="text-center grid gap-1">
          <h2 className="">Step-by-step</h2>
          <h1 className="font-jersey text-3xl">Types of Prediction Pools</h1>
        </div>
        <div className="mt-10 lg:mt-[4em]">
          <PoolTypes />
        </div>
      </section>
      <section className="my-10 px-5 md:px-10 xl:px-[100px]">
        <div className="border  p-[3em] lg:p-[100px] rounded-lg">
          <h2 className="text-3xl font-normal font-jersey text-center mb-10">Site Metrics</h2>
          <div className="flex flex-col lg:flex-row justify-center gap-4">
            <div className="border-[#fff] w-full lg:w-[205px] h-[119px] border text-center grid place-content-center rounded-lg">
              <h2 className="text-sm font-semibold">Total Bets Open</h2>
              <h3 className="font-bold text-3xl">17</h3>
            </div>
            <div className="border-[#fff] w-full lg:w-[205px] h-[119px] border text-center grid place-content-center rounded-lg">
              <h2 className="text-sm font-semibold">Total Bets Open</h2>
              <h3 className="font-bold text-3xl">17</h3>
            </div>
            <div className="border-[#fff] w-full lg:w-[205px] h-[119px] border text-center grid place-content-center rounded-lg">
              <h2 className="text-sm font-semibold">Total Bets Open</h2>
              <h3 className="font-bold text-3xl">17</h3>
            </div>
          </div>
        </div>
      </section>
      <section className="gap-3 mt-10 px-5 md:px-10 xl:px-[100px]">
        <div className="lg:p-[100px] rounded-lg">
          <h2 className="text-sm font-semibold text-center">Don’t be loner</h2>
          <h2 className="text-3xl font-normal font-jersey text-center mb-5">Connect and Predict Together!</h2>
          <div className="flex justify-center gap-4">
            <Link href={routes.createPool} className="bg-transparent rounded-full transition-all duration-200 hover:bg-[#37B7C3] hover:border-[#37B7C3] shadow-none border  hover:text-[#071952] py-1 px-4">
              Twitter
            </Link>
            <Link href={routes.dashboard} className="bg-transparent rounded-full transition-all duration-200 hover:bg-[#37B7C3] hover:border-[#37B7C3] shadow-none border  hover:text-[#071952] px-4 py-1">
              Telegram
            </Link >
          </div>
        </div>
      </section>

    </div>
  );
}
