import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SubagentsSection } from "./SubagentsSection";
import { Wrap, makeQueryClient, mockFetch, jsonResponse, noContent } from "@/test-utils";
import { Toaster } from "sonner";

const AGENT_ID = "11111111-1111-1111-1111-111111111111";
const CHILD = "22222222-2222-2222-2222-222222222222";

function attached(rows: { child_id: string; name: string; alias: string; description: string | null; position: number }[]) {
  return { attached: rows };
}

describe("SubagentsSection", () => {
  beforeEach(() => {
    mockFetch((url) => {
      if (url === `/api/agents/${AGENT_ID}/subagents`) {
        return jsonResponse(
          attached([
            { child_id: CHILD, name: "Code Reviewer", alias: "code-reviewer", description: "review", position: 0 },
          ]),
        );
      }
      if (url === "/api/agents") {
        return jsonResponse({ agents: [] });
      }
      return jsonResponse({}, 404);
    });
  });

  it("renders an attached sub-agent with its @handle", async () => {
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <SubagentsSection agentId={AGENT_ID} />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Code Reviewer")).toBeInTheDocument());
    expect(screen.getByText("@code-reviewer")).toBeInTheDocument();
  });

  it("detaches a sub-agent via the per-row button", async () => {
    const calls: { url: string; method?: string }[] = [];
    mockFetch((url, init) => {
      calls.push({ url, method: init?.method });
      if (url === `/api/agents/${AGENT_ID}/subagents` && (!init?.method || init.method === "GET")) {
        return jsonResponse(
          attached([
            { child_id: CHILD, name: "Code Reviewer", alias: "code-reviewer", description: null, position: 0 },
          ]),
        );
      }
      if (url === `/api/agents/${AGENT_ID}/subagents/${CHILD}` && init?.method === "DELETE") {
        return noContent();
      }
      if (url === "/api/agents") return jsonResponse({ agents: [] });
      return jsonResponse({}, 404);
    });

    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <SubagentsSection agentId={AGENT_ID} />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Code Reviewer")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: /detach code reviewer/i }));

    await waitFor(() =>
      expect(
        calls.some(
          (c) => c.url === `/api/agents/${AGENT_ID}/subagents/${CHILD}` && c.method === "DELETE",
        ),
      ).toBe(true),
    );
  });

  it("opens the picker and excludes the parent agent from candidates", async () => {
    mockFetch((url) => {
      if (url === `/api/agents/${AGENT_ID}/subagents`) {
        return jsonResponse(attached([]));
      }
      if (url === "/api/agents") {
        return jsonResponse({
          agents: [
            { id: AGENT_ID, name: "Self Parent", preamble: null, provider: "anthropic", model: "m", has_override_key: false, last_used_at: null, created_at: "", attached_skill_count: 0, conversation_count: 0 },
            { id: CHILD, name: "Other Agent", preamble: null, provider: "anthropic", model: "m", has_override_key: false, last_used_at: null, created_at: "", attached_skill_count: 0, conversation_count: 0 },
          ],
        });
      }
      return jsonResponse({}, 404);
    });

    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <SubagentsSection agentId={AGENT_ID} />
      </Wrap>,
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: /attach sub-agent/i })).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: /attach sub-agent/i }));

    await waitFor(() => expect(screen.getByText("Other Agent")).toBeInTheDocument());
    // The parent agent must not appear as a candidate (cannot attach to itself).
    expect(screen.queryByText("Self Parent")).not.toBeInTheDocument();
  });
});
