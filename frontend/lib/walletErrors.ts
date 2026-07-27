/**
 * Wallet connection error taxonomy and user-facing recovery helpers.
 *
 * Covers: network mismatch, missing extension, user rejection,
 * transaction timeout, and insufficient balance.
 */

export type WalletErrorCode =
  | "NETWORK_MISMATCH"
  | "EXTENSION_NOT_INSTALLED"
  | "USER_REJECTED"
  | "TRANSACTION_TIMEOUT"
  | "INSUFFICIENT_BALANCE"
  | "UNKNOWN";

export interface WalletError {
  code: WalletErrorCode;
  /** Short title for toasts / modals */
  title: string;
  /** User-facing explanation */
  message: string;
  /** Concrete next step the user can take */
  recoveryAction: string;
  /** Original error when available */
  cause?: unknown;
}

/** Expected Stellar network for PrediFi (override via env). */
export type StellarNetwork = "TESTNET" | "PUBLIC" | "FUTURENET";

export const REQUIRED_STELLAR_NETWORK: StellarNetwork =
  (process.env.NEXT_PUBLIC_STELLAR_NETWORK as StellarNetwork) || "TESTNET";

export const STELLAR_NETWORK_LABELS: Record<StellarNetwork, string> = {
  TESTNET: "Stellar Testnet",
  PUBLIC: "Stellar Mainnet",
  FUTURENET: "Stellar Futurenet",
};

const WALLET_ERRORS: Record<
  Exclude<WalletErrorCode, "UNKNOWN">,
  Omit<WalletError, "code" | "cause">
> = {
  NETWORK_MISMATCH: {
    title: "Wrong network",
    message: `Your wallet is on a different network than PrediFi expects (${STELLAR_NETWORK_LABELS[REQUIRED_STELLAR_NETWORK]}).`,
    recoveryAction: `Switch your wallet to ${STELLAR_NETWORK_LABELS[REQUIRED_STELLAR_NETWORK]}, then try again.`,
  },
  EXTENSION_NOT_INSTALLED: {
    title: "Wallet not found",
    message:
      "No compatible wallet extension was detected. PrediFi needs Freighter (or another Stellar wallet) installed in your browser.",
    recoveryAction:
      "Install Freighter from https://freighter.app, refresh this page, then connect again.",
  },
  USER_REJECTED: {
    title: "Connection cancelled",
    message: "You rejected the wallet request in your extension.",
    recoveryAction: "Open your wallet and approve the connection when prompted.",
  },
  TRANSACTION_TIMEOUT: {
    title: "Request timed out",
    message:
      "The wallet did not respond in time. The network may be congested or the extension may be locked.",
    recoveryAction:
      "Unlock your wallet, check your connection, then retry the transaction.",
  },
  INSUFFICIENT_BALANCE: {
    title: "Insufficient balance",
    message:
      "Your account does not have enough funds (including fees) to complete this action.",
    recoveryAction:
      "Add funds to your wallet on the correct network, then try again.",
  },
};

export function makeWalletError(
  code: Exclude<WalletErrorCode, "UNKNOWN">,
  cause?: unknown
): WalletError {
  return { code, ...WALLET_ERRORS[code], cause };
}

export function unknownWalletError(cause?: unknown): WalletError {
  const msg =
    cause instanceof Error
      ? cause.message
      : typeof cause === "string"
        ? cause
        : "An unexpected wallet error occurred.";
  return {
    code: "UNKNOWN",
    title: "Wallet error",
    message: msg,
    recoveryAction: "Refresh the page and try connecting again.",
    cause,
  };
}

/** Common wallet / RPC error code shapes (Freighter, MetaMask-style, Stellar). */
function extractRawCode(err: unknown): string | number | undefined {
  if (!err || typeof err !== "object") return undefined;
  const e = err as Record<string, unknown>;
  if (typeof e.code === "string" || typeof e.code === "number") return e.code;
  if (e.error && typeof e.error === "object") {
    const nested = e.error as Record<string, unknown>;
    if (typeof nested.code === "string" || typeof nested.code === "number") {
      return nested.code;
    }
  }
  return undefined;
}

function extractMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    const m = (err as { message: unknown }).message;
    if (typeof m === "string") return m;
  }
  return "";
}

/**
 * Map a raw wallet/provider error into a structured {@link WalletError}.
 */
export function classifyWalletError(err: unknown): WalletError {
  const code = extractRawCode(err);
  const message = extractMessage(err).toLowerCase();

  // User rejection (EIP-1193 4001, Freighter cancel, etc.)
  if (
    code === 4001 ||
    code === "USER_REJECTED" ||
    code === "action_rejected" ||
    message.includes("user rejected") ||
    message.includes("user denied") ||
    message.includes("rejected by user") ||
    message.includes("user cancelled") ||
    message.includes("user canceled")
  ) {
    return makeWalletError("USER_REJECTED", err);
  }

  // Extension missing
  if (
    code === 4100 ||
    message.includes("no provider") ||
    message.includes("freighter is not installed") ||
    message.includes("wallet not found") ||
    message.includes("extension not") ||
    message.includes("provider not found")
  ) {
    return makeWalletError("EXTENSION_NOT_INSTALLED", err);
  }

  // Network mismatch
  if (
    code === 4902 ||
    message.includes("chain mismatch") ||
    message.includes("wrong network") ||
    message.includes("network mismatch") ||
    message.includes("incorrect network") ||
    message.includes("unsupported network")
  ) {
    return makeWalletError("NETWORK_MISMATCH", err);
  }

  // Timeout
  if (
    message.includes("timeout") ||
    message.includes("timed out") ||
    message.includes("deadline exceeded")
  ) {
    return makeWalletError("TRANSACTION_TIMEOUT", err);
  }

  // Insufficient balance / funds
  if (
    message.includes("insufficient") ||
    message.includes("not enough") ||
    message.includes("underfunded") ||
    message.includes("balance too low") ||
    message.includes("op_underfunded")
  ) {
    return makeWalletError("INSUFFICIENT_BALANCE", err);
  }

  return unknownWalletError(err);
}

/** Detect whether a Stellar wallet extension appears available. */
export function isStellarWalletInstalled(): boolean {
  if (typeof window === "undefined") return false;
  const w = window as Window & {
    freighter?: unknown;
    freighterApi?: unknown;
    stellar?: unknown;
  };
  return Boolean(w.freighter || w.freighterApi || w.stellar);
}

/**
 * Compare the wallet-reported network passphrase / name to the required one.
 * Returns a NETWORK_MISMATCH error when they differ.
 */
export function checkNetworkMatch(
  walletNetwork: string | null | undefined
): WalletError | null {
  if (!walletNetwork) return null;
  const normalized = walletNetwork.toUpperCase().replace(/\s+/g, "");
  const required = REQUIRED_STELLAR_NETWORK;
  const aliases: Record<StellarNetwork, string[]> = {
    TESTNET: ["TESTNET", "TESTNETNETWORK", "STELLARTESTNET"],
    PUBLIC: ["PUBLIC", "MAINNET", "PUBLICNETWORK", "STELLARPUBLIC"],
    FUTURENET: ["FUTURENET", "FUTURENETNETWORK"],
  };
  const ok = aliases[required].some((a) => normalized.includes(a));
  if (!ok) {
    return makeWalletError("NETWORK_MISMATCH", { walletNetwork, required });
  }
  return null;
}

/**
 * Wrap a wallet promise with a timeout; rejects with TRANSACTION_TIMEOUT.
 */
export async function withWalletTimeout<T>(
  promise: Promise<T>,
  timeoutMs = 60_000
): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timer = setTimeout(() => {
          reject(makeWalletError("TRANSACTION_TIMEOUT"));
        }, timeoutMs);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}
