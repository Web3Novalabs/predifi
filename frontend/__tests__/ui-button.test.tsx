import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { cn } from "@/lib/utils";
import {
  BUTTON_VARIANTS,
  Button,
  buttonVariants,
} from "@/components/ui/button";

describe("Button", () => {
  it("renders its children", () => {
    render(<Button>Click me</Button>);

    expect(screen.getByRole("button", { name: "Click me" })).toBeInTheDocument();
  });

  it("fires onClick when clicked", async () => {
    const user = userEvent.setup();
    const onClick = jest.fn();

    render(<Button onClick={onClick}>Click me</Button>);

    await user.click(screen.getByRole("button", { name: "Click me" }));

    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("does not fire onClick when disabled", async () => {
    const user = userEvent.setup();
    const onClick = jest.fn();

    render(
      <Button disabled onClick={onClick}>
        Click me
      </Button>,
    );

    await user.click(screen.getByRole("button", { name: "Click me" }));

    expect(onClick).not.toHaveBeenCalled();
  });

  it.each(BUTTON_VARIANTS)(
    "applies the %s variant class",
    (variant) => {
      render(<Button variant={variant}>Click me</Button>);

      const button = screen.getByRole("button", { name: "Click me" });
      const expected = cn(buttonVariants({ variant }));

      for (const className of expected.split(" ")) {
        expect(button).toHaveClass(className);
      }
    },
  );
});
