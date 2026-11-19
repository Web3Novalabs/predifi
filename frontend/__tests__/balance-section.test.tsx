import { render, screen } from "@testing-library/react";
import { BalanceSection } from "@/components/dashboard/BalanceSection";

describe("BalanceSection", () => {
  it("shows a skeleton and no figure when balance is undefined", () => {
    render(<BalanceSection />);
    expect(screen.getByTestId("balance-skeleton")).toBeInTheDocument();
    expect(screen.queryByText(/15,255/)).not.toBeInTheDocument();
    expect(screen.queryByText("Unavailable")).not.toBeInTheDocument();
  });

  it("never falls back to the old demo values", () => {
    render(<BalanceSection />);
    expect(screen.queryByText("$15,255.25")).not.toBeInTheDocument();
    expect(screen.queryByText("$1,255.68")).not.toBeInTheDocument();
  });

  it("shows Unavailable when balance is null", () => {
    render(<BalanceSection balance={null} rewards={null} />);
    expect(screen.getByText("Unavailable")).toBeInTheDocument();
    expect(screen.queryByTestId("balance-skeleton")).not.toBeInTheDocument();
  });

  it("shows an em dash for null rewards", () => {
    render(<BalanceSection balance={10} rewards={null} />);
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("shows a skeleton for undefined rewards", () => {
    render(<BalanceSection balance={10} />);
    expect(screen.getByTestId("rewards-skeleton")).toBeInTheDocument();
  });

  it("renders real values", () => {
    render(<BalanceSection balance={42.5} rewards="7.25" />);
    expect(screen.getByText("$42.50")).toBeInTheDocument();
    expect(screen.getByText("$7.25")).toBeInTheDocument();
    expect(screen.queryByTestId("balance-skeleton")).not.toBeInTheDocument();
  });

  it("renders a real zero balance rather than treating it as missing", () => {
    render(<BalanceSection balance={0} rewards={0} />);
    expect(screen.getAllByText("$0.00")).toHaveLength(2);
  });

  it("shows the full-card skeleton while isLoading", () => {
    render(<BalanceSection isLoading balance={5} rewards={5} />);
    expect(screen.queryByText("Total Balance")).not.toBeInTheDocument();
  });
});
