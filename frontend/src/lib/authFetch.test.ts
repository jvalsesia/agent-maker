import { afterEach, describe, expect, it, vi } from "vitest";
import { authFetch } from "@/lib/authFetch";

function setFetch(impl: typeof fetch) {
  (globalThis as { fetch: typeof fetch }).fetch = impl;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("authFetch", () => {
  it("passes through non-401 responses without refresh", async () => {
    const calls: string[] = [];
    setFetch(
      vi.fn(async (input: RequestInfo | URL) => {
        calls.push(input.toString());
        return new Response("ok", { status: 200 });
      }) as unknown as typeof fetch,
    );
    const res = await authFetch("/api/agents");
    expect(res.status).toBe(200);
    expect(calls).toEqual(["/api/agents"]);
  });

  it("on 401, calls /auth/refresh once and retries the original request", async () => {
    const calls: string[] = [];
    let originalAttempts = 0;
    setFetch(
      vi.fn(async (input: RequestInfo | URL) => {
        const url = input.toString();
        calls.push(url);
        if (url === "/auth/refresh") {
          return new Response(JSON.stringify({ email: "x" }), { status: 200 });
        }
        originalAttempts++;
        return new Response(null, { status: originalAttempts === 1 ? 401 : 200 });
      }) as unknown as typeof fetch,
    );
    const res = await authFetch("/api/agents");
    expect(res.status).toBe(200);
    expect(calls).toEqual(["/api/agents", "/auth/refresh", "/api/agents"]);
  });

  it("on refresh failure, returns the original 401 (no second retry)", async () => {
    const calls: string[] = [];
    setFetch(
      vi.fn(async (input: RequestInfo | URL) => {
        const url = input.toString();
        calls.push(url);
        if (url === "/auth/refresh") return new Response(null, { status: 401 });
        return new Response(null, { status: 401 });
      }) as unknown as typeof fetch,
    );
    const res = await authFetch("/api/agents");
    expect(res.status).toBe(401);
    expect(calls).toEqual(["/api/agents", "/auth/refresh"]);
  });

  it("does not attempt to refresh when the original call is itself /auth/*", async () => {
    const calls: string[] = [];
    setFetch(
      vi.fn(async (input: RequestInfo | URL) => {
        calls.push(input.toString());
        return new Response(null, { status: 401 });
      }) as unknown as typeof fetch,
    );
    const res = await authFetch("/auth/me");
    expect(res.status).toBe(401);
    expect(calls).toEqual(["/auth/me"]);
  });
});
