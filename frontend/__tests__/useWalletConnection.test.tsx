import { act, renderHook } from "@testing-library/react";
import { useWalletConnection } from "@/lib/hooks/useWalletConnection";

type TestFreighterApi = {
  getNetworkDetails?: jest.Mock;
  requestAccess?: jest.Mock;
};

function installWallet(api: TestFreighterApi) {
  (window as unknown as Record<string, unknown>).freighterApi = api;
}

describe("useWalletConnection", () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).freighterApi;
  });

  it("reports a missing wallet extension", async () => {
    const { result } = renderHook(() => useWalletConnection());

    await act(async () => {
      await result.current.connect();
    });

    expect(result.current.error).toMatchObject({
      code: "EXTENSION_NOT_INSTALLED",
      title: "Wallet not found",
    });
  });

  it("stops before access when the wallet is on mainnet instead of testnet", async () => {
    const requestAccess = jest.fn();
    installWallet({
      getNetworkDetails: jest.fn().mockResolvedValue({ network: "PUBLIC" }),
      requestAccess,
    });
    const { result } = renderHook(() => useWalletConnection());

    await act(async () => {
      await result.current.connect();
    });

    expect(result.current.error).toMatchObject({
      code: "NETWORK_MISMATCH",
      recoveryAction: expect.stringContaining("Stellar Testnet"),
    });
    expect(requestAccess).not.toHaveBeenCalled();
  });

  it("surfaces network lookup failures instead of continuing connection", async () => {
    const requestAccess = jest.fn();
    installWallet({
      getNetworkDetails: jest.fn().mockRejectedValue(new Error("Wallet is locked")),
      requestAccess,
    });
    const { result } = renderHook(() => useWalletConnection());

    await act(async () => {
      await result.current.connect();
    });

    expect(result.current.error).toMatchObject({
      code: "UNKNOWN",
      message: "Wallet is locked",
    });
    expect(requestAccess).not.toHaveBeenCalled();
  });

  it("classifies a rejected wallet request and allows a fresh attempt", async () => {
    const requestAccess = jest
      .fn()
      .mockRejectedValue({ code: 4001, message: "User rejected the request" });
    installWallet({ requestAccess });
    const { result } = renderHook(() => useWalletConnection());

    await act(async () => {
      await result.current.connect();
    });

    expect(result.current.error).toMatchObject({
      code: "USER_REJECTED",
      recoveryAction: expect.stringContaining("approve"),
    });
    expect(result.current.isConnecting).toBe(false);
  });
});
