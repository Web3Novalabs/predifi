import { isComponentsDemoEnabled } from "@/lib/componentsDemo";
import ComponentsDemoLayout from "@/app/components-demo/layout";

const notFound = jest.fn(() => {
  throw new Error("NEXT_NOT_FOUND");
});
jest.mock("next/navigation", () => ({
  notFound: () => notFound(),
}));

describe("isComponentsDemoEnabled", () => {
  it("is enabled in development and test", () => {
    expect(isComponentsDemoEnabled({ NODE_ENV: "development" })).toBe(true);
    expect(isComponentsDemoEnabled({ NODE_ENV: "test" })).toBe(true);
  });

  it("is disabled in production by default", () => {
    expect(isComponentsDemoEnabled({ NODE_ENV: "production" })).toBe(false);
  });

  it("can be explicitly enabled in production", () => {
    expect(
      isComponentsDemoEnabled({
        NODE_ENV: "production",
        NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO: "1",
      }),
    ).toBe(true);
  });

  it("ignores flag values other than 1", () => {
    expect(
      isComponentsDemoEnabled({
        NODE_ENV: "production",
        NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO: "true",
      }),
    ).toBe(false);
  });
});

describe("ComponentsDemoLayout", () => {
  const env = process.env as Record<string, string | undefined>;
  const originalNodeEnv = env.NODE_ENV;
  const originalFlag = env.NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO;

  afterEach(() => {
    env.NODE_ENV = originalNodeEnv;
    if (originalFlag === undefined) delete env.NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO;
    else env.NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO = originalFlag;
    notFound.mockClear();
  });

  it("renders children outside production", () => {
    env.NODE_ENV = "development";
    expect(() => ComponentsDemoLayout({ children: null })).not.toThrow();
    expect(notFound).not.toHaveBeenCalled();
  });

  it("calls notFound() in production", () => {
    env.NODE_ENV = "production";
    delete env.NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO;
    expect(() => ComponentsDemoLayout({ children: null })).toThrow("NEXT_NOT_FOUND");
    expect(notFound).toHaveBeenCalledTimes(1);
  });

  it("renders in production when the flag is set", () => {
    env.NODE_ENV = "production";
    env.NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO = "1";
    expect(() => ComponentsDemoLayout({ children: null })).not.toThrow();
  });
});
