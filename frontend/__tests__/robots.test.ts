describe("robots()", () => {
  const original = process.env.NEXT_PUBLIC_SITE_URL;

  afterEach(() => {
    if (original === undefined) delete process.env.NEXT_PUBLIC_SITE_URL;
    else process.env.NEXT_PUBLIC_SITE_URL = original;
    jest.resetModules();
  });

  function loadRobots() {
    let robots!: typeof import("@/app/robots").default;
    jest.isolateModules(() => {
      robots = require("@/app/robots").default;
    });
    return robots();
  }

  it("points the sitemap at NEXT_PUBLIC_SITE_URL when set", () => {
    process.env.NEXT_PUBLIC_SITE_URL = "https://staging.predifi.app";
    expect(loadRobots().sitemap).toBe("https://staging.predifi.app/sitemap.xml");
  });

  it("strips a trailing slash from NEXT_PUBLIC_SITE_URL", () => {
    process.env.NEXT_PUBLIC_SITE_URL = "https://staging.predifi.app/";
    expect(loadRobots().sitemap).toBe("https://staging.predifi.app/sitemap.xml");
  });

  it("falls back to the production URL when unset", () => {
    delete process.env.NEXT_PUBLIC_SITE_URL;
    expect(loadRobots().sitemap).toBe("https://predifi.app/sitemap.xml");
  });

  it("falls back to the production URL when empty", () => {
    process.env.NEXT_PUBLIC_SITE_URL = "";
    expect(loadRobots().sitemap).toBe("https://predifi.app/sitemap.xml");
  });

  it("disallows /api/ and /components-demo", () => {
    const { rules } = loadRobots();
    const rule = Array.isArray(rules) ? rules[0] : rules;
    expect(rule.disallow).toEqual(["/api/", "/components-demo"]);
  });
});
