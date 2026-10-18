import { render, screen } from "@testing-library/jest-dom";
import { render as rtlRender } from "@testing-library/react";
import { Skeleton, SkeletonText, SkeletonCircle } from "@/components/ui/skeleton";

describe("Skeleton", () => {
  it("renders", () => {
    rtlRender(<Skeleton data-testid="skeleton" />);
    expect(screen.getByTestId("skeleton")).toBeInTheDocument();
  });

  it("accepts a className", () => {
    rtlRender(<Skeleton data-testid="skeleton" className="custom-class" />);
    expect(screen.getByTestId("skeleton")).toHaveClass("custom-class");
  });

  it("exposes an accessible busy state", () => {
    rtlRender(<Skeleton data-testid="skeleton" />);
    expect(screen.getByTestId("skeleton")).toHaveAttribute("aria-busy", "true");
  });
});

describe("SkeletonText", () => {
  it("renders", () => {
    rtlRender(<SkeletonText data-testid="skeleton-text" />);
    expect(screen.getByTestId("skeleton-text")).toBeInTheDocument();
  });

  it("accepts a className", () => {
    rtlRender(<SkeletonText data-testid="skeleton-text" className="custom-class" />);
    expect(screen.getByTestId("skeleton-text")).toHaveClass("custom-class");
  });

  it("exposes an accessible busy state", () => {
    rtlRender(<SkeletonText data-testid="skeleton-text" />);
    expect(screen.getByTestId("skeleton-text")).toHaveAttribute("aria-busy", "true");
  });
});

describe("SkeletonCircle", () => {
  it("renders", () => {
    rtlRender(<SkeletonCircle data-testid="skeleton-circle" />);
    expect(screen.getByTestId("skeleton-circle")).toBeInTheDocument();
  });

  it("accepts a className", () => {
    rtlRender(<SkeletonCircle data-testid="skeleton-circle" className="custom-class" />);
    expect(screen.getByTestId("skeleton-circle")).toHaveClass("custom-class");
  });

  it("exposes an accessible busy state", () => {
    rtlRender(<SkeletonCircle data-testid="skeleton-circle" />);
    expect(screen.getByTestId("skeleton-circle")).toHaveAttribute("aria-busy", "true");
  });
});
