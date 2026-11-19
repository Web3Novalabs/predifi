import { fireEvent, render, screen } from "@testing-library/react";
import { CreatePoolForm } from "@/components/pool/CreatePoolForm";

class TestResizeObserver implements ResizeObserver {
  observe(_target: Element, _options?: ResizeObserverOptions): void {}
  unobserve(_target: Element): void {}
  disconnect(): void {}
}

describe("CreatePoolForm", () => {
  beforeAll(() => {
    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: TestResizeObserver,
    });
  });

  it("does not simulate pool creation while contract submission is unavailable", () => {
    render(<CreatePoolForm />);

    expect(screen.getByRole("status")).toHaveTextContent(
      "Pool creation is coming soon. Submissions are not yet available.",
    );
    expect(screen.getByRole("button", { name: "Create Pool" })).toBeDisabled();

    fireEvent.submit(screen.getByRole("form", { name: "Create prediction pool" }));
    expect(
      screen.queryByRole("heading", { name: "Pool Created Successfully" }),
    ).not.toBeInTheDocument();
  });
});
