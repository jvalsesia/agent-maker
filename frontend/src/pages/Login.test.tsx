import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Route, Routes } from "react-router-dom";
import { LoginPage } from "@/pages/Login";
import { Wrap, jsonResponse, makeQueryClient, mockFetch } from "@/test-utils";

function renderLogin(initialPath = "/login") {
  return render(
    <Wrap client={makeQueryClient()} initialEntries={[initialPath]}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/" element={<div>home</div>} />
      </Routes>
    </Wrap>,
  );
}

describe("LoginPage", () => {
  it("submits credentials and navigates home on success", async () => {
    const seen: { url: string; body?: string }[] = [];
    mockFetch(async (url, init) => {
      seen.push({ url, body: init?.body as string | undefined });
      if (url === "/auth/login") {
        return jsonResponse({ email: "a@b.c", roles: [] });
      }
      return new Response(null, { status: 401 });
    });
    renderLogin();
    fireEvent.change(screen.getByLabelText(/email/i), {
      target: { value: "a@b.c" },
    });
    fireEvent.change(screen.getByLabelText(/password|senha/i), {
      target: { value: "secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: /sign in|entrar/i }));
    await waitFor(() => expect(screen.getByText("home")).toBeTruthy());
    const login = seen.find((s) => s.url === "/auth/login");
    expect(login).toBeTruthy();
    expect(login!.body).toContain("a@b.c");
    expect(login!.body).toContain("secret");
  });

  it("shows invalid-credentials error and clears password on 401", async () => {
    mockFetch(async (url) => {
      if (url === "/auth/login")
        return new Response(JSON.stringify({ error: "invalid_credentials" }), {
          status: 401,
        });
      return new Response(null, { status: 401 });
    });
    renderLogin();
    fireEvent.change(screen.getByLabelText(/email/i), {
      target: { value: "a@b.c" },
    });
    const password = screen.getByLabelText(/password|senha/i) as HTMLInputElement;
    fireEvent.change(password, { target: { value: "bad" } });
    fireEvent.click(screen.getByRole("button", { name: /sign in|entrar/i }));
    await waitFor(() =>
      expect(
        screen.getByText(/incorrect|incorretos/i),
      ).toBeTruthy(),
    );
    expect(password.value).toBe("");
  });

  it("renders the session-expired notice when ?expired=1", () => {
    mockFetch(async () => new Response(null, { status: 401 }));
    renderLogin("/login?expired=1");
    expect(
      screen.getByText(/session expired|sessão expirou/i),
    ).toBeTruthy();
  });
});
