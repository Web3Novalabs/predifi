import { render, screen, waitFor } from "@testing-library/react";
import { TransactionStatusTracker } from "@/components/ui/transaction-progress";

function rpcResponse(result: Record<string, unknown>): Response {
  return {
    ok: true,
    json: async () => ({ jsonrpc: "2.0", id: "tx", result }),
  } as Response;
}

describe("TransactionStatusTracker", () => {
  let fetchMock: jest.Mock;

  beforeEach(() => {
    fetchMock = jest.fn();
    Object.defineProperty(globalThis, "fetch", {
      configurable: true,
      value: fetchMock,
    });
  });

  afterEach(() => {
    delete (globalThis as { fetch?: typeof fetch }).fetch;
  });

  it("polls the selected Stellar network and links a confirmed transaction", async () => {
    fetchMock.mockResolvedValue(rpcResponse({ status: "SUCCESS" }));

    render(
      <TransactionStatusTracker txHash="abc123" network="PUBLIC" />,
    );

    expect(await screen.findByText("Confirmed")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      "https://soroban.stellar.org",
      expect.objectContaining({
        method: "POST",
        body: expect.stringContaining('"method":"getTransaction"'),
      }),
    );
    expect(
      screen.getByRole("link", { name: "View transaction in Stellar block explorer" }),
    ).toHaveAttribute(
      "href",
      "https://stellar.expert/explorer/public/tx/abc123",
    );
  });

  it("reports network failures as failed transactions", async () => {
    fetchMock.mockResolvedValue(
      rpcResponse({ status: "FAILED", errorResultXdr: "Transaction failed" }),
    );

    render(<TransactionStatusTracker txHash="bad" />);

    expect(await screen.findByText("Failed")).toBeInTheDocument();
    expect(screen.getByText("Transaction failed")).toBeInTheDocument();
  });

  it("times out without treating an unconfirmed transaction as failed", async () => {
    fetchMock.mockImplementation(
      (_input, init) =>
        new Promise((_resolve, reject) => {
          init?.signal?.addEventListener("abort", () =>
            reject(new DOMException("Aborted", "AbortError")),
          );
        }),
    );

    render(
      <TransactionStatusTracker txHash="slow" timeoutMs={20} pollIntervalMs={5} />,
    );

    await waitFor(() => {
      expect(screen.getByText("Still pending")).toBeInTheDocument();
    });
    expect(
      screen.getByText(/The transaction may still confirm; check the explorer before retrying/),
    ).toBeInTheDocument();
    expect(screen.queryByText("Failed")).not.toBeInTheDocument();
  });
});
