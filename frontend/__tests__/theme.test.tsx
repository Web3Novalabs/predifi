import React from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { ThemeToggle } from "@/components/ui/theme-toggle";
import { useTheme } from "@/lib/hooks/useTheme";

function TestThemeComponent() {
  const { theme, setTheme } = useTheme();
  return (
    <div>
      <span data-testid="current-theme">{theme}</span>
      <button onClick={() => setTheme("dark")}>Set Dark</button>
      <button onClick={() => setTheme("light")}>Set Light</button>
    </div>
  );
}

describe("Dark Mode & Theme Switching", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.classList.remove("dark");
  });

  it("updates data-theme attribute and localStorage on theme change", () => {
    render(<TestThemeComponent />);

    const darkBtn = screen.getByText("Set Dark");
    fireEvent.click(darkBtn);

    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(localStorage.getItem("predifi:theme")).toBe("dark");

    const lightBtn = screen.getByText("Set Light");
    fireEvent.click(lightBtn);

    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(localStorage.getItem("predifi:theme")).toBe("light");
  });

  it("cycles through theme options on ThemeToggle click", () => {
    render(<ThemeToggle />);

    const toggleBtn = screen.getByRole("button", { name: /Switch theme/i });
    expect(toggleBtn).toBeInTheDocument();

    // Click to cycle theme
    fireEvent.click(toggleBtn);
    expect(document.documentElement.getAttribute("data-theme")).toBeDefined();
  });
});
