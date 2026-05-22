import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { AgentForm } from "./AgentForm";
import { Wrap, makeQueryClient, mockFetch, jsonResponse } from "@/test-utils";
import { Toaster } from "sonner";

const settings = {
  default_provider: "anthropic",
  default_model: { anthropic: null, openai: null, openai_compat: null },
  memory_defaults: { recent_n: 10, top_k: 5 },
  appearance: { theme: "system" },
  providers: [
    { name: "anthropic", key_configured: true, key_masked: "sk-ant-***...ABCD", base_url: null },
    { name: "openai", key_configured: false, key_masked: null, base_url: null },
    { name: "openai_compat", key_configured: false, key_masked: null, base_url: null },
  ],
  key_store_backend: "keyring",
};

const models = { provider: "anthropic", models: ["claude-haiku-4-5"] };

describe("AgentForm (create mode)", () => {
  beforeEach(() => {
    mockFetch((url) => {
      if (url === "/api/settings") return jsonResponse(settings);
      if (url.startsWith("/api/agents/models")) return jsonResponse(models);
      return jsonResponse({}, 404);
    });
  });

  it("renders the field set with no agent loaded", async () => {
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <AgentForm />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText(/new agent/i)).toBeInTheDocument());
    expect(screen.getByText("System prompt")).toBeInTheDocument();
    expect(screen.getByText("Provider")).toBeInTheDocument();
    expect(screen.getByText("Model")).toBeInTheDocument();
  });

  it("surfaces an inline field error from a 400 response on save", async () => {
    mockFetch((url, init) => {
      if (url === "/api/settings") return jsonResponse(settings);
      if (url.startsWith("/api/agents/models")) return jsonResponse(models);
      if (url === "/api/agents" && init?.method === "POST") {
        return jsonResponse(
          {
            error: {
              code: "validation_error",
              message: "an agent with this name already exists",
              field: "name",
            },
          },
          400,
        );
      }
      return jsonResponse({}, 404);
    });
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <AgentForm />
      </Wrap>,
    );

    await waitFor(() => screen.getByText(/new agent/i));

    fireEvent.change(screen.getByPlaceholderText(/e\.g\. writing editor/i), {
      target: { value: "Editor" },
    });
    fireEvent.change(screen.getByPlaceholderText(/you are a/i), {
      target: { value: "x".repeat(120) },
    });
    // provider defaults to anthropic; pick the only model
    // (we cannot easily drive the radix Select via clicks here, so directly call save —
    // the form will hit the mocked POST and surface the field error)
    fireEvent.click(screen.getAllByRole("button", { name: /^save$/i })[0]);

    await waitFor(() =>
      expect(screen.getByText(/already exists/i)).toBeInTheDocument(),
    );
  });
});
