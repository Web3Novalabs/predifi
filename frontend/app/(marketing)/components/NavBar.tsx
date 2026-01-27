"use client";
import Link from "next/link";
import React, { useState } from "react";
import Image from "next/image";
import { Menu, X } from "lucide-react";

function Navbar() {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <nav
      className="bg-[#000A0B] relative z-50"
      style={{ boxShadow: "0px 20px 25px 0px #000A0B" }}
    >
      <div className="flex justify-between items-center p-5 md:px-[50px]">
        {/* LOGO */}
        <Link href="/">
          <Image src="/logo.svg" className="w-[80px] md:w-[100px] h-auto" alt="Logo" width={100} height={40} />
        </Link>

        {/* DESKTOP NAVIGATION (Hidden on mobile) */}
        <div className="hidden md:flex rounded-full p-4 border-[0.5px] border-[#EBFDFF99] text-[#DDDDDD99] items-center gap-x-[60px]">
          <Link href="/features" className="hover:text-white transition-colors">
            FEATURES
          </Link>
          <Link href="/benefits" className="hover:text-white transition-colors">
            BENEFITS
          </Link>
          <Link href="/faqs" className="hover:text-white transition-colors">
            FAQS
          </Link>
          <Link
            href="/community"
            className="hover:text-white transition-colors"
          >
            COMMUNITY
          </Link>
        </div>

        {/* DESKTOP CTA BUTTON (Hidden on mobile) */}
        <button className="hidden md:block py-[10px] px-5 rounded-2xl bg-[#37B7C3] text-black font-medium hover:bg-[#2aa0ac] transition-colors">
          Explore Pools
        </button>

        {/* MOBILE MENU TOGGLE (Visible on mobile) */}
        <button
          onClick={() => setIsOpen(!isOpen)}
          className="md:hidden text-white focus:outline-none p-2"
        >
          {isOpen ? <X size={22} /> : <Menu size={22} />}
        </button>
      </div>

      {/* MOBILE DROPDOWN MENU */}
      {isOpen && (
        <div className="md:hidden absolute top-full left-0 w-full bg-black border-t border-[#EBFDFF20] flex flex-col items-center py-8 space-y-6 shadow-2xl animate-in slide-in-from-top-5 fade-in duration-200">
          <Link
            href="/features"
            className="text-[#DDDDDD99] text-lg hover:text-white transition-colors"
            onClick={() => setIsOpen(false)}
          >
            FEATURES
          </Link>
          <Link
            href="/benefits"
            className="text-[#DDDDDD99] text-lg hover:text-white transition-colors"
            onClick={() => setIsOpen(false)}
          >
            BENEFITS
          </Link>
          <Link
            href="/faqs"
            className="text-[#DDDDDD99] text-lg hover:text-white transition-colors"
            onClick={() => setIsOpen(false)}
          >
            FAQS
          </Link>
          <Link
            href="/community"
            className="text-[#DDDDDD99] text-lg hover:text-white transition-colors"
            onClick={() => setIsOpen(false)}
          >
            COMMUNITY
          </Link>

          <button className="py-[10px] px-8 rounded-2xl bg-[#37B7C3] text-black font-medium w-fit">
            Explore Pools
          </button>
        </div>
      )}
    </nav>
  );
}

export default Navbar;
