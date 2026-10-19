"use client";

/**
 * NotificationSettings
 *
 * Lets a connected user:
 *   1. Register / update their email address for email notifications (#1929).
 *   2. Toggle per-type email opt-in.
 *   3. Manage their interest list (categories/tags) to receive "new pool
 *      matching your interests" alerts.
 *
 * All API calls go to:
 *   PUT /api/v1/users/:address/email
 *   GET|PUT /api/v1/users/:address/interests
 */

import { useState, useEffect, useCallback } from "react";
import useSWR from "swr";
import { useWalletAddress } from "@/lib/context";
import {
  fetchUserEmail,
  upsertUserEmail,
  fetchInterests,
  setInterests,
  userEmailUrl,
  interestsUrl,
  type UserEmailPrefs,
} from "@/lib/api/notifications";
import { Skeleton } from "@/components/ui";
import { invalidateCache } from "@/lib/cache";

const CATEGORY_INTERESTS = [
  "Crypto",
  "Sports",
  "Politics",
  "Finance",
  "Entertainment",
  "Technology",
  "Science",
  "Gaming",
];

/** Stable empty list — avoids a new array identity on every render while loading. */
const EMPTY_INTERESTS: string[] = [];

export function NotificationSettings() {
  const address = useWalletAddress();

  // ── Email prefs ────────────────────────────────────────────────────────
  const emailKey = address ? userEmailUrl(address) : null;
  const { data: emailPrefs, isLoading: emailLoading } = useSWR<UserEmailPrefs | null>(
    emailKey,
    fetchUserEmail,
  );

  const [emailInput, setEmailInput] = useState("");
  const [emailEnabled, setEmailEnabled] = useState(true);
  const [emailSaving, setEmailSaving] = useState(false);
  const [emailSaved, setEmailSaved] = useState(false);
  const [emailError, setEmailError] = useState("");

  // Initialise fields once data loads
  useEffect(() => {
    if (emailPrefs) {
      setEmailInput(emailPrefs.email);
      setEmailEnabled(emailPrefs.email_enabled);
    }
  }, [emailPrefs]);

  const handleSaveEmail = useCallback(async () => {
    if (!address) return;
    setEmailSaving(true);
    setEmailError("");
    setEmailSaved(false);
    try {
      await upsertUserEmail(address, emailInput.trim(), emailEnabled);
      await invalidateCache("user");
      setEmailSaved(true);
      setTimeout(() => setEmailSaved(false), 3000);
    } catch (e) {
      setEmailError(e instanceof Error ? e.message : "Failed to save email");
    } finally {
      setEmailSaving(false);
    }
  }, [address, emailInput, emailEnabled]);

  // ── Interests ──────────────────────────────────────────────────────────
  const interestsKey = address ? interestsUrl(address) : null;
  const { data: savedInterests = EMPTY_INTERESTS, isLoading: interestsLoading } =
    useSWR<string[]>(interestsKey, fetchInterests);

  const [interests, setLocalInterests] = useState<string[]>([]);
  const [interestsSaving, setInterestsSaving] = useState(false);
  const [interestsSaved, setInterestsSaved] = useState(false);
  const [interestsError, setInterestsError] = useState("");

  useEffect(() => {
    setLocalInterests(savedInterests);
  }, [savedInterests]);

  const toggleInterest = useCallback((interest: string) => {
    setLocalInterests((prev) =>
      prev.includes(interest) ? prev.filter((i) => i !== interest) : [...prev, interest],
    );
  }, []);

  const handleSaveInterests = useCallback(async () => {
    if (!address) return;
    setInterestsSaving(true);
    setInterestsError("");
    setInterestsSaved(false);
    try {
      await setInterests(address, interests);
      await invalidateCache("user");
      setInterestsSaved(true);
      setTimeout(() => setInterestsSaved(false), 3000);
    } catch (e) {
      setInterestsError(e instanceof Error ? e.message : "Failed to save interests");
    } finally {
      setInterestsSaving(false);
    }
  }, [address, interests]);

  if (!address) {
    return (
      <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-6 text-center text-sm text-zinc-500">
        Connect your wallet to manage notification settings.
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* ── Email notifications ────────────────────────────────────────── */}
      <section className="rounded-xl border border-zinc-800 bg-zinc-900 p-5 space-y-4">
        <div>
          <h3 className="text-sm font-semibold text-white">Email Notifications</h3>
          <p className="text-xs text-zinc-500 mt-0.5">
            Receive alerts via email when pools end, resolve, or match your interests.
          </p>
        </div>

        {emailLoading ? (
          <div className="space-y-3">
            <Skeleton className="h-9 w-full rounded-md" />
            <Skeleton className="h-5 w-48" />
          </div>
        ) : (
          <div className="space-y-3">
            <div>
              <label
                htmlFor="notif-email"
                className="block text-xs font-medium text-zinc-400 mb-1"
              >
                Email address
              </label>
              <input
                id="notif-email"
                type="email"
                value={emailInput}
                onChange={(e) => setEmailInput(e.target.value)}
                placeholder="you@example.com"
                className="w-full rounded-md bg-zinc-800 border border-zinc-700 px-3 py-2 text-sm text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-[#37B7C3]/50"
              />
            </div>

            <label className="flex items-center gap-2.5 cursor-pointer">
              <input
                type="checkbox"
                checked={emailEnabled}
                onChange={(e) => setEmailEnabled(e.target.checked)}
                className="h-4 w-4 rounded border-zinc-700 bg-zinc-800 text-[#37B7C3] focus:ring-[#37B7C3]/50"
              />
              <span className="text-xs text-zinc-400">Enable email notifications</span>
            </label>

            {emailError && (
              <p className="text-xs text-red-400">{emailError}</p>
            )}

            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={handleSaveEmail}
                disabled={emailSaving || !emailInput.trim()}
                className="rounded-md bg-[#37B7C3] px-4 py-1.5 text-xs font-semibold text-black hover:bg-[#7DE3EC] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {emailSaving ? "Saving…" : "Save email"}
              </button>
              {emailSaved && (
                <span className="text-xs text-emerald-400">Saved ✓</span>
              )}
            </div>
          </div>
        )}
      </section>

      {/* ── Interests ─────────────────────────────────────────────────── */}
      <section className="rounded-xl border border-zinc-800 bg-zinc-900 p-5 space-y-4">
        <div>
          <h3 className="text-sm font-semibold text-white">Pool Interests</h3>
          <p className="text-xs text-zinc-500 mt-0.5">
            Get notified when new pools are created in categories or with tags you follow.
          </p>
        </div>

        {interestsLoading ? (
          <div className="flex flex-wrap gap-2">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-7 w-20 rounded-full" />
            ))}
          </div>
        ) : (
          <div className="space-y-3">
            <div className="flex flex-wrap gap-2" role="group" aria-label="Interest categories">
              {CATEGORY_INTERESTS.map((cat) => {
                const active = interests.includes(cat);
                return (
                  <button
                    key={cat}
                    type="button"
                    onClick={() => toggleInterest(cat)}
                    aria-pressed={active}
                    className={[
                      "rounded-full px-3 py-1 text-xs font-medium transition-colors",
                      active
                        ? "bg-[#37B7C3]/20 text-[#7DE3EC] ring-1 ring-[#37B7C3]/40"
                        : "bg-zinc-800 text-zinc-500 hover:text-zinc-300",
                    ].join(" ")}
                  >
                    {cat}
                  </button>
                );
              })}
            </div>

            {interestsError && (
              <p className="text-xs text-red-400">{interestsError}</p>
            )}

            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={handleSaveInterests}
                disabled={interestsSaving}
                className="rounded-md bg-[#37B7C3] px-4 py-1.5 text-xs font-semibold text-black hover:bg-[#7DE3EC] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {interestsSaving ? "Saving…" : "Save interests"}
              </button>
              {interestsSaved && (
                <span className="text-xs text-emerald-400">Saved ✓</span>
              )}
              <span className="text-xs text-zinc-600">
                {interests.length} selected
              </span>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
