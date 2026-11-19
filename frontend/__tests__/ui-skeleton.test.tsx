import { render, screen } from "@testing-library/react";
import { Skeleton, SkeletonText, SkeletonCircle } from "@/components/ui/skeleton";

describe("Skeleton", () => {
  it("renders", () => {
    render(
      <div role="status" aria-label="Loading content">
        <Skeleton />
      </div>,
    );
    expect(
      screen.getByRole("status", { name: "Loading content" }).firstElementChild,
    ).toBeInTheDocument();
  });

  it("accepts a className", () => {
    const { container } = render(<Skeleton className="custom-class" />);
    expect(container.firstChild).toHaveClass("custom-class");
  });

  it("is hidden from assistive technology because it is decorative", () => {
    const { container } = render(<Skeleton />);
    expect(container.firstChild).toHaveAttribute("aria-hidden", "true");
  });
});

describe("SkeletonText", () => {
  it("renders", () => {
    const { container } = render(<SkeletonText />);
    expect(container.firstChild).toBeInTheDocument();
  });

  it("accepts a className", () => {
    const { container } = render(<SkeletonText className="custom-class" />);
    expect(container.firstChild).toHaveClass("custom-class");
  });

  it("hides its decorative lines from assistive technology", () => {
    const { container } = render(<SkeletonText />);
    expect(container.querySelectorAll('[aria-hidden="true"]')).toHaveLength(3);
  });
});

describe("SkeletonCircle", () => {
  it("renders", () => {
    const { container } = render(<SkeletonCircle />);
    expect(container.firstChild).toBeInTheDocument();
  });

  it("accepts a className", () => {
    const { container } = render(<SkeletonCircle className="custom-class" />);
    expect(container.firstChild).toHaveClass("custom-class");
  });

  it("is hidden from assistive technology because it is decorative", () => {
    const { container } = render(<SkeletonCircle />);
    expect(container.firstChild).toHaveAttribute("aria-hidden", "true");
  });
});
