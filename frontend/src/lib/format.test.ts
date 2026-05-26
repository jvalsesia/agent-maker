import { describe, it, expect } from "vitest";
import { formatDate, formatDateTime, formatNumber, formatPercent } from "./format";

const ISO = "2026-05-26T12:00:00Z";

describe("locale-aware formatters", () => {
  it("formats the same date differently per locale", () => {
    const en = formatDate(ISO, "en");
    const pt = formatDate(ISO, "pt-BR");
    expect(en).not.toEqual(pt);
    // Sanity: the year appears in both renderings.
    expect(en).toContain("2026");
    expect(pt).toContain("2026");
  });

  it("formats date-time without throwing and includes the year", () => {
    expect(formatDateTime(ISO, "pt-BR")).toContain("2026");
  });

  it("groups numbers per locale", () => {
    expect(formatNumber(1234567, "en")).toBe("1,234,567");
    // pt-BR uses a dot (or non-breaking grouping) — distinct from the en form.
    expect(formatNumber(1234567, "pt-BR")).not.toBe("1,234,567");
  });

  it("renders a 0..1 fraction as a percentage", () => {
    expect(formatPercent(0.42, "en")).toBe("42%");
    expect(formatPercent(0.5, "en")).toBe("50%");
  });
});
