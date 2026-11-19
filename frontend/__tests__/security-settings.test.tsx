import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { SecuritySettings } from "@/components/settings/SecuritySettings";
import {
  WalletContext,
  type WalletConnectionState,
} from "@/lib/context/WalletContext";

const mockAddToast = jest.fn();

jest.mock("@/components/ui/toast-provider", () => ({
  ToastProvider: ({ children }: { children: ReactNode }) => <>{children}</>,
  useToast: () => ({ toasts: [] }),
  useToastActions: () => ({ addToast: mockAddToast, removeToast: jest.fn() }),
}));

jest.mock("@/components/ui/copy-button", () => ({
  CopyButton: ({ text, "aria-label": label }: { text: string; "aria-label"?: string }) => (
    <button type="button" aria-label={label} data-copy={text} />
  ),
}));

const ADDRESS = "GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVW";

function wallet(overrides: Partial<WalletConnectionState> = {}): WalletConnectionState {
  return {
    address: ADDRESS,
    isConnecting: false,
    error: null,
    requiredNetworkLabel: "Testnet",
    connect: jest.fn().mockResolvedValue(ADDRESS),
    disconnect: jest.fn(),
    clearError: jest.fn(),
    ...overrides,
  };
}

function renderWith(value: WalletConnectionState) {
  return render(
    <WalletContext.Provider value={value}>
      <SecuritySettings />
    </WalletContext.Provider>,
  );
}

describe("SecuritySettings wallet section", () => {
  beforeEach(() => mockAddToast.mockClear());

  it("shows the truncated connected address with a copy button", () => {
    renderWith(wallet());

    expect(screen.getByTestId("wallet-address")).toHaveTextContent("GABCDE…TUVW");
    expect(screen.queryByText(/0x0000/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copy wallet address" })).toHaveAttribute(
      "data-copy",
      ADDRESS,
    );
  });

  it("disconnects through the wallet context and shows a toast", () => {
    const value = wallet();
    renderWith(value);

    fireEvent.click(screen.getByRole("button", { name: "Disconnect" }));

    expect(value.disconnect).toHaveBeenCalledTimes(1);
    expect(mockAddToast).toHaveBeenCalledWith(
      expect.objectContaining({ title: "Wallet disconnected" }),
    );
  });

  it("shows a connect prompt when no wallet is connected", () => {
    const value = wallet({ address: null });
    renderWith(value);

    expect(screen.getByText("No wallet connected")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Disconnect" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Connect wallet" }));
    expect(value.connect).toHaveBeenCalledTimes(1);
  });
});
