/**
 * Fetch wrapper that performs at most one silent `/auth/refresh` on a 401
 * response, then retries the original request. Refresh attempts are
 * de-duplicated across concurrent calls via a shared single-flight promise.
 *
 * Auth endpoints themselves bypass the refresh dance to avoid infinite loops.
 */

let refreshInFlight: Promise<boolean> | null = null;

async function tryRefresh(): Promise<boolean> {
  if (refreshInFlight) return refreshInFlight;
  refreshInFlight = (async () => {
    try {
      const res = await fetch("/auth/refresh", {
        method: "POST",
        credentials: "same-origin",
      });
      return res.ok;
    } catch {
      return false;
    } finally {
      // Release the single-flight slot a tick later so the original caller
      // can resolve before any retry kicks off another refresh.
      setTimeout(() => {
        refreshInFlight = null;
      }, 0);
    }
  })();
  return refreshInFlight;
}

function isAuthPath(input: RequestInfo | URL): boolean {
  const url =
    typeof input === "string"
      ? input
      : input instanceof URL
        ? input.pathname
        : input.url;
  try {
    const path = url.startsWith("http")
      ? new URL(url).pathname
      : url.split("?")[0];
    return path.startsWith("/auth/");
  } catch {
    return false;
  }
}

export async function authFetch(
  input: RequestInfo | URL,
  init?: RequestInit,
): Promise<Response> {
  const first = await fetch(input, { credentials: "same-origin", ...init });
  if (first.status !== 401 || isAuthPath(input)) return first;
  const refreshed = await tryRefresh();
  if (!refreshed) return first;
  return fetch(input, { credentials: "same-origin", ...init });
}
