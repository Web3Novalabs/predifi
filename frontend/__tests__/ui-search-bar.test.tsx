import React from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SearchBar } from "@/components/ui/search-bar";

describe("SearchBar", () => {
  it("renders with its placeholder", () => {
    render(
      <SearchBar
        placeholder="Search pools..."
        onSearch={jest.fn()}
      />,
    );

    expect(screen.getByPlaceholderText("Search pools...")).toBeInTheDocument();
  });

  it("calls the change handler as the user types", async () => {
    const user = userEvent.setup();
    const onChange = jest.fn();

    render(
      <SearchBar
        onSearch={jest.fn()}
        onChange={onChange}
        debounceDelay={0}
      />,
    );

    const input = screen.getByRole("searchbox");
    await user.type(input, "btc");

    expect(onChange).toHaveBeenCalled();
    expect(onChange).toHaveBeenLastCalledWith("btc");
  });

  it("clears via the clear control", async () => {
    const user = userEvent.setup();
    const onChange = jest.fn();

    render(
      <SearchBar
        onSearch={jest.fn()}
        onChange={onChange}
        debounceDelay={0}
      />,
    );

    const input = screen.getByRole("searchbox") as HTMLInputElement;
    await user.type(input, "eth");
    expect(input.value).toBe("eth");

    const clearBtn = screen.getByRole("button", { name: /clear search/i });
    await user.click(clearBtn);

    expect(input.value).toBe("");
    expect(onChange).toHaveBeenCalledWith("");
  });

  it("exposes an accessible label", () => {
    render(
      <SearchBar
        onSearch={jest.fn()}
        aria-label="Search markets"
      />,
    );

    expect(
      screen.getByRole("searchbox", { name: "Search markets" }),
    ).toBeInTheDocument();
  });
});
