import { describe, it, expect, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { Toaster } from "sonner";
import { TemplatesGallery } from "./TemplatesGallery";
import { Wrap, jsonResponse, makeQueryClient, mockFetch } from "@/test-utils";

const writingEditor = {
  slug: "writing-editor",
  name: "Writing Editor",
  category: "writing",
  preamble: "A sharp, opinionated writing editor.",
  suggested_skills: ["blunt-editor"],
};
const sqlBuddy = {
  slug: "sql-buddy",
  name: "SQL Buddy",
  category: "coding",
  preamble: "Pairs on SQL queries.",
  suggested_skills: ["sql-helper"],
};
const concise = {
  slug: "concise-replies",
  name: "Concise Replies",
  category: "writing",
  description: "Keep responses tight.",
};
const sqlHelper = {
  slug: "sql-helper",
  name: "SQL Helper",
  category: "coding",
  description: "Explain SQL.",
};

function listOk() {
  return jsonResponse({
    agents: [writingEditor, sqlBuddy],
    skills: [concise, sqlHelper],
  });
}

describe("TemplatesGallery", () => {
  beforeEach(() => {
    mockFetch((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (method === "GET" && url.startsWith("/api/templates") && !url.includes("/agents/") && !url.includes("/skills/")) {
        return listOk();
      }
      if (method === "GET" && url === "/api/templates/agents/writing-editor") {
        return jsonResponse({
          agent: {
            ...writingEditor,
            system_prompt: "You are a sharp editor.",
            default_provider: "anthropic",
            default_model: "claude-haiku-4-5",
            suggested_skills: [
              { slug: "blunt-editor", name: "Blunt Editor", description: "Direct edits." },
            ],
          },
        });
      }
      if (method === "POST" && url === "/api/templates/agents/writing-editor/adopt") {
        return jsonResponse({
          agent: {
            id: "11111111-1111-1111-1111-111111111111",
            name: "Writing Editor",
            preamble: writingEditor.preamble,
            system_prompt: "You are a sharp editor.",
            provider: "anthropic",
            model: "claude-haiku-4-5",
            has_override_key: false,
            recent_n_override: null,
            top_k_override: null,
            created_at: "2026-01-01T00:00:00Z",
            updated_at: "2026-01-01T00:00:00Z",
            last_used_at: null,
          },
          attached_skill_ids: ["22222222-2222-2222-2222-222222222222"],
          warnings: [],
        });
      }
      return jsonResponse({ error: { code: "not_found", message: "nope" } }, 404);
    });
  });

  it("renders the 2+2 templates from a mocked GET", async () => {
    render(
      <Wrap client={makeQueryClient()}>
        <Toaster />
        <TemplatesGallery />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Writing Editor")).toBeInTheDocument());
    expect(screen.getByText("SQL Buddy")).toBeInTheDocument();
    expect(screen.getByText("Concise Replies")).toBeInTheDocument();
    expect(screen.getByText("SQL Helper")).toBeInTheDocument();
  });

  it("filters by category chip", async () => {
    render(
      <Wrap client={makeQueryClient()}>
        <Toaster />
        <TemplatesGallery />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Writing Editor")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /^coding$/i }));
    expect(screen.queryByText("Writing Editor")).toBeNull();
    expect(screen.queryByText("Concise Replies")).toBeNull();
    expect(screen.getByText("SQL Buddy")).toBeInTheDocument();
    expect(screen.getByText("SQL Helper")).toBeInTheDocument();
  });

  it("narrows by search text", async () => {
    render(
      <Wrap client={makeQueryClient()}>
        <Toaster />
        <TemplatesGallery />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Writing Editor")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText(/search templates/i), {
      target: { value: "concise" },
    });
    expect(screen.getByText("Concise Replies")).toBeInTheDocument();
    expect(screen.queryByText("Writing Editor")).toBeNull();
    expect(screen.queryByText("SQL Buddy")).toBeNull();
  });

  it("opens the preview drawer with prompt body", async () => {
    render(
      <Wrap client={makeQueryClient()}>
        <Toaster />
        <TemplatesGallery />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Writing Editor")).toBeInTheDocument());

    const card = screen.getByText("Writing Editor").closest("div")!;
    fireEvent.click(within(card.parentElement!).getByRole("button", { name: /preview/i }));
    await waitFor(() =>
      expect(screen.getByText("You are a sharp editor.")).toBeInTheDocument(),
    );
    expect(screen.getByText("Blunt Editor")).toBeInTheDocument();
  });

  it("adopts an agent and shows a success toast with Open agent shortcut", async () => {
    render(
      <Wrap client={makeQueryClient()}>
        <Toaster />
        <TemplatesGallery />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Writing Editor")).toBeInTheDocument());
    const card = screen.getByText("Writing Editor").closest("div")!;
    fireEvent.click(within(card.parentElement!).getByRole("button", { name: /adopt/i }));
    await waitFor(() =>
      expect(screen.getByText(/Adopted "Writing Editor"/)).toBeInTheDocument(),
    );
    expect(screen.getByRole("button", { name: /open agent/i })).toBeInTheDocument();
  });
});
