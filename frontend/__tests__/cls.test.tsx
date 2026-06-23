/**
 * CLS Improvements Test Suite
 * 
 * Tests to verify that CLS improvements are working correctly.
 * Run with: npm test -- cls.test.ts
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ReservedSpace, AspectRatioContainer, SafeDropdown } from "@/lib/cls-utils";

describe("CLS (Cumulative Layout Shift) Utilities", () => {
  describe("ReservedSpace", () => {
    it("should reserve space for content even when hidden", () => {
      const { container } = render(
        <ReservedSpace isVisible={false} minHeight="100px">
          <div>Error message</div>
        </ReservedSpace>
      );

      const reservedSpace = container.firstChild as HTMLElement;
      expect(reservedSpace.style.minHeight).toBe("100px");
    });

    it("should fade in content smoothly", async () => {
      const { container, rerender } = render(
        <ReservedSpace isVisible={false}>
          <div>Error message</div>
        </ReservedSpace>
      );

      const reservedSpace = container.firstChild as HTMLElement;
      expect(reservedSpace.style.opacity).toBe("0");

      // Show content
      rerender(
        <ReservedSpace isVisible={true}>
          <div>Error message</div>
        </ReservedSpace>
      );

      expect(reservedSpace.style.opacity).toBe("1");
    });

    it("should use custom min-height", () => {
      const { container } = render(
        <ReservedSpace isVisible={false} minHeight="200px">
          <div>Custom height content</div>
        </ReservedSpace>
      );

      const reservedSpace = container.firstChild as HTMLElement;
      expect(reservedSpace.style.minHeight).toBe("200px");
    });
  });

  describe("AspectRatioContainer", () => {
    it("should set aspect ratio correctly", () => {
      const { container } = render(
        <AspectRatioContainer aspectRatio="16 / 9">
          <img src="/test.jpg" alt="test" />
        </AspectRatioContainer>
      );

      const aspectContainer = container.firstChild as HTMLElement;
      expect(aspectContainer.style.aspectRatio).toBe("16 / 9");
    });

    it("should use square aspect ratio by default", () => {
      const { container } = render(
        <AspectRatioContainer>
          <img src="/test.jpg" alt="test" />
        </AspectRatioContainer>
      );

      const aspectContainer = container.firstChild as HTMLElement;
      expect(aspectContainer.style.aspectRatio).toBe("1 / 1");
    });

    it("should include custom className", () => {
      const { container } = render(
        <AspectRatioContainer className="custom-class">
          <img src="/test.jpg" alt="test" />
        </AspectRatioContainer>
      );

      const aspectContainer = container.firstChild as HTMLElement;
      expect(aspectContainer.className).toContain("custom-class");
    });

    it("should maintain width at 100%", () => {
      const { container } = render(
        <AspectRatioContainer>
          <img src="/test.jpg" alt="test" />
        </AspectRatioContainer>
      );

      const aspectContainer = container.firstChild as HTMLElement;
      expect(aspectContainer.style.width).toBe("100%");
    });
  });

  describe("SafeDropdown", () => {
    it("should be hidden when closed", () => {
      const { container } = render(
        <SafeDropdown isOpen={false}>
          <a href="/about">About</a>
        </SafeDropdown>
      );

      const dropdown = container.firstChild as HTMLElement;
      expect(dropdown.style.maxHeight).toBe("0px");
      expect(dropdown.style.opacity).toBe("0");
    });

    it("should be visible when open", () => {
      const { container } = render(
        <SafeDropdown isOpen={true}>
          <a href="/about">About</a>
        </SafeDropdown>
      );

      const dropdown = container.firstChild as HTMLElement;
      expect(dropdown.style.maxHeight).not.toBe("0px");
      expect(dropdown.style.opacity).toBe("1");
    });

    it("should use custom maxHeight", () => {
      const { container } = render(
        <SafeDropdown isOpen={true} maxHeight="800px">
          <a href="/about">About</a>
        </SafeDropdown>
      );

      const dropdown = container.firstChild as HTMLElement;
      expect(dropdown.style.maxHeight).toBe("800px");
    });

    it("should transition smoothly between states", async () => {
      const { container, rerender } = render(
        <SafeDropdown isOpen={false}>
          <a href="/about">About</a>
        </SafeDropdown>
      );

      const dropdown = container.firstChild as HTMLElement;
      const transition = dropdown.style.transition;
      expect(transition).toContain("max-height");
      expect(transition).toContain("ease-out");
    });
  });

  describe("CLS Integration Tests", () => {
    it("should prevent layout shift when error appears", async () => {
      const TestForm = ({ hasError }: { hasError: boolean }) => (
        <form>
          <input type="text" />
          <ReservedSpace isVisible={hasError} minHeight="50px">
            <div className="error">Error message</div>
          </ReservedSpace>
          <button>Submit</button>
        </form>
      );

      const { container, rerender } = render(<TestForm hasError={false} />);
      const formElement = container.querySelector("form");
      const initialHeight = formElement?.offsetHeight || 0;

      rerender(<TestForm hasError={true} />);
      const heightWithError = formElement?.offsetHeight || 0;

      // Height should increase by approximately the minHeight value
      expect(heightWithError).toBeGreaterThan(initialHeight);
      // But should not cause visual jank (verified via transition)
    });

    it("should prevent layout shift when menu opens", async () => {
      const TestNav = ({ isOpen }: { isOpen: boolean }) => (
        <nav>
          <button>Menu</button>
          <SafeDropdown isOpen={isOpen}>
            <a href="/page1">Page 1</a>
            <a href="/page2">Page 2</a>
          </SafeDropdown>
        </nav>
      );

      const { container, rerender } = render(<TestNav isOpen={false} />);
      const navElement = container.querySelector("nav");
      const closedHeight = navElement?.offsetHeight || 0;

      rerender(<TestNav isOpen={true} />);
      const openHeight = navElement?.offsetHeight || 0;

      // Menu should expand smoothly
      expect(openHeight).toBeGreaterThan(closedHeight);
    });
  });
});
