import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SkillForm } from "./SkillForm";
import { Wrap, makeQueryClient, mockFetch, jsonResponse } from "@/test-utils";
import { Toaster } from "sonner";

describe("SkillForm (create mode)", () => {
  beforeEach(() => {
    mockFetch(() => jsonResponse({}, 404));
  });

  it("renders the field set with no skill loaded", async () => {
    const client = makeQueryClient();
    render(
      <Wrap client={client}>
        <Toaster />
        <SkillForm />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText(/new skill/i)).toBeInTheDocument());
    expect(screen.getByText("Name")).toBeInTheDocument();
    expect(screen.getByText("Description")).toBeInTheDocument();
    expect(screen.getByText("Instruction body")).toBeInTheDocument();
  });

  it("surfaces an inline field error from a 400 response on save", async () => {
    mockFetch((url, init) => {
      if (url === "/api/skills" && init?.method === "POST") {
        return jsonResponse(
          {
            error: {
              code: "validation_error",
              message: "a skill with this name already exists",
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
        <SkillForm />
      </Wrap>,
    );

    await waitFor(() => screen.getByText(/new skill/i));

    fireEvent.change(screen.getByPlaceholderText(/concise replies/i), {
      target: { value: "Dup" },
    });
    fireEvent.change(screen.getByPlaceholderText(/one-line summary/i), {
      target: { value: "A description" },
    });
    fireEvent.change(screen.getByPlaceholderText(/prefer short sentences/i), {
      target: { value: "Be concise. Use short sentences. No filler. Bullet lists welcome." },
    });

    fireEvent.click(screen.getAllByRole("button", { name: /^save$/i })[0]);

    await waitFor(() =>
      expect(screen.getByText(/already exists/i)).toBeInTheDocument(),
    );
  });
});
