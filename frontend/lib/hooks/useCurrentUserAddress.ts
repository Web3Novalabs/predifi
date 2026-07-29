"use client";

/**
 * Placeholder for the connected wallet address.
 *
 * The app doesn't have a wallet-connect integration wired up yet (no
 * Freighter / Stellar wallet hook exists in the codebase), so profile,
 * notification, and claim-status views need a single point to read the
 * "current user" from. Swap this implementation for a real wallet hook once
 * one lands — every consumer already goes through here.
 */
const PLACEHOLDER_ADDRESS = "GDRXJ7QVZK3F4P5N6M2H8L9C1B0A2E3D4F5G6H7J8K9L0M1N2P3Q4R5T7K4P";

export function useCurrentUserAddress(): string | undefined {
  return PLACEHOLDER_ADDRESS;
}
