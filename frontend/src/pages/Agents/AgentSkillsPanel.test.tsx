import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { AgentSkillsPanel } from "./AgentSkillsPanel";
import { Wrap, makeQueryClient, mockFetch, jsonResponse, noContent } from "@/test-utils";
import { Toaster } from "sonner";

const AGENT_ID = "11111111-1111-1111-1111-111111111111";
const S1 = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaa1";
const S2 = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaa2";

function attachedPayload(rows: { skill_id: string; name: string; description: string; position: number }[]) {
  return { attached: rows };
}

function composePayload(fraction: number) {
  return {
    composed: "x",
    length_chars: Math.round(fraction * 800000),
    model_context_chars: 800000,
    fraction,
    warning: fraction >= 0.8 ? "composed prompt is large" : null,
  };
}

describe("AgentSkillsPanel", () => {
  beforeEach(() => {
    mockFetch((url) => {
      if (url === `/api/agents/${AGENT_ID}/skills`) {
        return jsonResponse(
          attachedPayload([
            { skill_id: S1, name: "Concise", description: "Keep it tight", position: 0 },
            { skill_id: S2, name: "Cite Sources", description: "Cite always", position: 1 },
          ]),
        );
      }
      if (url === `/api/agents/${AGENT_ID}/compose`) {
        return jsonResponse(composePayload(0.05));
      }
      if (url === "/api/skills") {
        return jsonResponse({
          skills: [
            { id: S1, name: "Concise", description: "Keep it tight", attached_agent_count: 1, created_at: "", updated_at: "" },
            { id: S2, name: "Cite Sources", description: "Cite always", attached_agent_count: 1, created_at: "", updated_at: "" },
          ],
        });
      }
      return jsonResponse({}, 404);
    });
  });

  it("renders attached skills in order", async () => {
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <AgentSkillsPanel agentId={AGENT_ID} />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Concise")).toBeInTheDocument());
    expect(screen.getByText("Cite Sources")).toBeInTheDocument();
    expect(screen.getByText(/\(5%\)/)).toBeInTheDocument();
  });

  it("detaches a skill via the per-row button", async () => {
    const calls: { url: string; method?: string }[] = [];
    mockFetch((url, init) => {
      calls.push({ url, method: init?.method });
      if (url === `/api/agents/${AGENT_ID}/skills` && (!init || !init.method || init.method === "GET")) {
        return jsonResponse(
          attachedPayload([
            { skill_id: S1, name: "Concise", description: "Keep it tight", position: 0 },
          ]),
        );
      }
      if (url === `/api/agents/${AGENT_ID}/skills/${S1}` && init?.method === "DELETE") {
        return noContent();
      }
      if (url === `/api/agents/${AGENT_ID}/compose`) {
        return jsonResponse(composePayload(0.05));
      }
      if (url === "/api/skills") {
        return jsonResponse({ skills: [] });
      }
      return jsonResponse({}, 404);
    });

    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <AgentSkillsPanel agentId={AGENT_ID} />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Concise")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: /detach concise/i }));

    await waitFor(() =>
      expect(
        calls.some(
          (c) => c.url === `/api/agents/${AGENT_ID}/skills/${S1}` && c.method === "DELETE",
        ),
      ).toBe(true),
    );
  });

  it("renders a yellow warning when composed fraction is high", async () => {
    mockFetch((url) => {
      if (url === `/api/agents/${AGENT_ID}/skills`) {
        return jsonResponse(attachedPayload([]));
      }
      if (url === `/api/agents/${AGENT_ID}/compose`) {
        return jsonResponse(composePayload(0.87));
      }
      if (url === "/api/skills") return jsonResponse({ skills: [] });
      return jsonResponse({}, 404);
    });
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <AgentSkillsPanel agentId={AGENT_ID} />
      </Wrap>,
    );
    await waitFor(() =>
      expect(screen.getByText(/87%/)).toBeInTheDocument(),
    );
    expect(screen.getByText(/composed prompt is large/i)).toBeInTheDocument();
  });

  it("sends merged ordered list when attaching from the modal", async () => {
    let putBody: any = null;
    mockFetch((url, init) => {
      if (url === `/api/agents/${AGENT_ID}/skills` && init?.method === "PUT") {
        putBody = JSON.parse(init.body as string);
        return jsonResponse({ attached: [], warnings: [] });
      }
      if (url === `/api/agents/${AGENT_ID}/skills`) {
        return jsonResponse(attachedPayload([]));
      }
      if (url === `/api/agents/${AGENT_ID}/compose`) {
        return jsonResponse(composePayload(0.01));
      }
      if (url === "/api/skills") {
        return jsonResponse({
          skills: [
            { id: S2, name: "Cite Sources", description: "Cite always", attached_agent_count: 0, created_at: "", updated_at: "" },
            { id: S1, name: "Concise", description: "Keep it tight", attached_agent_count: 0, created_at: "", updated_at: "" },
          ],
        });
      }
      return jsonResponse({}, 404);
    });

    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <AgentSkillsPanel agentId={AGENT_ID} />
      </Wrap>,
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: /attach skills/i })).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: /attach skills/i }));

    await waitFor(() => expect(screen.getByText("Cite Sources")).toBeInTheDocument());

    const checkboxes = screen.getAllByRole("checkbox");
    fireEvent.click(checkboxes[0]);
    fireEvent.click(checkboxes[1]);

    fireEvent.click(screen.getByRole("button", { name: /attach selected/i }));

    await waitFor(() => expect(putBody).not.toBeNull());
    // Newly checked skills are appended in alphabetical order: Cite Sources, then Concise.
    expect(putBody.skill_ids).toEqual([S2, S1]);
  });
});
