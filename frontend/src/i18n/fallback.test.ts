import { describe, it, expect, afterEach } from "vitest";
import i18n from "./index";

afterEach(async () => {
  await i18n.changeLanguage("en");
});

describe("i18n fallback", () => {
  it("renders the English string when a key is missing in the active locale, never the raw key", async () => {
    // A key present only in English; pt-BR has no translation for it.
    i18n.addResource("en", "translation", "onlyInEnglish", "Fallback Value");
    await i18n.changeLanguage("pt-BR");

    const value = i18n.t("onlyInEnglish");
    expect(value).toBe("Fallback Value");
    expect(value).not.toBe("onlyInEnglish");
  });

  it("never returns null for a missing key (returnNull: false)", async () => {
    await i18n.changeLanguage("pt-BR");
    // A wholly unknown key returns the key string itself, not null.
    expect(i18n.t("does.not.exist.anywhere")).toBe("does.not.exist.anywhere");
  });
});
