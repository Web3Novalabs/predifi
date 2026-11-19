import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import Waitlist from "@/components/Waitlist";

const fetchMock = jest.fn();

beforeEach(() => {
  fetchMock.mockReset();
  global.fetch = fetchMock as unknown as typeof fetch;
});

function fill(name: string, email: string) {
  fireEvent.change(screen.getByPlaceholderText("Full Name"), { target: { value: name } });
  fireEvent.change(screen.getByPlaceholderText("Email Address"), { target: { value: email } });
}

function submit() {
  fireEvent.click(screen.getByRole("button", { name: "Join Waitlist" }));
}

describe("Waitlist", () => {
  it("POSTs name and email and shows success on a 2xx response", async () => {
    fetchMock.mockResolvedValue({ ok: true, status: 200 });
    render(<Waitlist />);
    fill("  Ada Lovelace ", " ada@example.com ");
    submit();

    expect(await screen.findByText("You're on the list!")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toMatch(/\/api\/v1\/waitlist$/);
    expect(init.method).toBe("POST");
    expect(init.headers).toEqual({ "Content-Type": "application/json" });
    expect(JSON.parse(init.body)).toEqual({ name: "Ada Lovelace", email: "ada@example.com" });
  });

  it("shows the error state, not success, on a non-2xx response", async () => {
    fetchMock.mockResolvedValue({ ok: false, status: 500 });
    render(<Waitlist />);
    fill("Ada", "ada@example.com");
    submit();

    expect(await screen.findByRole("alert")).toHaveTextContent("Something went wrong");
    expect(screen.queryByText("You're on the list!")).not.toBeInTheDocument();
    // The form keeps what the user typed so they can retry.
    expect(screen.getByPlaceholderText("Email Address")).toHaveValue("ada@example.com");
  });

  it("shows a rate-limit message on 429", async () => {
    fetchMock.mockResolvedValue({ ok: false, status: 429 });
    render(<Waitlist />);
    fill("Ada", "ada@example.com");
    submit();
    expect(await screen.findByRole("alert")).toHaveTextContent("Too many attempts");
  });

  it("shows the error state when the network request fails", async () => {
    fetchMock.mockRejectedValue(new TypeError("Failed to fetch"));
    render(<Waitlist />);
    fill("Ada", "ada@example.com");
    submit();
    expect(await screen.findByRole("alert")).toHaveTextContent("Something went wrong");
    expect(screen.queryByText("You're on the list!")).not.toBeInTheDocument();
  });

  it("does not call the API when validation fails", async () => {
    render(<Waitlist />);
    fill("Ada", "not-an-email");
    submit();
    expect(await screen.findByRole("alert")).toHaveTextContent("valid email");

    fill("  ", "ada@example.com");
    submit();
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("name"));
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("clears the form after a successful sign-up", async () => {
    fetchMock.mockResolvedValue({ ok: true, status: 200 });
    render(<Waitlist />);
    fill("Ada", "ada@example.com");
    submit();
    await screen.findByText("You're on the list!");
    expect(screen.queryByPlaceholderText("Email Address")).not.toBeInTheDocument();
  });
});
