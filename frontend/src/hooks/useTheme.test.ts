import { describe, it, expect, beforeEach, vi } from "vitest";
import { applyTheme } from "./useTheme";

beforeEach(() => {
  document.documentElement.className = "";
  vi.spyOn(window, "matchMedia").mockImplementation((q: string) => ({
    matches: q.includes("dark"),
    media: q,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }) as unknown as MediaQueryList);
});

describe("applyTheme", () => {
  it("adds 'dark' class for theme=dark", () => {
    applyTheme("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
  it("removes 'dark' class for theme=light", () => {
    document.documentElement.classList.add("dark");
    applyTheme("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });
  it("follows system preference for theme=system (mocked dark)", () => {
    applyTheme("system");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
});
