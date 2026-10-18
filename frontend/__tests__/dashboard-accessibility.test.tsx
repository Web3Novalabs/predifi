import { fireEvent, render, screen } from "@testing-library/react";
import { PredictionList } from "@/components/dashboard/PredictionList";

jest.mock("@/components/ui/copy-button", () => ({
  CopyButton: () => <button type="button" aria-label="Copy prediction ID" />,
}));

describe("PredictionList accessibility", () => {
  it("exposes selected tabs and supports arrow-key navigation", () => {
    render(<PredictionList />);

    const activeTab = screen.getByRole("tab", { name: /Active Prediction/ });
    const pastTab = screen.getByRole("tab", { name: "Past Predictions" });
    expect(activeTab).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tabpanel")).toHaveAccessibleName(
      "Active Prediction 6",
    );

    fireEvent.keyDown(activeTab, { key: "ArrowRight" });

    expect(pastTab).toHaveAttribute("aria-selected", "true");
    expect(pastTab).toHaveFocus();
    expect(screen.getByRole("tabpanel")).toHaveAccessibleName(
      "Past Predictions",
    );
  });
});
