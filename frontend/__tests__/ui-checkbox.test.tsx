import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Checkbox } from "@/components/ui/checkbox";

describe("Checkbox", () => {
  it("renders unchecked by default", () => {
    render(<Checkbox aria-label="Accept terms" />);

    const checkbox = screen.getByRole("checkbox", { name: "Accept terms" });
    expect(checkbox).toBeInTheDocument();
    expect(checkbox).toHaveAttribute("data-state", "unchecked");
    expect(checkbox).toHaveAttribute("aria-checked", "false");
  });

  it("toggles on click", async () => {
    const user = userEvent.setup();
    render(<Checkbox aria-label="Accept terms" />);

    const checkbox = screen.getByRole("checkbox", { name: "Accept terms" });
    await user.click(checkbox);
    expect(checkbox).toHaveAttribute("data-state", "checked");

    await user.click(checkbox);
    expect(checkbox).toHaveAttribute("data-state", "unchecked");
  });

  it("fires onCheckedChange when toggled", async () => {
    const user = userEvent.setup();
    const onCheckedChange = jest.fn();
    render(<Checkbox aria-label="Accept terms" onCheckedChange={onCheckedChange} />);

    const checkbox = screen.getByRole("checkbox", { name: "Accept terms" });
    await user.click(checkbox);

    expect(onCheckedChange).toHaveBeenCalledTimes(1);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });

  it("is reachable and toggleable by keyboard (Space)", async () => {
    const user = userEvent.setup();
    const onCheckedChange = jest.fn();
    render(<Checkbox aria-label="Accept terms" onCheckedChange={onCheckedChange} />);

    const checkbox = screen.getByRole("checkbox", { name: "Accept terms" });
    await user.tab();
    expect(checkbox).toHaveFocus();

    await user.keyboard(" ");
    expect(checkbox).toHaveAttribute("data-state", "checked");
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });
});
