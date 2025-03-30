"use client";
import { useState } from "react";
import { useAccount, useDisconnect, useStarkName } from "@starknet-react/core";
import { addressSlice } from "@/lib/helper";
import Conectors from "../conectors";
import { Button } from "../ui/button";
import ChevronDown from "@/svg/chevron-down";
import Link from "next/link";
import { routes } from "@/lib/route";
import Image from "next/image";
import DarkModeToggle from "./DarkmodeButton";

function Nav() {
    const [openModal, setModal] = useState(false);
    const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
    const { address, isConnected } = useAccount();
    const { disconnect } = useDisconnect({});
    const user = isConnected ? addressSlice(address ?? "") : "Connect Wallet";

    const { data } = useStarkName({
        address,
    });

    function modalHandler() {
        setModal((prev) => !prev);
    }

    function toggleMobileMenu() {
        setMobileMenuOpen((prev) => !prev);
    }

    return (
        <>
            {openModal && !isConnected && (
                <Conectors setIsOpen={modalHandler} />
            )}
            <div className="relative py-4 px-4 md:px-10 xl:px-[100px]">
                <nav className="flex justify-between items-center relative">
                    {/* Logo */}
                    <Link href={routes.home} className="text-xl font-normal">
                        <Image
                            height={50}
                            width={50}
                            src="/logo.svg"
                            alt="logo"
                            className="max-h-12 max-w-12"
                        />
                    </Link>

                    {/* Desktop Navigation Menu */}
                    <ul className="hidden sm:flex justify-center items-center gap-4 lg:gap-8 capitalize flex-grow mx-4">
                        <li className="hover:text-primary transition-colors cursor-pointer">
                            Features
                        </li>
                        <li className="hover:text-primary transition-colors cursor-pointer">
                            How it works
                        </li>
                        <li className="hover:text-primary transition-colors cursor-pointer">
                            About
                        </li>
                    </ul>

                    {/* Right Side Actions */}
                    <div className="flex items-center gap-2 sm:gap-3">
                        {/* Dark Mode Toggle */}
                        <div className="hidden sm:block">
                            <DarkModeToggle />
                        </div>

                        {/* Wallet Connect Button */}
                        <Button
                            className="bg-transparent rounded-full hover:bg-transparent shadow-none border"
                            onClick={modalHandler}
                        >
                            {data ? data : user}
                            <span
                                className={`${
                                    openModal ? "-rotate-180" : "rotate-0"
                                } transition-all duration-500 ml-2`}
                            >
                                <ChevronDown />
                            </span>
                        </Button>

                        {/* Mobile Menu Toggle */}
                        <button
                            onClick={toggleMobileMenu}
                            className="sm:hidden focus:outline-none"
                        >
                            {mobileMenuOpen ? (
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    className="h-6 w-6"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                >
                                    <path
                                        strokeLinecap="round"
                                        strokeLinejoin="round"
                                        strokeWidth={2}
                                        d="M6 18L18 6M6 6l12 12"
                                    />
                                </svg>
                            ) : (
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    className="h-6 w-6"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                >
                                    <path
                                        strokeLinecap="round"
                                        strokeLinejoin="round"
                                        strokeWidth={2}
                                        d="M4 6h16M4 12h16M4 18h16"
                                    />
                                </svg>
                            )}
                        </button>
                    </div>

                    {/* Disconnect Wallet Button */}
                    {openModal && (
                        <Button
                            className={`fixed top-16 right-4 md:right-20 transition-all duration-500 text-[#37B7C3] border border-[#37B7C3] bg-inherit rounded-full hover:bg-transparent ${
                                isConnected ? "block" : "hidden"
                            }`}
                            onClick={() => {
                                disconnect();
                                setModal((prev) => !prev);
                            }}
                        >
                            Disconnect Wallet
                        </Button>
                    )}
                </nav>

                {/* Mobile Menu */}
                {mobileMenuOpen && (
                    <div className="sm:hidden absolute top-full left-0 w-full bg-white dark:bg-black dark:text-white shadow-lg z-50">
                        <ul className="flex flex-col items-center py-4 space-y-4 capitalize">
                            <li className="hover:text-primary transition-colors cursor-pointer">
                                Features
                            </li>
                            <li className="hover:text-primary transition-colors cursor-pointer">
                                How it works
                            </li>
                            <li className="hover:text-primary transition-colors cursor-pointer">
                                About
                            </li>
                            <li className="sm:hidden">
                                <DarkModeToggle />
                            </li>
                        </ul>
                    </div>
                )}
            </div>
        </>
    );
}

export default Nav;
