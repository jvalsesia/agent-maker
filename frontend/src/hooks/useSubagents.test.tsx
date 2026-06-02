import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import type { ReactNode } from "react";
import {
  useAttachedSubagents,
  useAttachSubagent,
  useDetachSubagent,
} from "./useSubagents";
import { Wrap, makeQueryClient, mockFetch, jsonResponse, noContent } from "@/test-utils";

const AGENT_ID = "11111111-1111-1111-1111-111111111111";
const CHILD = "22222222-2222-2222-2222-222222222222";

function row() {
  return { child_id: CHILD, name: "Code Reviewer", alias: "code-reviewer", description: null, position: 0 };
}

describe("useSubagents", () => {
  beforeEach(() => {
    mockFetch((url, init) => {
      if (url === `/api/agents/${AGENT_ID}/subagents` && (!init?.method || init.method === "GET")) {
        return jsonResponse({ attached: [row()] });
      }
      return jsonResponse({}, 404);
    });
  });

  it("lists attached sub-agents", async () => {
    const client = makeQueryClient();
    const { result } = renderHook(() => useAttachedSubagents(AGENT_ID), {
      wrapper: ({ children }: { children: ReactNode }) => <Wrap client={client}>{children}</Wrap>,
    });
    await waitFor(() => expect(result.current.data?.attached).toHaveLength(1));
    expect(result.current.data?.attached[0].alias).toBe("code-reviewer");
  });

  it("invalidates the list cache after attach", async () => {
    let getCount = 0;
    mockFetch((url, init) => {
      if (url === `/api/agents/${AGENT_ID}/subagents` && (!init?.method || init.method === "GET")) {
        getCount += 1;
        return jsonResponse({ attached: getCount > 1 ? [row()] : [] });
      }
      if (url === `/api/agents/${AGENT_ID}/subagents` && init?.method === "POST") {
        return jsonResponse({ attached: row() }, 201);
      }
      return jsonResponse({}, 404);
    });

    const client = makeQueryClient();
    const { result } = renderHook(
      () => ({ list: useAttachedSubagents(AGENT_ID), attach: useAttachSubagent(AGENT_ID) }),
      { wrapper: ({ children }: { children: ReactNode }) => <Wrap client={client}>{children}</Wrap> },
    );
    await waitFor(() => expect(result.current.list.data?.attached).toHaveLength(0));

    await act(async () => {
      await result.current.attach.mutateAsync({ child_id: CHILD, alias: "code-reviewer" });
    });

    await waitFor(() => expect(result.current.list.data?.attached).toHaveLength(1));
  });

  it("invalidates the list cache after detach", async () => {
    const calls: string[] = [];
    let getCount = 0;
    mockFetch((url, init) => {
      if (url === `/api/agents/${AGENT_ID}/subagents` && (!init?.method || init.method === "GET")) {
        getCount += 1;
        return jsonResponse({ attached: getCount > 1 ? [] : [row()] });
      }
      if (url === `/api/agents/${AGENT_ID}/subagents/${CHILD}` && init?.method === "DELETE") {
        calls.push("delete");
        return noContent();
      }
      return jsonResponse({}, 404);
    });

    const client = makeQueryClient();
    const { result } = renderHook(
      () => ({ list: useAttachedSubagents(AGENT_ID), detach: useDetachSubagent(AGENT_ID) }),
      { wrapper: ({ children }: { children: ReactNode }) => <Wrap client={client}>{children}</Wrap> },
    );
    await waitFor(() => expect(result.current.list.data?.attached).toHaveLength(1));

    await act(async () => {
      await result.current.detach.mutateAsync(CHILD);
    });

    expect(calls).toContain("delete");
    await waitFor(() => expect(result.current.list.data?.attached).toHaveLength(0));
  });
});
