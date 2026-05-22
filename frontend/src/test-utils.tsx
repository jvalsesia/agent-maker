import { ReactNode } from "react";
import { vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";

export function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0, staleTime: 0 } },
  });
}

export function Wrap({
  client,
  initialEntries = ["/"],
  children,
}: {
  client: QueryClient;
  initialEntries?: string[];
  children: ReactNode;
}) {
  return (
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={initialEntries}>{children}</MemoryRouter>
    </QueryClientProvider>
  );
}

type FetchMock = ReturnType<typeof vi.fn>;

export function mockFetch(handler: (url: string, init?: RequestInit) => Response | Promise<Response>): FetchMock {
  const fn = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === "string" ? input : input.toString();
    return await handler(url, init);
  }) as unknown as FetchMock;
  (globalThis as { fetch: unknown }).fetch = fn;
  return fn;
}

export function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

export function noContent(): Response {
  return new Response(null, { status: 204 });
}
