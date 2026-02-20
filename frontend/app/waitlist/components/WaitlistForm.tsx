"use client";

import { useState, FormEvent } from "react";
import { Button, Input } from "@/components/ui";
import { CheckCircle2 } from "lucide-react";

export default function WaitlistForm() {
  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [status, setStatus] = useState<
    "idle" | "loading" | "success" | "error"
  >("idle");
  const [errorMessage, setErrorMessage] = useState("");

  const validateEmail = (email: string): boolean => {
    const re = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return re.test(email);
  };

  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    if (!name.trim()) {
      setStatus("error");
      setErrorMessage("Please enter your name");
      return;
    }

    if (!validateEmail(email)) {
      setStatus("error");
      setErrorMessage("Please enter a valid email address");
      return;
    }

    setStatus("loading");
    setErrorMessage("");

    // Simulate API call - replace with actual endpoint
    try {
      await new Promise((resolve) => setTimeout(resolve, 1500));
      setStatus("success");
      setName("");
      setEmail("");
      // eslint-disable-next-line
    } catch (_) {
      setStatus("error");
      setErrorMessage("Something went wrong. Please try again.");
    }
  };

  return (
    <section className="relative py-12 md:py-[105px] flex flex-col items-center text-center overflow-visible px-5">
      {/* Background Pattern */}
      <img
        src="/swirl-pattern.png"
        alt=""
        aria-hidden="true"
        className="absolute inset-0 w-full h-full object-cover pointer-events-none z-0"
      />

      {/* Main Content */}
      <div className="relative z-10 flex flex-col items-center max-w-2xl w-full">
        {/* Heading */}
        <h1 className="max-w-[736px] font-medium text-[48px] leading-[110%] md:text-[80px] md:leading-[120%] -tracking-[0.05em] md:-tracking-[10%] bg-[linear-gradient(263.91deg,#CEFFF7_30.32%,#59B1A6_93.13%)] bg-clip-text text-transparent mb-4 md:mb-6">
          Join the Waitlist
        </h1>

        {/* Early Access Message */}
        <div className="mb-8 md:mb-12 space-y-4">
          <p className="text-[#E0FFFB] text-base md:text-[18px]/[140%] tracking-[2%] max-w-xl">
            PrediFi is currently in <span className="font-semibold text-white">early access</span> and coming soon!
          </p>
          <p className="text-[#B3CECB] text-sm md:text-base tracking-[2%] max-w-lg">
            Be among the first to experience decentralized prediction markets. 
            Join our waitlist to get early access and stay updated on our launch.
          </p>
        </div>

        {/* Form Card */}
        <div className="w-full max-w-md text-left">
          {status !== "success" ? (
            <form onSubmit={handleSubmit} className="space-y-6">
              <div className="rounded-[24px] md:rounded-[33px] bg-[#03353A4D] backdrop-blur-[15px] p-6 md:py-10 md:px-8 space-y-6 border border-[#ffffff0d]">
                <div className="space-y-4">
                  <Input
                    id="name"
                    name="name"
                    type="text"
                    label="Full Name"
                    required
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="Enter your full name"
                    disabled={status === "loading"}
                    className="bg-[#001518] border-[#EBFDFF33] text-white placeholder:text-[#B3CECB] focus-visible:ring-[#37B7C3]"
                  />
                  <Input
                    id="email"
                    name="email"
                    type="email"
                    label="Email Address"
                    required
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    placeholder="Enter your email address"
                    disabled={status === "loading"}
                    className="bg-[#001518] border-[#EBFDFF33] text-white placeholder:text-[#B3CECB] focus-visible:ring-[#37B7C3]"
                  />
                </div>

                {status === "error" && (
                  <div className="rounded-lg bg-red-900/20 border border-red-800 p-3">
                    <p className="text-sm text-red-400 text-center">
                      {errorMessage}
                    </p>
                  </div>
                )}

                <Button
                  type="submit"
                  disabled={status === "loading"}
                  variant="primary"
                  size="large"
                  loading={status === "loading"}
                  className="w-full bg-[#37B7C3] text-black hover:bg-[#2aa0ac] font-semibold text-base md:text-lg"
                >
                  {status === "loading" ? "Joining Waitlist..." : "Join Waitlist"}
                </Button>
              </div>
            </form>
          ) : (
            <div className="rounded-[24px] md:rounded-[33px] bg-[#03353A4D] backdrop-blur-[15px] p-8 md:py-12 md:px-10 border border-[#37B7C3]/30 space-y-6">
              <div className="mx-auto w-16 h-16 bg-[#37B7C3] rounded-full flex items-center justify-center">
                <CheckCircle2 className="w-8 h-8 text-black" />
              </div>
              <div className="space-y-3">
                <h3 className="text-2xl md:text-3xl font-semibold text-white">
                  You&apos;re on the list!
                </h3>
                <p className="text-[#B3CECB] text-sm md:text-base">
                  Thank you for joining the waitlist. We&apos;ll notify you when 
                  PrediFi launches so you can be among the first to start predicting!
                </p>
              </div>
            </div>
          )}
        </div>

        {/* Additional Info */}
        <div className="mt-8 md:mt-12 text-[#758382] text-xs md:text-sm max-w-lg">
          <p>
            By joining the waitlist, you&apos;ll receive updates about our launch, 
            early access opportunities, and exclusive features.
          </p>
        </div>
      </div>
    </section>
  );
}
