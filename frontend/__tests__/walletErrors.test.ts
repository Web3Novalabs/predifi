import {
  classifyWalletError,
  makeWalletError,
  unknownWalletError,
  checkNetworkMatch,
  isStellarWalletInstalled,
} from "@/lib/walletErrors";

describe("walletErrors", () => {
  describe("classifyWalletError", () => {
    it("maps a user-rejected error (EIP-1193 code 4001) to USER_REJECTED", () => {
      const err = { code: 4001, message: "User rejected the request" };
      const result = classifyWalletError(err);

      expect(result.code).toBe("USER_REJECTED");
      expect(result.title).toBe("Connection cancelled");
      expect(result.cause).toBe(err);
    });

    it("recognizes a user-rejected error identified only by its message text", () => {
      const result = classifyWalletError(
        new Error("User denied transaction signature"),
      );
      expect(result.code).toBe("USER_REJECTED");
    });

    it("maps a network-mismatch error to NETWORK_MISMATCH", () => {
      const result = classifyWalletError(new Error("Wrong network selected"));
      expect(result.code).toBe("NETWORK_MISMATCH");
      expect(result.title).toBe("Wrong network");
    });

    it("maps a missing-extension error to EXTENSION_NOT_INSTALLED", () => {
      const result = classifyWalletError(
        new Error("Freighter is not installed"),
      );
      expect(result.code).toBe("EXTENSION_NOT_INSTALLED");
    });

    it("maps a timeout error to TRANSACTION_TIMEOUT", () => {
      const result = classifyWalletError(new Error("Request timed out"));
      expect(result.code).toBe("TRANSACTION_TIMEOUT");
    });

    it("maps an insufficient-funds error to INSUFFICIENT_BALANCE", () => {
      const result = classifyWalletError(
        new Error("Insufficient balance for this operation"),
      );
      expect(result.code).toBe("INSUFFICIENT_BALANCE");
    });

    it("falls through to a generic UNKNOWN error for an unrecognized error", () => {
      const err = new Error("Something went sideways");
      const result = classifyWalletError(err);

      expect(result.code).toBe("UNKNOWN");
      expect(result.title).toBe("Wallet error");
      expect(result.message).toBe("Something went sideways");
      expect(result.recoveryAction).toBe(
        "Refresh the page and try connecting again.",
      );
    });

    it("falls through to a generic message when the error carries no usable message", () => {
      const result = classifyWalletError({ foo: "bar" });

      expect(result.code).toBe("UNKNOWN");
      expect(result.message).toBe("An unexpected wallet error occurred.");
    });
  });

  describe("makeWalletError", () => {
    it("builds a structured error from a known code, preserving the cause", () => {
      const cause = new Error("boom");
      const result = makeWalletError("INSUFFICIENT_BALANCE", cause);

      expect(result).toMatchObject({
        code: "INSUFFICIENT_BALANCE",
        title: "Insufficient balance",
        cause,
      });
    });
  });

  describe("unknownWalletError", () => {
    it("extracts a message from an Error cause", () => {
      expect(unknownWalletError(new Error("oops")).message).toBe("oops");
    });

    it("extracts a message from a string cause", () => {
      expect(unknownWalletError("plain string error").message).toBe(
        "plain string error",
      );
    });

    it("uses a generic message when the cause is neither an Error nor a string", () => {
      expect(unknownWalletError({ weird: true }).message).toBe(
        "An unexpected wallet error occurred.",
      );
    });
  });

  describe("checkNetworkMatch", () => {
    it("returns null when the wallet network matches the required network", () => {
      expect(checkNetworkMatch("Testnet")).toBeNull();
    });

    it("returns a NETWORK_MISMATCH error when the wallet is on a different network", () => {
      const result = checkNetworkMatch("Public");
      expect(result?.code).toBe("NETWORK_MISMATCH");
    });

    it("returns null when no wallet network has been reported yet", () => {
      expect(checkNetworkMatch(null)).toBeNull();
      expect(checkNetworkMatch(undefined)).toBeNull();
    });
  });

  describe("isStellarWalletInstalled", () => {
    afterEach(() => {
      delete (window as unknown as Record<string, unknown>).freighter;
      delete (window as unknown as Record<string, unknown>).freighterApi;
      delete (window as unknown as Record<string, unknown>).stellar;
    });

    it("returns false when no wallet globals are present", () => {
      expect(isStellarWalletInstalled()).toBe(false);
    });

    it("returns true when a wallet global is present", () => {
      (window as unknown as Record<string, unknown>).freighter = {};
      expect(isStellarWalletInstalled()).toBe(true);
    });
  });
});
