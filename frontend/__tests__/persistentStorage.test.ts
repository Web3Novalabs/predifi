import {
  getPersistedValue,
  setPersistedValue,
  removePersistedValue,
} from "@/lib/persistentStorage";

const STORAGE_PREFIX = "predifi:";

function clearAllCookies() {
  document.cookie.split(";").forEach((entry) => {
    const name = entry.split("=")[0]?.trim();
    if (name) {
      document.cookie = `${name}=; Path=/; Max-Age=0`;
    }
  });
}

/**
 * Shadows `document.cookie` with an accessor that silently swallows writes,
 * the way a browser does when cookies are blocked. Returns a restore
 * function that removes the shadow so the real jsdom implementation takes
 * over again.
 */
function blockCookies() {
  Object.defineProperty(document, "cookie", {
    configurable: true,
    get: () => "",
    set: () => {
      throw new Error("cookies blocked");
    },
  });

  return () => {
    delete (document as unknown as Record<string, unknown>).cookie;
  };
}

describe("persistentStorage", () => {
  beforeEach(() => {
    clearAllCookies();
    window.localStorage.clear();
  });

  afterEach(() => {
    clearAllCookies();
    window.localStorage.clear();
    jest.restoreAllMocks();
  });

  it("writes then reads back the same value (round-trip via cookie)", () => {
    expect(setPersistedValue("theme", "dark")).toBe(true);
    expect(getPersistedValue("theme")).toBe("dark");
  });

  it("removes a persisted value so it can no longer be read", () => {
    setPersistedValue("theme", "dark");
    removePersistedValue("theme");
    expect(getPersistedValue("theme")).toBeNull();
  });

  it("clears a stale localStorage fallback once the cookie write succeeds", () => {
    window.localStorage.setItem(`${STORAGE_PREFIX}theme`, "stale");
    setPersistedValue("theme", "dark");
    expect(window.localStorage.getItem(`${STORAGE_PREFIX}theme`)).toBeNull();
  });

  it("returns null when a key has never been persisted", () => {
    expect(getPersistedValue("never-set")).toBeNull();
  });

  it("falls back to localStorage when no cookie is present for the key", () => {
    window.localStorage.setItem(`${STORAGE_PREFIX}lang`, "en");
    expect(getPersistedValue("lang")).toBe("en");
  });

  describe("when the browser blocks or fails storage", () => {
    it("setPersistedValue falls back to localStorage without throwing when cookies are blocked", () => {
      const unblock = blockCookies();
      try {
        expect(() => setPersistedValue("theme", "dark")).not.toThrow();
        expect(setPersistedValue("theme", "dark")).toBe(true);
        expect(window.localStorage.getItem(`${STORAGE_PREFIX}theme`)).toBe(
          "dark",
        );
      } finally {
        unblock();
      }
    });

    it("setPersistedValue returns false without throwing when both cookies and localStorage are unavailable", () => {
      const unblock = blockCookies();
      jest.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
        throw new Error("storage disabled");
      });

      try {
        expect(() => setPersistedValue("theme", "dark")).not.toThrow();
        expect(setPersistedValue("theme", "dark")).toBe(false);
      } finally {
        unblock();
      }
    });

    it("getPersistedValue returns null instead of throwing when localStorage.getItem throws", () => {
      const unblock = blockCookies();
      jest.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
        throw new Error("storage disabled");
      });

      try {
        expect(() => getPersistedValue("theme")).not.toThrow();
        expect(getPersistedValue("theme")).toBeNull();
      } finally {
        unblock();
      }
    });

    it("removePersistedValue does not throw when localStorage.removeItem throws", () => {
      jest.spyOn(Storage.prototype, "removeItem").mockImplementation(() => {
        throw new Error("storage disabled");
      });

      expect(() => removePersistedValue("theme")).not.toThrow();
    });
  });
});
