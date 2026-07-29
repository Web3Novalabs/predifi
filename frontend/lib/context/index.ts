/**
 * Context barrel — re-exports all app-wide React contexts and their hooks.
 *
 * Always import contexts from here rather than individual files:
 *   import { useWallet, WalletProvider, useWalletAddress } from "@/lib/context";
 *   import { useThemeContext, ThemeProvider } from "@/lib/context";
 *
 * Exception: useCurrentUserAddress imports WalletContext directly to avoid a
 * circular dependency (barrel → WalletContext → barrel).
 */

export {
  WalletContext,
  WalletProvider,
  useWallet,
  useWalletAddress,
  type WalletConnectionState,
} from "./WalletContext";

export {
  ThemeContext,
  ThemeProvider,
  useThemeContext,
  type ThemeContextValue,
} from "./ThemeContext";
