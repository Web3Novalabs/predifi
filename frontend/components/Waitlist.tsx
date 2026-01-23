"use client";

import { useState, FormEvent } from "react";

export default function Waitlist() {
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
    } catch (error) {
      setStatus("error");
      setErrorMessage("Something went wrong. Please try again.");
    }
  };

  return (
    <div className="min-h-screen bg-[#0a0a0a] flex items-center justify-center px-4 sm:px-6 lg:px-8">
      <div className="max-w-md w-full space-y-8">
        <div className="text-center">
          <h1 className="text-[#00D9D9] text-4xl font-bold tracking-tight">
            PrediFi
          </h1>
          <div className="mt-6">
            <h2 className="text-3xl font-bold text-white">Join the Waitlist</h2>
            <p className="mt-3 text-gray-400 text-base">
              Be among the first to experience decentralized prediction markets
            </p>
          </div>
        </div>

        {/* Form */}
        {status !== "success" ? (
          <form onSubmit={handleSubmit} className="mt-8 space-y-6">
            <div className="space-y-4">
              <div>
                <label htmlFor="name" className="sr-only">
                  Full Name
                </label>
                <input
                  id="name"
                  name="name"
                  type="text"
                  required
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="appearance-none relative block w-full px-4 py-3 border border-gray-700 bg-[#1a1a1a] placeholder-gray-500 text-white rounded-lg focus:outline-none focus:ring-2 focus:ring-[#00D9D9] focus:border-transparent transition-all"
                  placeholder="Full Name"
                  disabled={status === "loading"}
                />
              </div>
              <div>
                <label htmlFor="email" className="sr-only">
                  Email Address
                </label>
                <input
                  id="email"
                  name="email"
                  type="email"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  className="appearance-none relative block w-full px-4 py-3 border border-gray-700 bg-[#1a1a1a] placeholder-gray-500 text-white rounded-lg focus:outline-none focus:ring-2 focus:ring-[#00D9D9] focus:border-transparent transition-all"
                  placeholder="Email Address"
                  disabled={status === "loading"}
                />
              </div>
            </div>

            {status === "error" && (
              <div className="rounded-lg bg-red-900/20 border border-red-800 p-3">
                <p className="text-sm text-red-400 text-center">
                  {errorMessage}
                </p>
              </div>
            )}

            <button
              type="submit"
              disabled={status === "loading"}
              className="group relative w-full flex justify-center py-3 px-4 border border-transparent text-base font-medium rounded-lg text-black bg-[#00D9D9] hover:bg-[#00c4c4] focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-[#00D9D9] transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {status === "loading" ? "Joining..." : "Join Waitlist"}
            </button>
          </form>
        ) : (
          <div className="mt-8 rounded-lg bg-[#00D9D9]/10 border border-[#00D9D9] p-8 text-center space-y-4">
            <div className="mx-auto w-16 h-16 bg-[#00D9D9] rounded-full flex items-center justify-center">
              <svg
                className="w-8 h-8 text-black"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={3}
                  d="M5 13l4 4L19 7"
                />
              </svg>
            </div>
            <h3 className="text-2xl font-bold text-white">
              You're on the list!
            </h3>
            <p className="text-gray-400">
              We'll notify you when PrediFi launches. Get ready to predict!
            </p>
          </div>
        )}

        {/* Footer */}
        <p className="mt-8 text-center text-sm text-gray-500">
          Already have an account?{" "}
          <a
            href="/login"
            className="text-[#00D9D9] hover:text-[#00c4c4] font-medium transition-colors"
          >
            Sign in
          </a>
        </p>
      </div>
    </div>
  );
}
