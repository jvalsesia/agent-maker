import { describe, it, expect, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Toaster } from "sonner";
import { Route, Routes } from "react-router-dom";
import { ChatPage } from "./ChatPage";
import { Wrap, jsonResponse, makeQueryClient, mockFetch, noContent } from "@/test-utils";

const AGENT_ID = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

const convA = {
  id: "11111111-1111-1111-1111-111111111111",
  agent_id: AGENT_ID,
  title: "Alpha",
  created_at: "2026-01-01T00:00:00Z",
  last_activity_at: "2026-01-02T00:00:00Z",
  message_count: 2,
};
const convB = {
  id: "22222222-2222-2222-2222-222222222222",
  agent_id: AGENT_ID,
  title: "Beta",
  created_at: "2026-01-01T00:00:00Z",
  last_activity_at: "2026-01-01T00:00:00Z",
  message_count: 0,
};

function renderChat() {
  return render(
    <Wrap client={makeQueryClient()} initialEntries={[`/agents/${AGENT_ID}/chat`]}>
      <Toaster />
      <Routes>
        <Route path="/agents/:id/chat" element={<ChatPage />} />
      </Routes>
    </Wrap>,
  );
}

function listResp(convs: typeof convA[]) {
  return jsonResponse({ conversations: convs });
}

function messagesResp(convId: string, content: string[] = []) {
  return jsonResponse({
    conversation: { ...convA, id: convId },
    messages: content.map((c, i) => ({
      id: `m-${i}`,
      conversation_id: convId,
      role: i % 2 === 0 ? "user" : "assistant",
      content: c,
      status: "complete",
      model: null,
      token_count: null,
      created_at: `2026-01-01T00:00:0${i}Z`,
    })),
  });
}

describe("ChatPage", () => {
  beforeEach(() => {
    mockFetch((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (method === "GET" && url === `/api/agents/${AGENT_ID}/conversations`) {
        return listResp([convA, convB]);
      }
      if (method === "GET" && url === `/api/conversations/${convA.id}/messages`) {
        return messagesResp(convA.id, ["hello", "hi back"]);
      }
      if (method === "GET" && url === `/api/conversations/${convB.id}/messages`) {
        return messagesResp(convB.id, []);
      }
      return jsonResponse({ error: { code: "not_found", message: "nope" } }, 404);
    });
  });

  it("renders the sidebar with the conversation list and activates the first row", async () => {
    renderChat();
    await waitFor(() => expect(screen.getByText("Alpha")).toBeInTheDocument());
    expect(screen.getByText("Beta")).toBeInTheDocument();
    // Active conversation's messages render
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());
    expect(screen.getByText("hi back")).toBeInTheDocument();
  });

  it("creates a new conversation and selects it", async () => {
    const newConv = {
      ...convA,
      id: "33333333-3333-3333-3333-333333333333",
      title: "New conversation",
      message_count: 0,
    };
    mockFetch((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (method === "GET" && url === `/api/agents/${AGENT_ID}/conversations`) {
        // First load returns the two original, second load (after invalidation) includes the new one
        if ((globalThis as { __created?: boolean }).__created) {
          return listResp([newConv, convA, convB]);
        }
        return listResp([convA, convB]);
      }
      if (method === "POST" && url === `/api/agents/${AGENT_ID}/conversations`) {
        (globalThis as { __created?: boolean }).__created = true;
        return jsonResponse({ conversation: newConv, warnings: [] }, 201);
      }
      if (method === "GET" && url === `/api/conversations/${newConv.id}/messages`) {
        return messagesResp(newConv.id, []);
      }
      if (method === "GET" && url === `/api/conversations/${convA.id}/messages`) {
        return messagesResp(convA.id, ["hello"]);
      }
      return jsonResponse({}, 404);
    });

    renderChat();
    await waitFor(() => expect(screen.getByText("Alpha")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /new conversation/i }));
    await waitFor(() => expect(screen.getByText("New conversation")).toBeInTheDocument());
  });

  it("renames a conversation via inline edit", async () => {
    let renamedBody: { title?: string } = {};
    mockFetch((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (method === "GET" && url === `/api/agents/${AGENT_ID}/conversations`) {
        if (renamedBody.title) {
          return listResp([{ ...convA, title: renamedBody.title }, convB]);
        }
        return listResp([convA, convB]);
      }
      if (method === "PATCH" && url === `/api/conversations/${convA.id}`) {
        renamedBody = JSON.parse(init?.body as string);
        return jsonResponse({
          conversation: { ...convA, title: renamedBody.title },
          warnings: [],
        });
      }
      if (method === "GET" && url === `/api/conversations/${convA.id}/messages`) {
        return messagesResp(convA.id, []);
      }
      return jsonResponse({}, 404);
    });

    renderChat();
    await waitFor(() => expect(screen.getByText("Alpha")).toBeInTheDocument());

    // Each conversation row exposes a Rename and Delete labelled button.
    // Alpha is the first row, so its rename is the first one.
    const renameButtons = screen.getAllByLabelText(/rename/i);
    fireEvent.click(renameButtons[0]);
    const input = await screen.findByDisplayValue("Alpha");
    fireEvent.change(input, { target: { value: "Renamed" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(screen.getByText("Renamed")).toBeInTheDocument());
    expect(renamedBody.title).toBe("Renamed");
  });

  it("deletes a conversation through the confirmation dialog", async () => {
    let deleted = false;
    mockFetch((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (method === "GET" && url === `/api/agents/${AGENT_ID}/conversations`) {
        if (deleted) return listResp([convB]);
        return listResp([convA, convB]);
      }
      if (method === "DELETE" && url === `/api/conversations/${convA.id}`) {
        deleted = true;
        return noContent();
      }
      if (method === "GET" && url === `/api/conversations/${convA.id}/messages`) {
        return messagesResp(convA.id, []);
      }
      if (method === "GET" && url === `/api/conversations/${convB.id}/messages`) {
        return messagesResp(convB.id, []);
      }
      return jsonResponse({}, 404);
    });

    renderChat();
    await waitFor(() => expect(screen.getByText("Alpha")).toBeInTheDocument());
    const deleteButtons = screen.getAllByLabelText(/delete/i);
    fireEvent.click(deleteButtons[0]);
    expect(screen.getByText(/2 messages/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /^delete$/i }));

    await waitFor(() => expect(screen.queryByText("Alpha")).toBeNull());
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("preserves a draft across conversation switches", async () => {
    renderChat();
    await waitFor(() => expect(screen.getByText("Alpha")).toBeInTheDocument());
    // Wait until messages query has settled and composer is rendered
    await waitFor(() => expect(screen.getByText("hello")).toBeInTheDocument());

    const composer = () => screen.getByPlaceholderText(/composer arrives with F07/i) as HTMLTextAreaElement;
    fireEvent.change(composer(), { target: { value: "draft for Alpha" } });

    // Switch to Beta
    fireEvent.click(screen.getByText("Beta"));
    await waitFor(() => expect(composer().value).toBe(""));
    fireEvent.change(composer(), { target: { value: "draft for Beta" } });

    // Switch back to Alpha
    fireEvent.click(screen.getByText("Alpha"));
    await waitFor(() => expect(composer().value).toBe("draft for Alpha"));
  });
});
