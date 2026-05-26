import { describe, it, expect } from "vitest";
import { isSupported, resolveBrowserLocale } from "./supported";

describe("isSupported", () => {
  it("accepts the supported codes and rejects others", () => {
    expect(isSupported("en")).toBe(true);
    expect(isSupported("pt-BR")).toBe(true);
    expect(isSupported("fr")).toBe(false);
    expect(isSupported("pt")).toBe(false);
  });
});

describe("resolveBrowserLocale", () => {
  it("maps browser language tags to the nearest supported locale", () => {
    // Exact and primary-subtag matches resolve to pt-BR.
    expect(resolveBrowserLocale("pt-BR")).toBe("pt-BR");
    expect(resolveBrowserLocale("pt")).toBe("pt-BR");
    expect(resolveBrowserLocale("pt-PT")).toBe("pt-BR");
    // Unknown and empty inputs fall back to English.
    expect(resolveBrowserLocale("fr")).toBe("en");
    expect(resolveBrowserLocale("")).toBe("en");
    expect(resolveBrowserLocale(null)).toBe("en");
    expect(resolveBrowserLocale(undefined)).toBe("en");
  });
});
