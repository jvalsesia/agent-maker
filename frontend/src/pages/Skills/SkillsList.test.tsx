import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { SkillsList } from "./SkillsList";
import { Wrap, makeQueryClient, mockFetch, jsonResponse } from "@/test-utils";
import { Toaster } from "sonner";

describe("SkillsList", () => {
  beforeEach(() => {
    mockFetch((url) => {
      if (url.startsWith("/api/skills")) return jsonResponse({ skills: [] });
      return jsonResponse({}, 404);
    });
  });

  it("renders the empty state with a CTA when no skills exist", async () => {
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <SkillsList />
      </Wrap>,
    );
    await waitFor(() =>
      expect(screen.getByText(/no skills yet/i)).toBeInTheDocument(),
    );
    expect(screen.getByRole("button", { name: /create skill/i })).toBeInTheDocument();
  });

  it("renders skill rows with the attached-agents badge", async () => {
    mockFetch((url) => {
      if (url.startsWith("/api/skills"))
        return jsonResponse({
          skills: [
            {
              id: "11111111-1111-1111-1111-111111111111",
              name: "Concise",
              description: "Keep replies tight",
              attached_agent_count: 2,
              created_at: "2026-01-01T00:00:00Z",
              updated_at: "2026-01-01T00:00:00Z",
            },
          ],
        });
      return jsonResponse({}, 404);
    });
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <SkillsList />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("Concise")).toBeInTheDocument());
    expect(screen.getByText("2 agents")).toBeInTheDocument();
  });
});
