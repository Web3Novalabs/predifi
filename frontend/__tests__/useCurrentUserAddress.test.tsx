import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { useCurrentUserAddress } from "@/hooks/useCurrentUserAddress";
import { WalletContext } from "@/context/WalletContext";

const CONNECTED_ADDRESS = "0x1234567890abcdef1234567890abcdef12345678";

function createWrapper(address: string | null | undefined) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <WalletContext.Provider value={{ address }}>
        {children}
      </WalletContext.Provider>
    );
  };
}

describe("useCurrentUserAddress", () => {
  it("returns the connected address when a wallet is connected", () => {
    const { result } = renderHook(() => useCurrentUserAddress(), {
      wrapper: createWrapper(CONNECTED_ADDRESS),
    });

    expect(result.current).toBe(CONNECTED_ADDRESS);
  });

  it("returns null when no wallet is connected", () => {
    const { result } = renderHook(() => useCurrentUserAddress(), {
      wrapper: createWrapper(null),
    });

    expect(result.current).toBeNull();
  });

  it("returns undefined when the address is undefined", () => {
    const { result } = renderHook(() => useCurrentUserAddress(), {
      wrapper: createWrapper(undefined),
    });

    expect(result.current).toBeUndefined();
  });
});
