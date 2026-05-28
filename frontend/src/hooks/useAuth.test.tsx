import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { useMe, useLogin, useLogout } from "@/hooks/useAuth";
import { Wrap, makeQueryClient, mockFetch, jsonResponse, noContent } from "@/test-utils";

function MeProbe() {
  const { data, isLoading } = useMe();
  if (isLoading) return <div>loading</div>;
  if (!data) return <div>anonymous</div>;
  return <div>email:{data.email}</div>;
}

function LoginProbe({ email, password }: { email: string; password: string }) {
  const m = useLogin();
  return (
    <div>
      <button onClick={() => m.mutate({ email, password })}>login</button>
      <div data-testid="status">{m.status}</div>
    </div>
  );
}

function LogoutProbe() {
  const m = useLogout();
  return (
    <div>
      <button onClick={() => m.mutate()}>logout</button>
      <div data-testid="status">{m.status}</div>
    </div>
  );
}

describe("useAuth", () => {
  it("useMe returns null on 401", async () => {
    mockFetch(async () => new Response(null, { status: 401 }));
    render(
      <Wrap client={makeQueryClient()}>
        <MeProbe />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("anonymous")).toBeTruthy());
  });

  it("useMe returns the user payload on 200", async () => {
    mockFetch(async () =>
      jsonResponse({ email: "admin@example.com", roles: ["admin"] }),
    );
    render(
      <Wrap client={makeQueryClient()}>
        <MeProbe />
      </Wrap>,
    );
    await waitFor(() =>
      expect(screen.getByText("email:admin@example.com")).toBeTruthy(),
    );
  });

  it("useLogin populates the me cache on success", async () => {
    mockFetch(async (url) => {
      if (url === "/auth/login") {
        return jsonResponse({ email: "x@y.z", roles: [] });
      }
      // Initial /auth/me before login returns 401 (anonymous).
      return new Response(null, { status: 401 });
    });
    const qc = makeQueryClient();
    render(
      <Wrap client={qc}>
        <LoginProbe email="x@y.z" password="p" />
        <MeProbe />
      </Wrap>,
    );
    await waitFor(() => expect(screen.getByText("anonymous")).toBeTruthy());
    screen.getByText("login").click();
    await waitFor(() =>
      expect(screen.getByTestId("status").textContent).toBe("success"),
    );
    await waitFor(() =>
      expect(screen.getByText("email:x@y.z")).toBeTruthy(),
    );
    // useLogin should have written into the cache directly (no extra refetch).
    expect(qc.getQueryData(["me"])).toEqual({ email: "x@y.z", roles: [] });
  });

  it("useLogout clears the me cache so subsequent useMe returns null", async () => {
    const qc = makeQueryClient();
    qc.setQueryData(["me"], { email: "x@y.z", roles: [] });
    let logoutHit = false;
    mockFetch(async (url) => {
      if (url === "/auth/logout") {
        logoutHit = true;
        return noContent();
      }
      // After clear, useMe re-fetches and gets 401.
      return new Response(null, { status: 401 });
    });
    render(
      <Wrap client={qc}>
        <LogoutProbe />
        <MeProbe />
      </Wrap>,
    );
    screen.getByText("logout").click();
    await waitFor(() =>
      expect(screen.getByTestId("status").textContent).toBe("success"),
    );
    expect(logoutHit).toBe(true);
    await waitFor(() => expect(screen.getByText("anonymous")).toBeTruthy());
  });
});
