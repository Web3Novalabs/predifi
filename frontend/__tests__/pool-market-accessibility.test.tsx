import { fireEvent, render, screen } from "@testing-library/react";
import { PoolMarketClient } from "@/app/user/pool-market/PoolMarketClient";
import { usePools } from "@/lib/hooks/usePools";
import { useTags } from "@/lib/hooks/useTags";
import type { Pool } from "@/lib/api/pools";

jest.mock("@/lib/hooks/usePools", () => ({ usePools: jest.fn() }));
jest.mock("@/lib/hooks/useTags", () => ({ useTags: jest.fn() }));

const pool: Pool = {
  pool_id: 42,
  name: "Will BTC reach $100K?",
  category: "Crypto",
  tags: ["bitcoin"],
  total_stake: 123,
  end_time: 1_800_000_000,
  created_at: "2026-01-01T00:00:00Z",
  state: "active",
  creator: "GABC",
  token: "XLM",
  result: null,
};

describe("PoolMarketClient accessibility", () => {
  beforeEach(() => {
    jest.mocked(usePools).mockReturnValue({
      pools: [pool],
      total: 1,
      limit: 20,
      offset: 0,
      data: undefined,
      isLoading: false,
      isError: false,
      error: undefined,
      refresh: jest.fn(),
      cacheKey: "",
    });
    jest.mocked(useTags).mockReturnValue({
      tags: ["bitcoin"],
      isLoading: false,
    });
  });

  it("labels filter states, announces result changes, and makes pool rows keyboard links", () => {
    render(<PoolMarketClient />);

    expect(screen.getByRole("main")).toHaveAttribute("id", "main-content");
    expect(screen.getByRole("button", { name: "Crypto" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    fireEvent.click(screen.getByRole("button", { name: "Crypto" }));
    expect(screen.getByRole("button", { name: "Crypto" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    const poolLink = screen.getByRole("link", {
      name: /Will BTC reach \$100K\?.*Crypto.*123 XLM staked.*active/,
    });
    expect(poolLink).toHaveAttribute("href", "/user/pool-market/42");
    expect(poolLink).not.toHaveAttribute("tabindex", "-1");

    const calculatorToggle = screen.getByRole("button", {
      name: "Odds Calculator",
    });
    expect(calculatorToggle).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(calculatorToggle);
    expect(calculatorToggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("heading", { name: "Odds Calculator" })).toBeVisible();
  });
});
