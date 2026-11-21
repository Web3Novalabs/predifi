import React from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { WalletDropdown } from "@/components/WalletDropdown";
import { WalletErrorBanner } from "@/components/WalletErrorBanner";
import { WalletConnectionControl } from "@/components/WalletConnectionControl";
import { WalletContext, type WalletConnectionState } from "@/lib/context/WalletContext";
import type { WalletError } from "@/lib/walletErrors";

describe("Wallet Components and Connection Flow", () => {
  describe("WalletDropdown", () => {
    const mockOnDisconnect = jest.fn();
    const testAddress = "0x1234567890abcdef1234567890abcdef12345678";

    beforeEach(() => {
      jest.clearAllMocks();
    });

    it("renders 'Connect wallet' when address is null", () => {
      render(<WalletDropdown address={null} onDisconnect={mockOnDisconnect} />);
      expect(screen.getByText("Connect wallet")).toBeInTheDocument();
    });

    it("renders truncated address when connected", () => {
      render(<WalletDropdown address={testAddress} onDisconnect={mockOnDisconnect} />);
      expect(screen.getAllByText("0x...5678")[0]).toBeInTheDocument();
    });

    it("opens and closes dropdown menu on click", () => {
      render(<WalletDropdown address={testAddress} onDisconnect={mockOnDisconnect} />);
      const toggleBtn = screen.getByRole("button", { name: /0x...5678/i });

      // Initially closed
      expect(screen.getByRole("menu", { hidden: true })).toBeInTheDocument();

      // Open dropdown
      fireEvent.click(toggleBtn);
      expect(screen.getByRole("menu")).toBeVisible();

      // Disconnect button in menu
      const disconnectBtn = screen.getByRole("menuitem", { name: /Disconnect Wallet/i });
      fireEvent.click(disconnectBtn);
      expect(mockOnDisconnect).toHaveBeenCalledTimes(1);
    });

    it("handles copy address action", async () => {
      Object.assign(navigator, {
        clipboard: {
          writeText: jest.fn().mockImplementation(() => Promise.resolve()),
        },
      });

      render(<WalletDropdown address={testAddress} onDisconnect={mockOnDisconnect} />);
      const toggleBtn = screen.getByRole("button", { name: /0x...5678/i });
      fireEvent.click(toggleBtn);

      const copyBtn = screen.getByRole("menuitem", { name: /Copy Address/i });
      fireEvent.click(copyBtn);

      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith(testAddress);
      });
    });
  });

  describe("WalletErrorBanner", () => {
    const mockOnRetry = jest.fn();
    const mockOnDismiss = jest.fn();

    const sampleError: WalletError = {
      code: "NETWORK_MISMATCH",
      title: "Wrong Network",
      message: "Please switch to Stellar Futurenet",
      recoveryAction: "Switch network in Freighter extension",
    };

    it("renders error title, message, and recovery action", () => {
      render(
        <WalletErrorBanner
          error={sampleError}
          onRetry={mockOnRetry}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText("Wrong Network")).toBeInTheDocument();
      expect(
        screen.getByText("Please switch to Stellar Futurenet"),
      ).toBeInTheDocument();
      expect(
        screen.getByText("Switch network in Freighter extension"),
      ).toBeInTheDocument();
    });

    it("triggers onRetry and onDismiss callbacks when clicked", () => {
      render(
        <WalletErrorBanner
          error={sampleError}
          onRetry={mockOnRetry}
          onDismiss={mockOnDismiss}
        />
      );

      const retryBtn = screen.getByRole("button", { name: /Try again/i });
      const dismissBtn = screen.getByRole("button", { name: /Dismiss/i });

      fireEvent.click(retryBtn);
      expect(mockOnRetry).toHaveBeenCalledTimes(1);

      fireEvent.click(dismissBtn);
      expect(mockOnDismiss).toHaveBeenCalledTimes(1);
    });

    it("renders neutral recovery guidance when the user cancelled", () => {
      mockOnRetry.mockClear();
      const rejectionError: WalletError = {
        code: "USER_REJECTED",
        title: "Connection cancelled",
        message: "You rejected the wallet request in your extension.",
        recoveryAction: "Open your wallet and approve the connection when prompted.",
      };

      render(
        <WalletErrorBanner
          error={rejectionError}
          onRetry={mockOnRetry}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
      expect(screen.getByRole("status")).toBeInTheDocument();
      expect(screen.getByTestId("wallet-rejection-notice")).toBeInTheDocument();
      expect(
        screen.getByText("Open your wallet and approve the connection when prompted."),
      ).toBeInTheDocument();
      fireEvent.click(screen.getByRole("button", { name: /Try again/i }));
      expect(mockOnRetry).toHaveBeenCalledTimes(1);
    });

    it("renders error alert role for genuine wallet failures", () => {
      const failureError: WalletError = {
        code: "TRANSACTION_TIMEOUT",
        title: "Request timed out",
        message: "The wallet did not respond in time.",
        recoveryAction: "Unlock your wallet and retry.",
      };

      render(<WalletErrorBanner error={failureError} />);

      expect(screen.getByRole("alert")).toBeInTheDocument();
      expect(screen.getByText("Request timed out")).toBeInTheDocument();
    });
  });

  describe("WalletConnectionControl", () => {
    it("connects from the visible button and exposes error recovery", () => {
      const connect = jest.fn().mockResolvedValue(null);
      const walletState: WalletConnectionState = {
        address: null,
        isConnecting: false,
        error: {
          code: "EXTENSION_NOT_INSTALLED",
          title: "Wallet not found",
          message: "Install a compatible wallet extension.",
          recoveryAction: "Install Freighter, refresh, then connect again.",
        },
        requiredNetworkLabel: "Stellar Testnet",
        connect,
        disconnect: jest.fn(),
        clearError: jest.fn(),
      };

      render(
        <WalletContext.Provider value={walletState}>
          <WalletConnectionControl />
        </WalletContext.Provider>,
      );

      fireEvent.click(screen.getByRole("button", { name: /Connect wallet/i }));
      expect(connect).toHaveBeenCalledTimes(1);
      expect(
        screen.getByText("Install Freighter, refresh, then connect again."),
      ).toBeInTheDocument();
      fireEvent.click(screen.getByRole("button", { name: /Try again/i }));
      expect(connect).toHaveBeenCalledTimes(2);
    });
  });
});
