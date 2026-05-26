import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import i18n from "@/i18n";
import { useLocaleSetter } from "./useLocale";
import { Wrap, makeQueryClient, mockFetch, jsonResponse } from "@/test-utils";

let putBody: unknown;

beforeEach(() => {
  putBody = undefined;
  mockFetch((url, init) => {
    if (url === "/api/settings" && init?.method === "PUT") {
      putBody = JSON.parse(init.body as string);
      return jsonResponse({ appearance: { locale: "pt-BR", theme: "system" } });
    }
    return jsonResponse({}, 404);
  });
});

afterEach(async () => {
  await i18n.changeLanguage("en");
});

describe("useLocaleSetter", () => {
  it("persists the locale via the settings mutation and switches i18n in place", async () => {
    const client = makeQueryClient();
    const { result } = renderHook(() => useLocaleSetter(), {
      wrapper: ({ children }) => <Wrap client={client}>{children}</Wrap>,
    });

    await act(async () => {
      await result.current.setLocale("pt-BR");
    });

    expect(putBody).toEqual({ appearance: { locale: "pt-BR" } });
    expect(i18n.language).toBe("pt-BR");
    expect(document.documentElement.lang).toBe("pt-BR");
  });
});
