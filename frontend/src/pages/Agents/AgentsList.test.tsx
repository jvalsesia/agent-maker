import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { AgentsList } from "./AgentsList";
import { Wrap, makeQueryClient, mockFetch, jsonResponse } from "@/test-utils";
import { Toaster } from "sonner";

describe("AgentsList", () => {
  beforeEach(() => {
    mockFetch((url) => {
      if (url.startsWith("/api/agents")) return jsonResponse({ agents: [] });
      return jsonResponse({}, 404);
    });
  });

  it("renders the empty state with a CTA when no agents exist", async () => {
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <AgentsList />
      </Wrap>,
    );
    await waitFor(() =>
      expect(screen.getByText(/no agents yet/i)).toBeInTheDocument(),
    );
    expect(screen.getByRole("button", { name: /create agent/i })).toBeInTheDocument();
  });

  it("renders agent rows when the API returns agents", async () => {
    mockFetch((url) => {
      if (url.startsWith("/api/agents"))
        return jsonResponse({
          agents: [
            {
              id: "11111111-1111-1111-1111-111111111111",
              name: "Editor",
              preamble: "A sharp editor",
              provider: "anthropic",
              model: "claude-haiku-4-5",
              has_override_key: false,
              last_used_at: null,
              created_at: "2026-01-01T00:00:00Z",
              attached_skill_count: 0,
              conversation_count: 0,
              subagents: [{ alias: "code-reviewer", name: "Code Reviewer" }],
            },
          ],
        });
      return jsonResponse({}, 404);
    });
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <AgentsList />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Editor")).toBeInTheDocument());
    expect(screen.getByText("anthropic:claude-haiku-4-5")).toBeInTheDocument();
    expect(screen.getByText("@code-reviewer")).toBeInTheDocument();
  });
});
