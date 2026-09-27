import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CopyButton } from "@/components/ui/copy-button";
import { ToastProvider } from "@/components/ui/toast-provider";

function renderCopyButton(text: string) {
  return render(
    <ToastProvider>
      <CopyButton text={text} />
    </ToastProvider>,
  );
}

describe("CopyButton", () => {
  const writeText = jest.fn().mockResolvedValue(undefined);

  beforeEach(() => {
    writeText.mockClear();
    Object.assign(navigator, {
      clipboard: { writeText },
    });
  });

  it("copies the given value when clicked", async () => {
    const user = userEvent.setup();
    const value = "0xabc123def456";

    renderCopyButton(value);

    const button = screen.getByRole("button", { name: /copy to clipboard/i });
    await user.click(button);

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith(value);
    });
  });

  it("updates the accessible label to a copied state after copying", async () => {
    const user = userEvent.setup();

    renderCopyButton("hello-world");

    const button = screen.getByRole("button", { name: /copy to clipboard/i });
    await user.click(button);

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: /copied/i }),
      ).toBeInTheDocument();
    });
  });

  it("shows error state when clipboard write fails", async () => {
    const user = userEvent.setup();
    const error = new Error("Clipboard access denied");
    writeText.mockRejectedValueOnce(error);

    renderCopyButton("test-value");

    const button = screen.getByRole("button", { name: /copy to clipboard/i });
    await user.click(button);

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: /copy failed/i }),
      ).toBeInTheDocument();
    });
  });

  it("resets error state after resetDelay", async () => {
    const user = userEvent.setup();
    const error = new Error("Clipboard access denied");
    writeText.mockRejectedValueOnce(error);

    renderCopyButton("test-value");

    const button = screen.getByRole("button", { name: /copy to clipboard/i });
    await user.click(button);

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: /copy failed/i }),
      ).toBeInTheDocument();
    });

    // Wait for error state to reset (default resetDelay is 2000ms)
    await waitFor(
      () => {
        expect(
          screen.getByRole("button", { name: /copy to clipboard/i }),
        ).toBeInTheDocument();
      },
      { timeout: 3000 },
    );
  });
});
