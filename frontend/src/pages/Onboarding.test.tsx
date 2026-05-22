import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Routes, Route } from "react-router-dom";
import { OnboardingPage } from "./Onboarding";
import {
  Wrap,
  makeQueryClient,
  mockFetch,
  jsonResponse,
  noContent,
} from "@/test-utils";

const emptyProviders = {
  default_provider: "anthropic",
  default_model: { anthropic: null, openai: null, openai_compat: null },
  memory_defaults: { recent_n: 10, top_k: 5 },
  appearance: { theme: "system" },
  providers: [
    { name: "anthropic", key_configured: false, key_masked: null, base_url: null },
    { name: "openai", key_configured: false, key_masked: null, base_url: null },
    { name: "openai_compat", key_configured: false, key_masked: null, base_url: null },
  ],
  key_store_backend: "keyring",
};

describe("OnboardingPage", () => {
  let putCalls: { url: string; body: string }[];

  beforeEach(() => {
    putCalls = [];
    mockFetch((url, init) => {
      if (url === "/api/settings" && (!init || init.method === undefined || init.method === "GET"))
        return jsonResponse(emptyProviders);
      if (url.startsWith("/api/settings/providers/") && init?.method === "PUT") {
        putCalls.push({ url, body: init?.body as string });
        return noContent();
      }
      return jsonResponse({}, 404);
    });
  });

  it("renders the onboarding form when no providers are configured", () => {
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <OnboardingPage />
      </Wrap>,
    );
    expect(screen.getByText(/Welcome to agent-maker/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/API key/i)).toBeInTheDocument();
  });

  it("navigates to /settings after the key is saved", async () => {
    const client = makeQueryClient();
    render(
      <Wrap client={client} initialEntries={["/onboarding"]}>
        <Routes>
          <Route path="/onboarding" element={<OnboardingPage />} />
          <Route path="/settings" element={<div data-testid="settings-page">SETTINGS</div>} />
        </Routes>
      </Wrap>,
    );
    fireEvent.change(screen.getByLabelText(/API key/i), {
      target: { value: "sk-ant-api03-FRESH123456" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Save and continue/i }));

    await waitFor(() => expect(screen.getByTestId("settings-page")).toBeInTheDocument());
    expect(putCalls).toHaveLength(1);
    expect(putCalls[0].url).toBe("/api/settings/providers/anthropic/key");
  });
});
