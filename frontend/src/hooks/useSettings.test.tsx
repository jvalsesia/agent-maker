import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { useSettings } from "./useSettings";
import { api } from "@/lib/api";
import { Wrap, makeQueryClient, mockFetch, jsonResponse } from "@/test-utils";

const baseSettings = {
  default_provider: "anthropic",
  default_model: { anthropic: null, openai: null, openai_compat: null },
  memory_defaults: { recent_n: 10, top_k: 5 },
  appearance: { theme: "system" },
  providers: [
    { name: "anthropic", key_configured: false, key_masked: null, base_url: null },
    { name: "openai", key_configured: false, key_masked: null, base_url: null },
    { name: "openai_compat", key_configured: false, key_masked: null, base_url: null },
  ],
  key_store_backend: "keyring",
};

describe("useSettings", () => {
  let getCount: number;
  beforeEach(() => {
    getCount = 0;
    mockFetch((url, init) => {
      if (url === "/api/settings" && (!init || init.method === undefined || init.method === "GET")) {
        getCount += 1;
        return jsonResponse({
          ...baseSettings,
          appearance: { theme: getCount > 1 ? "dark" : "system" },
        });
      }
      if (url === "/api/settings" && init?.method === "PUT") {
        return jsonResponse({ ...baseSettings, appearance: { theme: "dark" } });
      }
      return jsonResponse({}, 404);
    });
  });

  it("refetches /api/settings after a successful PUT invalidation", async () => {
    const client = makeQueryClient();
    const { result } = renderHook(() => useSettings(), {
      wrapper: ({ children }) => <Wrap client={client}>{children}</Wrap>,
    });
    await waitFor(() => expect(result.current.data?.appearance.theme).toBe("system"));
    expect(getCount).toBe(1);

    await act(async () => {
      await api.putSettings({ appearance: { theme: "dark" } });
      await client.invalidateQueries({ queryKey: ["settings"] });
    });

    await waitFor(() => expect(result.current.data?.appearance.theme).toBe("dark"));
    expect(getCount).toBeGreaterThanOrEqual(2);
  });
});
