import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  ApiError,
  api,
  setAuthTokenGetter,
  setUnauthorizedHandler,
} from "./api";

function okJson(body: unknown) {
  return Promise.resolve({
    ok: true,
    status: 200,
    json: () => Promise.resolve(body),
  } as Response);
}

describe("api request() auth wiring (F10)", () => {
  beforeEach(() => {
    setAuthTokenGetter(null);
    setUnauthorizedHandler(null);
    vi.restoreAllMocks();
  });
  afterEach(() => {
    setAuthTokenGetter(null);
    setUnauthorizedHandler(null);
  });

  it("request_injects_bearer_when_token_present", async () => {
    const fetchSpy = vi
      .spyOn(globalThis, "fetch")
      .mockReturnValue(okJson({ agents: [] }));
    setAuthTokenGetter(() => Promise.resolve("tok-123"));

    await api.listAgents();

    const [, init] = fetchSpy.mock.calls[0];
    const headers = init?.headers as Record<string, string>;
    expect(headers.authorization).toBe("Bearer tok-123");
  });

  it("request_omits_bearer_when_disabled", async () => {
    const fetchSpy = vi
      .spyOn(globalThis, "fetch")
      .mockReturnValue(okJson({ agents: [] }));
    // No token getter registered.

    await api.listAgents();

    const [, init] = fetchSpy.mock.calls[0];
    const headers = (init?.headers as Record<string, string>) ?? {};
    expect(headers.authorization).toBeUndefined();
  });

  it("request_invokes_unauthorized_handler_on_401", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue({
      ok: false,
      status: 401,
      json: () => Promise.resolve({ error: { code: "unauthorized", message: "nope" } }),
    } as Response);
    const handler = vi.fn();
    setUnauthorizedHandler(handler);

    await expect(api.listAgents()).rejects.toBeInstanceOf(ApiError);
    expect(handler).toHaveBeenCalledOnce();
  });
});
