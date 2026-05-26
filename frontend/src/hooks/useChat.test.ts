import { describe, expect, it } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { mockFetch } from "@/test-utils";
import { useChat } from "./useChat";

function sseResponse(events: unknown[]): Response {
  const text = events.map((e) => `data: ${JSON.stringify(e)}\n\n`).join("");
  const stream = new ReadableStream<Uint8Array>({
    start(c) {
      c.enqueue(new TextEncoder().encode(text));
      c.close();
    },
  });
  return new Response(stream, {
    status: 200,
    headers: { "content-type": "text/event-stream" },
  });
}

function wrapper() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: ReactNode }) =>
    createElement(QueryClientProvider, { client }, children);
}

describe("useChat", () => {
  it("posts the message to the chat endpoint and streams to completion", async () => {
    const fetchMock = mockFetch(() =>
      sseResponse([
        { type: "meta", user_message_id: "u1", assistant_message_id: "a1", model: "gpt-4o", degraded: false, recalled: [] },
        { type: "chunk", content: "Hello " },
        { type: "chunk", content: "world" },
        { type: "done", status: "complete", token_count: 5 },
      ]),
    );

    const { result } = renderHook(() => useChat("c1", "agent1"), { wrapper: wrapper() });
    await act(async () => {
      result.current.send("hi there");
    });
    await waitFor(() => expect(result.current.streaming).toBe(false));

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/conversations/c1/chat");
    // The active UI locale rides along so the backend can resolve an `auto`
    // agent's response language; default runtime locale is English.
    expect(JSON.parse((init as RequestInit).body as string)).toEqual({
      content: "hi there",
      locale: "en",
    });
  });

  it("captures a mid-stream provider error", async () => {
    mockFetch(() =>
      sseResponse([
        { type: "meta", user_message_id: "u1", assistant_message_id: "a1", model: "gpt-4o", degraded: false, recalled: [] },
        { type: "error", code: "provider_error", message: "boom" },
      ]),
    );

    const { result } = renderHook(() => useChat("c1", "agent1"), { wrapper: wrapper() });
    await act(async () => {
      result.current.send("hi");
    });
    await waitFor(() => expect(result.current.streaming).toBe(false));
    expect(result.current.error?.message).toBe("boom");
    expect(result.current.error?.code).toBe("provider_error");
  });
});
