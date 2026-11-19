import { render, screen } from "@testing-library/react";
import { BalanceSectionContainer } from "@/components/dashboard/BalanceSectionContainer";

const mockAddress = jest.fn();
const mockProfile = jest.fn();
jest.mock("@/lib/hooks/useCurrentUserAddress", () => ({
  useCurrentUserAddress: () => mockAddress(),
}));
jest.mock("@/lib/hooks/useProfile", () => ({
  useProfile: (a: string | undefined) => mockProfile(a),
}));

const claim = (over: Record<string, unknown>) => ({
  amount: 10,
  is_winner: true,
  claimed: false,
  claim_expired: false,
  ...over,
});

describe("BalanceSectionContainer", () => {
  it("shows em dash rewards with no wallet connected", () => {
    mockAddress.mockReturnValue(undefined);
    mockProfile.mockReturnValue({ profile: undefined, isLoading: false, isError: false });
    render(<BalanceSectionContainer />);
    expect(screen.getByText("Unavailable")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("shows a rewards skeleton while the profile loads", () => {
    mockAddress.mockReturnValue("GABC");
    mockProfile.mockReturnValue({ profile: undefined, isLoading: true, isError: false });
    render(<BalanceSectionContainer />);
    expect(screen.getByTestId("rewards-skeleton")).toBeInTheDocument();
  });

  it("sums only unclaimed, unexpired winning claims", () => {
    mockAddress.mockReturnValue("GABC");
    mockProfile.mockReturnValue({
      isLoading: false,
      isError: false,
      profile: {
        claims: [
          claim({ amount: 10 }),
          claim({ amount: 5.5 }),
          claim({ amount: 100, claimed: true }),
          claim({ amount: 100, claim_expired: true }),
          claim({ amount: 100, is_winner: false }),
          claim({ amount: 100, is_winner: null }),
        ],
      },
    });
    render(<BalanceSectionContainer />);
    expect(screen.getByText("$15.50")).toBeInTheDocument();
  });
});
