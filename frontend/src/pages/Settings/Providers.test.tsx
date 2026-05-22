import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ProvidersSection } from "./Providers";
import { Wrap, makeQueryClient, mockFetch, jsonResponse } from "@/test-utils";

const RAW_KEY = "sk-ant-api03-XYZ123ABC";
const MASK = "sk-ant-***...3ABC";

const settings = {
  default_provider: "anthropic",
  default_model: { anthropic: null, openai: null, openai_compat: null },
  memory_defaults: { recent_n: 10, top_k: 5 },
  appearance: { theme: "system" },
  providers: [
    { name: "anthropic", key_configured: true, key_masked: MASK, base_url: null },
    { name: "openai", key_configured: false, key_masked: null, base_url: null },
    { name: "openai_compat", key_configured: false, key_masked: null, base_url: "http://localhost:11434/v1" },
  ],
  key_store_backend: "keyring",
};

describe("ProvidersSection", () => {
  beforeEach(() => {
    mockFetch((url) => {
      if (url === "/api/settings") return jsonResponse(settings);
      return jsonResponse({}, 404);
    });
  });

  it("renders masked key by default and reveals the masked string on toggle (raw key never in DOM)", async () => {
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <ProvidersSection />
      </Wrap>,
    );
    await waitFor(() => screen.getByText("Anthropic"));

    // bullets are shown initially (mask hidden behind dots)
    expect(document.body.textContent).toContain("•".repeat(MASK.length));
    expect(document.body.textContent).not.toContain(MASK);

    fireEvent.click(screen.getByLabelText("show key"));

    await waitFor(() => expect(document.body.textContent).toContain(MASK));
    // raw key must never appear in the DOM
    expect(document.body.textContent).not.toContain(RAW_KEY);
  });
});
