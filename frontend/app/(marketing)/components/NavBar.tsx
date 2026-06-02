"use client";
import Link from "next/link";
import Image from "next/image";
import React, { useState } from "react";
import { Menu, X } from "lucide-react";

/**
 * NavBar
 *
 * Primary site navigation rendered on every marketing page.
 *
 * All nav links use `prefetch={true}` so Next.js 15 eagerly prefetches the
 * full route payload as soon as the navbar enters the viewport. This overrides
 * the Next.js 15 default of lazy/null prefetching and ensures instant
 * navigation for the most critical pages in the app.
 */
function Navbar() {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <nav
      className="bg-[#000A0B] relative z-50"
      style={{ boxShadow: "0px 20px 25px 0px #000A0B" }}
    >
      <div className="flex justify-between items-center p-5 md:px-[50px]">
        {/* LOGO — home is the most critical destination; prefetch eagerly */}
        <Link href="/" prefetch={true}>
          <Image
            src="/logo.svg"
            alt="Logo"
            width={100}
            height={100}
            className="w-[80px] md:w-[100px]"
            priority
            loading="eager"
            fetchPriority="high"
          />
        </Link>

        {/* DESKTOP NAVIGATION (Hidden on mobile) */}
        <div className="hidden md:flex rounded-full p-4 border-[0.5px] border-[#EBFDFF99] text-[#DDDDDD99] items-center gap-x-[60px]">
          <Link href="/about" prefetch={true} className="hover:text-white transition-colors">
            ABOUT
          </Link>
          <Link href="/features" prefetch={true} className="hover:text-white transition-colors">
            FEATURES
          </Link>
          <Link href="/benefits" prefetch={true} className="hover:text-white transition-colors">
            BENEFITS
          </Link>
          <Link href="/faqs" prefetch={true} className="hover:text-white transition-colors">
            FAQS
          </Link>
          <Link
            href="/community"
            prefetch={true}
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
          {/*
           * Mobile links also get prefetch={true}. Although the mobile menu is
           * hidden until the hamburger is tapped, the links are rendered in the
           * DOM at that point and should prefetch immediately so tapping a link
           * feels instant.
           */}
          <Link
            href="/about"
            prefetch={true}
            className="text-[#DDDDDD99] text-lg hover:text-white transition-colors"
            onClick={() => setIsOpen(false)}
          >
            ABOUT
          </Link>
          <Link
            href="/features"
            prefetch={true}
            className="text-[#DDDDDD99] text-lg hover:text-white transition-colors"
            onClick={() => setIsOpen(false)}
          >
            FEATURES
          </Link>
          <Link
            href="/benefits"
            prefetch={true}
            className="text-[#DDDDDD99] text-lg hover:text-white transition-colors"
            onClick={() => setIsOpen(false)}
          >
            BENEFITS
          </Link>
          <Link
            href="/faqs"
            prefetch={true}
            className="text-[#DDDDDD99] text-lg hover:text-white transition-colors"
            onClick={() => setIsOpen(false)}
          >
            FAQS
          </Link>
          <Link
            href="/community"
            prefetch={true}
            className="text-[#DDDDDD99] text-lg hover:text-white transition-colors"
            onClick={() => setIsOpen(false)}
          >
            COMMUNITY
          </Link>

          <button className="py-[10px] px-8 rounded-2xl bg-[#37B7C3] text-black font-medium w-fit">
            Explore Pools
          </button>
        </div>
      </div>
    </nav>
  );
}

export default Navbar;
