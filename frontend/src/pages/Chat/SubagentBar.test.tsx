import { describe, it, expect, beforeEach, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Toaster } from "sonner";
import { SubagentBar } from "./SubagentBar";
import { Wrap, jsonResponse, makeQueryClient, mockFetch, noContent } from "@/test-utils";

const PARENT = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const CHILD = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

const reviewer = {
  child_id: CHILD,
  name: "Code Reviewer",
  alias: "code-reviewer",
  description: "Use for code review",
  position: 0,
};

const childAgentSummary = {
  id: CHILD,
  name: "Code Reviewer",
  preamble: "Reviews code",
  provider: "anthropic",
  model: "claude-opus-4-8",
  has_override_key: false,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  last_used_at: null,
};

function renderBar(onPick = vi.fn()) {
  render(
    <Wrap client={makeQueryClient()}>
      <Toaster />
      <SubagentBar agentId={PARENT} onPick={onPick} />
    </Wrap>,
  );
  return { onPick };
}

describe("SubagentBar", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("stays visible with an Attach control when the agent has no sub-agents", async () => {
    mockFetch((url) => {
      if (url.includes("/subagents")) return jsonResponse({ attached: [] });
      return jsonResponse({ agents: [] });
    });
    renderBar();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: /attach sub-agent/i })).toBeInTheDocument(),
    );
  });

  it("inserts the attached @handle into the composer on a successful attach", async () => {
    mockFetch((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (url.includes("/subagents") && method === "POST") {
        return jsonResponse({ attached: reviewer }, 201);
      }
      if (url.includes("/subagents")) return jsonResponse({ attached: [] });
      if (url.startsWith("/api/agents")) return jsonResponse({ agents: [childAgentSummary] });
      return jsonResponse({}, 404);
    });
    const { onPick } = renderBar();

    fireEvent.click(await screen.findByRole("button", { name: /attach sub-agent/i }));
    // Pick the candidate in the picker, then confirm.
    fireEvent.click(await screen.findByRole("radio"));
    fireEvent.click(screen.getByRole("button", { name: /^attach$/i }));

    await waitFor(() => expect(onPick).toHaveBeenCalledWith("code-reviewer"));
  });

  it("inserts nothing when the attach is rejected (e.g. cycle/cap)", async () => {
    mockFetch((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (url.includes("/subagents") && method === "POST") {
        return jsonResponse(
          { error: { code: "validation", message: "would create a cycle" } },
          422,
        );
      }
      if (url.includes("/subagents")) return jsonResponse({ attached: [] });
      if (url.startsWith("/api/agents")) return jsonResponse({ agents: [childAgentSummary] });
      return jsonResponse({}, 404);
    });
    const { onPick } = renderBar();

    fireEvent.click(await screen.findByRole("button", { name: /attach sub-agent/i }));
    fireEvent.click(await screen.findByRole("radio"));
    fireEvent.click(screen.getByRole("button", { name: /^attach$/i }));

    // The error toast surfaces and onPick is never called.
    await waitFor(() => expect(screen.getByText(/would create a cycle/i)).toBeInTheDocument());
    expect(onPick).not.toHaveBeenCalled();
  });

  it("detaches a sub-agent through the chip's detach control", async () => {
    let detached = false;
    mockFetch((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (url.includes("/subagents") && method === "DELETE") {
        detached = true;
        return noContent();
      }
      if (url.includes("/subagents")) {
        return jsonResponse({ attached: detached ? [] : [reviewer] });
      }
      if (url.startsWith("/api/agents")) return jsonResponse({ agents: [childAgentSummary] });
      return jsonResponse({}, 404);
    });
    renderBar();

    fireEvent.click(await screen.findByRole("button", { name: /detach code reviewer/i }));
    await waitFor(() => expect(detached).toBe(true));
    await waitFor(() =>
      expect(screen.queryByText("@code-reviewer")).not.toBeInTheDocument(),
    );
  });
});
