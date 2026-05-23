import { describe, it, expect, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Toaster } from "sonner";
import { Route, Routes } from "react-router-dom";
import { ChatPage } from "./ChatPage";
import { Wrap, jsonResponse, makeQueryClient, mockFetch, noContent } from "@/test-utils";

const AGENT_ID = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const CONV_ID = "11111111-1111-1111-1111-111111111111";

const conv = {
  id: CONV_ID,
  agent_id: AGENT_ID,
  title: "Alpha",
  created_at: "2026-01-01T00:00:00Z",
  last_activity_at: "2026-01-02T00:00:00Z",
  message_count: 2,
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

function baseHandlers(extra?: (url: string, init?: RequestInit) => Response | undefined) {
  mockFetch((url, init) => {
    const method = (init?.method ?? "GET").toUpperCase();
    const fromExtra = extra?.(url, init);
    if (fromExtra) return fromExtra;
    if (method === "GET" && url === `/api/agents/${AGENT_ID}/conversations`) {
      return jsonResponse({ conversations: [conv] });
    }
    if (method === "GET" && url === `/api/conversations/${CONV_ID}/messages`) {
      return jsonResponse({ conversation: conv, messages: [] });
    }
    return jsonResponse({ error: { code: "not_found", message: "nope" } }, 404);
  });
}

describe("ClearMemoryDialog (via ConversationSidebar)", () => {
  beforeEach(() => {
    baseHandlers();
  });

  it("opens the dialog, shows the embedded turn count, and clears on confirm", async () => {
    let cleared = false;
    baseHandlers((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (method === "GET" && url === `/api/conversations/${CONV_ID}/memory`) {
        return jsonResponse({ embedded: cleared ? 0 : 7 });
      }
      if (method === "DELETE" && url === `/api/conversations/${CONV_ID}/memory`) {
        cleared = true;
        return jsonResponse({ removed: 7 });
      }
      return undefined;
    });

    renderChat();
    await waitFor(() => expect(screen.getByText("Alpha")).toBeInTheDocument());

    fireEvent.click(screen.getByLabelText(/clear memory/i));
    expect(await screen.findByText(/clear long-term memory/i)).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText(/embedded turns:\s*7/i)).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: /clear memory/i }));
    await waitFor(() => expect(cleared).toBe(true));
    await waitFor(() => expect(screen.getByText(/cleared 7 embedded turns/i)).toBeInTheDocument());
  });

  it("surfaces an error toast on DELETE 500 and keeps the conversation visible", async () => {
    baseHandlers((url, init) => {
      const method = (init?.method ?? "GET").toUpperCase();
      if (method === "GET" && url === `/api/conversations/${CONV_ID}/memory`) {
        return jsonResponse({ embedded: 3 });
      }
      if (method === "DELETE" && url === `/api/conversations/${CONV_ID}/memory`) {
        return jsonResponse(
          { error: { code: "database_error", message: "boom" } },
          500,
        );
      }
      return undefined;
    });

    renderChat();
    await waitFor(() => expect(screen.getByText("Alpha")).toBeInTheDocument());

    fireEvent.click(screen.getByLabelText(/clear memory/i));
    await waitFor(() => expect(screen.getByText(/embedded turns:\s*3/i)).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: /clear memory/i }));

    await waitFor(() => expect(screen.getByText(/boom/i)).toBeInTheDocument());
    expect(screen.getAllByText("Alpha").length).toBeGreaterThan(0);
  });
});

// keeps the unused `noContent` import-tree benign
void noContent;
