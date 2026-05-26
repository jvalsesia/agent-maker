import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import i18n from "@/i18n";
import { AppearanceSection } from "./Appearance";
import { Wrap, makeQueryClient, mockFetch, jsonResponse } from "@/test-utils";

const settings = {
  default_provider: "anthropic",
  default_model: { anthropic: null, openai: null, openai_compat: null },
  memory_defaults: { recent_n: 10, top_k: 5 },
  appearance: { theme: "system", locale: "en" },
  providers: [],
  key_store_backend: "keyring",
};

function renderSection(locale: "en" | "pt-BR") {
  mockFetch((url) => {
    if (url === "/api/settings") {
      return jsonResponse({ ...settings, appearance: { theme: "system", locale } });
    }
    return jsonResponse({}, 404);
  });
  const client = makeQueryClient();
  render(
    <Wrap client={client}>
      <AppearanceSection />
    </Wrap>,
  );
}

afterEach(async () => {
  await i18n.changeLanguage("en");
});

describe("AppearanceSection language selector", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
  });

  it("renders the language label and shows the active locale by native name", async () => {
    renderSection("en");
    // Label from the en catalog.
    expect(await screen.findByText("Language")).toBeInTheDocument();
    // The Select trigger (combobox) displays the current locale's native name.
    const trigger = screen.getByRole("combobox");
    expect(trigger.textContent).toContain("English");
  });

  it("reflects a persisted pt-BR setting in the trigger", async () => {
    // The Gate applies the persisted locale to i18n; emulate that here.
    await i18n.changeLanguage("pt-BR");
    renderSection("pt-BR");
    // Label now comes from the pt-BR catalog…
    expect(await screen.findByText("Idioma")).toBeInTheDocument();
    // …and the trigger shows Portuguese as the selected value by native name.
    const trigger = screen.getByRole("combobox");
    expect(trigger.textContent).toContain("Português (Brasil)");
  });
});
